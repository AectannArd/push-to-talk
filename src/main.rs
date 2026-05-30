mod config;
mod hotkey;
mod indicator;
mod recorder;
mod transcriber;

use anyhow::Result;
use recorder::Recorder;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tracing::{error, info, warn};
use transcriber::Transcriber;

// ---- CLI -----------------------------------------------------------------

struct Cli {
    config_path: PathBuf,
    non_interactive: bool,
    debug_voice_record: bool,
}

fn parse_args() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path: Option<PathBuf> = None;
    let mut non_interactive = false;
    let mut debug_voice_record = false;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(PathBuf::from(&args[i]));
                } else {
                    eprintln!("⚠  --config requires a path argument");
                }
            }
            "--non-interactive" => non_interactive = true,
            "--debug-voice-record" => debug_voice_record = true,
            other if other.starts_with("--config=") => {
                config_path = Some(PathBuf::from(&other[9..]));
            }
            _ => {}
        }
        i += 1;
    }

    Cli {
        config_path: config_path.unwrap_or_else(config::default_path),
        non_interactive,
        debug_voice_record,
    }
}

// ---- main ----------------------------------------------------------------

fn main() -> Result<()> {
    let cli = parse_args();

    // ---- load config -------------------------------------------------------
    let cfg_path = cli.config_path.clone();
    let mut cfg = config::Config::load(&cfg_path);

    // ---- validate model -----------------------------------------------------
    let model_missing = cfg.model.as_ref().map_or(true, |m| !Path::new(m).exists());
    if model_missing && cli.non_interactive {
        anyhow::bail!(
            "Model not configured or file not found.\n\
             Run without --non-interactive to set up the model."
        );
    }

    // ---- interactive config review (before logger — uses eprintln!) ---------
    if !cli.non_interactive {
        review_config(&mut cfg, model_missing)?;
        cfg.save(&cfg_path);
    }

    // ---- expand env vars in log path, create dir -----------------------------
    let log_dir = expand_env_vars(&cfg.log_dir);
    std::fs::create_dir_all(&log_dir)?;

    // ---- init tracing subscriber (uses final config values) ------------------
    let log_ext: &'static str = if cfg.log_format == "json" { "json" } else { "log" };
    let rolling = RollingFileWriter::new(&log_dir, "push-to-talk", log_ext);
    let (non_blocking, _flush_guard) = tracing_appender::non_blocking(rolling);

    let filter = tracing_subscriber::EnvFilter::try_new(&cfg.log_level)
        .unwrap_or_else(|_| {
            eprintln!(
                "⚠  Invalid log_level '{}', falling back to 'error'",
                cfg.log_level
            );
            tracing_subscriber::EnvFilter::new("error")
        });

    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    if cfg.log_format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(std::io::stderr)
                    .with_target(false),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_target(false),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(false),
            )
            .init();
    }

    // Bridge log -> tracing so whisper-rs log_backend output is captured
    tracing_log::LogTracer::init().ok();

    // Cleanup old log files
    cleanup_old_logs(&log_dir, cfg.log_retention_hours);

    // Spawn periodic cleanup
    let cleanup_dir = log_dir.clone();
    let cleanup_hours = cfg.log_retention_hours;
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(600));
        cleanup_old_logs(&cleanup_dir, cleanup_hours);
    });

    // ---- banner (after logger so it appears in both console and file) --------
    info!("╔══════════════════════════════════════════╗");
    info!("║   🎙  Push-to-Talk CLI                   ║");
    info!("║   Hold hotkey, speak, release.          ║");
    info!("║   Text → auto-type → verify → Enter     ║");
    info!("╚══════════════════════════════════════════╝");
    info!("📁 Config: {}", cfg_path.display());
    info!("📁 Logs:   {}", log_dir);
    info!(
        "📊 Log level: {}, format: {}, retention: {}h",
        cfg.log_level,
        cfg.log_format,
        cfg.log_retention_hours,
    );

    // ---- parse hotkey from config -------------------------------------------
    let parsed_hotkey = hotkey::parse_hotkey(&cfg.hotkey).map_err(|e| {
        anyhow::anyhow!("Invalid hotkey '{}': {e}", cfg.hotkey)
    })?;
    info!(
        "⌨  Hotkey: {}{}{}{}{}",
        if parsed_hotkey.needs_ctrl { "Ctrl+" } else { "" },
        if parsed_hotkey.needs_shift { "Shift+" } else { "" },
        if parsed_hotkey.needs_alt { "Alt+" } else { "" },
        if parsed_hotkey.needs_win { "Win+" } else { "" },
        cfg.hotkey.split('+').last().unwrap_or("?"),
    );

    // ---- on-screen recording indicator -------------------------------------
    let indicator = indicator::spawn();
    let ind_vis = Arc::new(AtomicBool::new(false));

    // ---- load Whisper model ------------------------------------------------
    let model_path = find_model(&cfg.model, &cfg.model_search_dirs)?;
    // If model was auto-detected (not from config), save it
    if cfg.model.is_none() {
        cfg.model = Some(model_path.to_string_lossy().to_string());
        cfg.save(&cfg_path);
    }
    let transcriber = Transcriber::new(&model_path, cfg.language.clone())?;

    // ---- init recorder ------------------------------------------------------
    let (recorder, device_filter) = Recorder::new(cfg.device.as_deref())?;
    if let Some(filter) = device_filter {
        cfg.device = Some(filter);
        cfg.save(&cfg_path);
    }

    // ---- shared state (callback ← → transcription thread) --------------------
    let ctrl_held = Arc::new(AtomicBool::new(false));
    let shift_held = Arc::new(AtomicBool::new(false));
    let alt_held = Arc::new(AtomicBool::new(false));
    let win_held = Arc::new(AtomicBool::new(false));
    let is_recording = Arc::new(AtomicBool::new(false));
    let recording: Arc<Mutex<Option<recorder::Recording>>> =
        Arc::new(Mutex::new(None));

    let (tx, rx) = mpsc::channel::<Vec<i16>>();

    // ---- voice-records directory (sibling to log dir) -----------------------
    let voice_records_dir = if cli.debug_voice_record {
        let vr_dir = Path::new(&log_dir)
            .parent()
            .unwrap_or(Path::new("."))
            .join("voice-records");
        std::fs::create_dir_all(&vr_dir)?;
        info!("🎙  Voice records: {}", vr_dir.display());
        Some(vr_dir)
    } else {
        None
    };

    // ---- transcription background thread ------------------------------------
    let tr = Arc::new(transcriber);
    let tr_clone = tr.clone();
    let save_wav = cli.debug_voice_record;
    let wav_dir = voice_records_dir.clone();

    std::thread::spawn(move || {
        for audio in rx {
            let peak = audio.iter().map(|&s| s.abs()).max().unwrap_or(0);
            let rms = (audio
                .iter()
                .map(|&s| s as f64 * s as f64)
                .sum::<f64>()
                / audio.len() as f64)
                .sqrt();
            info!(
                "🔊 Audio: {:.1}s, peak={:.1}%, RMS={:.1}%",
                audio.len() as f64 / 16_000.0,
                peak as f64 / i16::MAX as f64 * 100.0,
                rms / i16::MAX as f64 * 100.0,
            );

            if save_wav {
                if let Some(ref dir) = wav_dir {
                    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let path = dir.join(format!("push-to-talk_{ts}.wav"));
                    if let Ok(mut writer) = hound::WavWriter::create(
                        &path,
                        hound::WavSpec {
                            channels: 1,
                            sample_rate: 16_000,
                            bits_per_sample: 16,
                            sample_format: hound::SampleFormat::Int,
                        },
                    ) {
                        for &s in &audio {
                            let _ = writer.write_sample(s);
                        }
                        let _ = writer.finalize();
                        info!("💾 Saved voice record: {}", path.display());
                    }
                }
            }

            let transcription_span = tracing::info_span!("transcribe");
            let _guard = transcription_span.enter();
            match tr_clone.transcribe(&audio) {
                Ok(text) if text.is_empty() => {
                    warn!("⚠  No speech detected — try again.");
                }
                Ok(text) => {
                    info!("📝 \"{}\"", text);
                    std::thread::sleep(std::time::Duration::from_millis(80));

                    if let Ok(mut enigo) = enigo::Enigo::new(&enigo::Settings::default()) {
                        use enigo::Keyboard;
                        let _ = enigo.text(&text);
                        info!("⌨  Text typed into active window.");
                    } else {
                        error!("⚠  Failed to initialise keyboard input.");
                    }

                    if let Ok(mut clip) = arboard::Clipboard::new() {
                        let _ = clip.set_text(&text);
                    }
                }
                Err(e) => error!("❌ Transcription error: {e}"),
            }
        }
    });

    // ---- capture-state clones for the callback -------------------------------
    let cb_ctrl = ctrl_held.clone();
    let cb_shift = shift_held.clone();
    let cb_alt = alt_held.clone();
    let cb_win = win_held.clone();
    let cb_is_rec = is_recording.clone();
    let cb_rec = recording.clone();
    let rec = Arc::new(recorder);
    let trigger_key = parsed_hotkey.key;
    let need_ctrl = parsed_hotkey.needs_ctrl;
    let need_shift = parsed_hotkey.needs_shift;
    let need_alt = parsed_hotkey.needs_alt;
    let need_win = parsed_hotkey.needs_win;
    let cb_ind = ind_vis.clone();
    let ind = indicator;

    // ---- global-hotkey event loop -------------------------------------------
    if let Err(e) = rdev::listen(move |event| {
        match event.event_type {
            rdev::EventType::KeyPress(key) => {
                if update_modifier_state(&key, true, &cb_ctrl, &cb_shift, &cb_alt, &cb_win) {
                    return;
                }
                if key == trigger_key
                    && modifier_match(need_ctrl, cb_ctrl.load(Ordering::SeqCst))
                    && modifier_match(need_shift, cb_shift.load(Ordering::SeqCst))
                    && modifier_match(need_alt, cb_alt.load(Ordering::SeqCst))
                    && modifier_match(need_win, cb_win.load(Ordering::SeqCst))
                    && !cb_is_rec.load(Ordering::SeqCst)
                {
                    cb_is_rec.store(true, Ordering::SeqCst);
                    cb_ind.store(true, Ordering::SeqCst);
                    ind.set_visible(true);
                    match rec.start() {
                        Ok(r) => {
                            *cb_rec.lock().unwrap() = Some(r);
                            info!("🎤 Recording... (release to stop)");
                        }
                        Err(e) => {
                            error!("❌ Failed to start recording: {e}");
                            cb_is_rec.store(false, Ordering::SeqCst);
                            cb_ind.store(false, Ordering::SeqCst);
                            ind.set_visible(false);
                        }
                    }
                }
            }

            rdev::EventType::KeyRelease(key) => {
                update_modifier_state(&key, false, &cb_ctrl, &cb_shift, &cb_alt, &cb_win);
                if key == trigger_key && cb_is_rec.load(Ordering::SeqCst) {
                    cb_is_rec.store(false, Ordering::SeqCst);
                    cb_ind.store(false, Ordering::SeqCst);
                    ind.set_visible(false);
                    let audio = cb_rec
                        .lock()
                        .unwrap()
                        .take()
                        .map(|r| r.stop())
                        .unwrap_or_default();

                    if audio.is_empty() {
                        warn!("⚠  No audio captured — recording too short.");
                        return;
                    }

                    info!(
                        "🛑 Captured {:.1}s — transcribing…",
                        audio.len() as f64 / 16_000.0
                    );
                    let _ = tx.send(audio);
                }
            }

            _ => {}
        }
    }) {
        anyhow::bail!(
            "Failed to register global hotkey ({e:?}).\n\
             Is another push-to-talk instance running? On some systems, \
             running as Administrator may be required for global hotkeys."
        );
    }

    Ok(())
}

// ---- config review -------------------------------------------------------

/// Print current config and ask the user if they want to edit it.
fn review_config(cfg: &mut config::Config, force: bool) -> Result<()> {
    eprintln!();
    eprintln!("┌─ Current config ───────────────────────────────────");
    eprintln!(
        "│ device:             {}",
        cfg.device.as_deref().unwrap_or("<prompt on startup>")
    );
    eprintln!(
        "│ language:           {}",
        cfg.language.as_deref().unwrap_or("auto")
    );
    eprintln!(
        "│ model:              {}",
        cfg.model.as_deref().unwrap_or("<not set>")
    );
    eprintln!("│ hotkey:             {}", cfg.hotkey);
    eprintln!(
        "│ model_search_dirs:  {:?}",
        cfg.model_search_dirs,
    );
    eprintln!("│ log_dir:            {}", cfg.log_dir);
    eprintln!("│ log_level:          {}", cfg.log_level);
    eprintln!("│ log_format:         {}", cfg.log_format);
    eprintln!("│ log_retention:      {}h", cfg.log_retention_hours);
    eprintln!("└─────────────────────────────────────────────────────");

    if !force {
        eprint!("\n✏  Edit config? [y/N]: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            return Ok(());
        }
    } else {
        eprintln!("\n⚠  Model not configured or file missing — setup required.");
    }

    let mut input = String::new();

    // -- Step 1: Model search dirs FIRST (before model selection) --
    eprintln!();
    eprintln!("─── Model search directories ─────────────────────────");
    eprintln!("These directories are scanned for ggml-*.bin files.");
    let cur_dirs = cfg.model_search_dirs.join(", ");
    eprint!("Directories (comma-separated) [{cur_dirs}]: ");
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.model_search_dirs = val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // -- Step 2: Scan with the (possibly updated) dirs, pick model --
    eprintln!();
    eprintln!("─── Model selection ──────────────────────────────────");
    let models = scan_all_models(&cfg.model_search_dirs);
    if models.is_empty() {
        anyhow::bail!(
            "No ggml-*.bin models found in: {dirs:?}\n\
             Download a model from https://huggingface.co/ggerganov/whisper.cpp\n\
             Recommended: ggml-base.bin (~142 MB) or ggml-tiny.bin (~78 MB).",
            dirs = cfg.model_search_dirs
        );
    } else {
        eprintln!("┌─ Available models ───────────────────────────────────");
        for (i, m) in models.iter().enumerate() {
            let mark = if Some(&m.to_string_lossy().to_string()) == cfg.model.as_ref() {
                " ← current"
            } else {
                ""
            };
            eprintln!("│ [{i}] {}{mark}", m.display());
        }
        eprintln!("└──────────────────────────────────────────────────────");

        let cur_model = cfg.model.as_deref().unwrap_or("<not set>");
        eprintln!("Current: {cur_model}");
        eprint!("Model index (or Enter to keep): ");
        input.clear();
        std::io::stdin().read_line(&mut input).ok();
        let val = input.trim().to_string();
        if !val.is_empty() {
            if let Ok(idx) = val.parse::<usize>() {
                if let Some(m) = models.get(idx) {
                    cfg.model = Some(m.to_string_lossy().to_string());
                } else {
                    eprintln!("⚠  Invalid index");
                }
            } else {
                eprintln!("⚠  Enter a numeric index");
            }
        }
    }

    if force && cfg.model.is_none() {
        eprintln!("⚠  No model selected — will retry on next start.");
        return Ok(());
    }

    // -- Remaining settings (skip in force mode unless user wants to) --
    if force {
        eprintln!();
        eprintln!("─── Remaining settings ───────────────────────────────");
        eprint!("Review other settings? [y/N]: ");
        input.clear();
        std::io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            return Ok(());
        }
    }

    edit_remaining_settings(cfg);
    Ok(())
}

/// Interactive editing of all non-model settings.
fn edit_remaining_settings(cfg: &mut config::Config) {
    let mut input = String::new();

    // -- Edit device --
    eprintln!();
    match recorder::list_input_devices() {
        Ok(devices) => {
            eprintln!("┌─ Available input devices ────────────────────────────");
            for d in &devices {
                let marker = if d.is_default { " (default)" } else { "" };
                eprintln!(
                    "│ [{i}] {name} — {cfg}{marker}",
                    i = d.index,
                    name = d.name,
                    cfg = d.config,
                );
            }
            eprintln!("└──────────────────────────────────────────────────────");
        }
        Err(e) => eprintln!("⚠  Could not list devices: {e}"),
    }
    let cur_dev = cfg.device.as_deref().unwrap_or_default();
    eprint!("Device [{cur_dev}]: ");
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.device = Some(val);
    }

    // -- Edit language --
    eprintln!();
    eprintln!("Language (ISO 639-1 code, or 'auto'):");
    let cur_lang = cfg.language.as_deref().unwrap_or("auto");
    eprint!("Language [{cur_lang}]: ");
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let lang_val = input.trim().to_string();
    if !lang_val.is_empty() {
        cfg.language = Some(lang_val);
    }

    // -- Edit hotkey --
    eprintln!();
    eprintln!("Hotkey (format: Mod+Mod+Key, e.g. Ctrl+Shift+T):");
    eprintln!("Modifiers: Ctrl, Shift, Alt, Win");
    eprint!("Hotkey [{}]: ", cfg.hotkey);
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        match hotkey::parse_hotkey(&val) {
            Ok(_) => cfg.hotkey = val,
            Err(e) => eprintln!("⚠  Invalid hotkey: {e} — keeping current"),
        }
    }

    // -- Edit log dir --
    eprintln!();
    eprintln!("Log directory:");
    eprint!("Log dir [{}]: ", cfg.log_dir);
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.log_dir = val;
    }

    // -- Edit log level --
    eprintln!();
    eprintln!("Log level (trace, debug, info, warn, error):");
    eprint!("Log level [{}]: ", cfg.log_level);
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        let lv = val.to_lowercase();
        if matches!(lv.as_str(), "trace" | "debug" | "info" | "warn" | "error") {
            cfg.log_level = lv;
        } else {
            eprintln!("⚠  Invalid level — keeping current");
        }
    }

    // -- Edit log format --
    eprintln!();
    eprintln!("Log format (text or json):");
    eprint!("Format [{}]: ", cfg.log_format);
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_lowercase();
    if !val.is_empty() {
        if matches!(val.as_str(), "text" | "json") {
            cfg.log_format = val;
        } else {
            eprintln!("⚠  Invalid format — use 'text' or 'json'");
        }
    }

    // -- Edit log retention --
    eprintln!();
    eprintln!("Log retention in hours (rotated files older than this are deleted):");
    eprint!("Retention hours [{}]: ", cfg.log_retention_hours);
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        if let Ok(n) = val.parse::<u64>() {
            if n > 0 {
                cfg.log_retention_hours = n;
            } else {
                eprintln!("⚠  Must be at least 1");
            }
        } else {
            eprintln!("⚠  Not a number");
        }
    }
}

// ---- helpers -------------------------------------------------------------

/// Returns `true` if `key` is a modifier and the state was updated.
fn update_modifier_state(
    key: &rdev::Key,
    pressed: bool,
    ctrl: &AtomicBool,
    shift: &AtomicBool,
    alt: &AtomicBool,
    win: &AtomicBool,
) -> bool {
    match key {
        rdev::Key::ControlLeft | rdev::Key::ControlRight => {
            ctrl.store(pressed, Ordering::SeqCst);
            true
        }
        rdev::Key::ShiftLeft | rdev::Key::ShiftRight => {
            shift.store(pressed, Ordering::SeqCst);
            true
        }
        rdev::Key::Alt | rdev::Key::AltGr => {
            alt.store(pressed, Ordering::SeqCst);
            true
        }
        rdev::Key::MetaLeft | rdev::Key::MetaRight => {
            win.store(pressed, Ordering::SeqCst);
            true
        }
        _ => false,
    }
}

/// Check if a required modifier matches its actual state.
/// If the modifier is not needed, we don't care about its state.
fn modifier_match(needed: bool, actual: bool) -> bool {
    if needed {
        actual
    } else {
        true // don't care — but in practice, if user presses extra mods that's fine
    }
}

/// Discover the Whisper model file.
///
/// 1. `WHISPER_MODEL` env var (exact path)
/// 2. `explicit_path` from config (exact path)
/// 3. Scan `search_dirs` for `ggml-*.bin`, pick the first one
fn find_model(explicit_path: &Option<String>, search_dirs: &[String]) -> Result<PathBuf> {
    // 1. Env var override
    if let Ok(p) = std::env::var("WHISPER_MODEL") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Ok(path);
        }
        warn!("WHISPER_MODEL={p} does not exist");
    }

    // 2. Explicit model from config
    if let Some(p) = explicit_path {
        let path = PathBuf::from(p);
        if path.exists() {
            info!("🔍 Using model from config: {}", path.display());
            return Ok(path);
        }
        warn!("Configured model not found at {p}, scanning…");
    }

    // 3. Scan directories
    for dir in search_dirs {
        if let Some(path) = scan_for_model(Path::new(dir)) {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No Whisper model (ggml-*.bin) found in: {dirs:?}\n\
         Download: curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        dirs = search_dirs,
    );
}

/// Scan directories for all ggml-*.bin files, returning full paths.
pub fn scan_all_models(dirs: &[String]) -> Vec<PathBuf> {
    let mut models = Vec::new();
    for dir in dirs {
        let root = Path::new(dir);
        if !root.exists() {
            continue;
        }
        collect_models(root, 0, &mut models);
    }
    models
}

fn collect_models(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_models(&path, depth + 1, out);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ggml-") && n.ends_with(".bin"))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
}

/// Recursively scan `root` (up to 3 levels deep) for `ggml-*.bin` files.
fn scan_for_model(root: &Path) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    fn walk(dir: &Path, depth: u32) -> Option<PathBuf> {
        if depth > 3 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path, depth + 1) {
                    return Some(found);
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ggml-") && n.ends_with(".bin"))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
        None
    }

    let found = walk(root, 0);
    if let Some(ref p) = found {
        info!("🔍 Found model: {}", p.display());
    }
    found
}

/// Delete rotated log files older than `max_age_hours`.
fn cleanup_old_logs(dir: &str, max_age_hours: u64) {
    let dir = Path::new(dir);
    if !dir.exists() {
        return;
    }
    let Some(cutoff) = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_hours * 3600))
    else {
        return;
    };
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path
                .extension()
                .map_or(true, |e| e != "log" && e != "json")
            {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

// ---- custom rolling file writer ------------------------------------------

use std::io::{self, Write};

struct RollingFileWriter {
    dir: PathBuf,
    prefix: String,
    ext: String,
    current_key: String,
    file: Option<std::fs::File>,
}

impl RollingFileWriter {
    fn new(dir: &str, prefix: &str, ext: &str) -> Self {
        std::fs::create_dir_all(dir).ok();
        Self {
            dir: PathBuf::from(dir),
            prefix: prefix.to_string(),
            ext: ext.to_string(),
            current_key: String::new(),
            file: None,
        }
    }

    /// Returns the rotation key for the current minute.
    fn rotation_key() -> String {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d-%H-%M").to_string()
    }

    fn rotate(&mut self) -> io::Result<()> {
        let key = Self::rotation_key();
        if key == self.current_key {
            return Ok(());
        }
        self.current_key = key;
        let filename = format!("{}.{}.{}", self.prefix, self.current_key, self.ext);
        let path = self.dir.join(filename);
        self.file = Some(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?,
        );
        Ok(())
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.rotate()?;
        if let Some(f) = self.file.as_mut() {
            f.write(buf)
        } else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.flush()
        } else {
            Ok(())
        }
    }
}

/// Expand environment variables in a path string.
/// Supports `%VAR%` (Windows) and `$VAR` / `${VAR}` (Unix).
fn expand_env_vars(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            // Windows-style %VAR%
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let var: String = chars[i + 1..i + 1 + end].iter().collect();
                let value = std::env::var(&var).unwrap_or_else(|_| {
                    format!("%{var}%")
                });
                result.push_str(&value);
                i += end + 2;
                continue;
            }
        }
        if chars[i] == '$' && i + 1 < chars.len() {
            // Unix-style $VAR or ${VAR}
            let start = i + 1;
            if chars[start] == '{' {
                if let Some(end) = chars[start + 1..].iter().position(|&c| c == '}') {
                    let var: String = chars[start + 1..start + 1 + end].iter().collect();
                    let value = std::env::var(&var).unwrap_or_else(|_| {
                        format!("${{{var}}}")
                    });
                    result.push_str(&value);
                    i += end + 3;
                    continue;
                }
            } else {
                let end = chars[start..]
                    .iter()
                    .position(|&c| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(chars.len() - start);
                let var: String = chars[start..start + end].iter().collect();
                if !var.is_empty() {
                    let value =
                        std::env::var(&var).unwrap_or_else(|_| format!("${var}"));
                    result.push_str(&value);
                    i += end + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}
