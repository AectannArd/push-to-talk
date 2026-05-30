mod config;
mod hotkey;
mod indicator;
mod recorder;
mod transcriber;

use anyhow::Result;
use log::{error, info, warn};
use recorder::Recorder;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use transcriber::Transcriber;

// ---- CLI -----------------------------------------------------------------

struct Cli {
    config_path: PathBuf,
    non_interactive: bool,
}

fn parse_args() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path: Option<PathBuf> = None;
    let mut non_interactive = false;
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
    }
}

// ---- main ----------------------------------------------------------------

fn main() -> Result<()> {
    let cli = parse_args();

    // ---- init rolling file logger (rotates every 1 minute) -----------------
    let _logger_handle = flexi_logger::Logger::try_with_str("info")?
        .log_to_file(
            flexi_logger::FileSpec::default()
                .directory("logs")
                .basename("push-to-talk"),
        )
        .rotate(
            flexi_logger::Criterion::Age(flexi_logger::Age::Minute),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepLogFiles(120),
        )
        .duplicate_to_stderr(flexi_logger::Duplicate::All)
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stderr(flexi_logger::colored_default_format)
        .start()?;

    info!("╔══════════════════════════════════════════╗");
    info!("║   🎙  Push-to-Talk CLI                   ║");
    info!("║   Hold hotkey, speak, release.          ║");
    info!("║   Text → auto-type → verify → Enter     ║");
    info!("╚══════════════════════════════════════════╝");
    info!("📁 Config: {}", cli.config_path.display());
    info!(
        "📁 Logs:   {}",
        std::env::current_dir().unwrap_or_default().join("logs").display()
    );

    // ---- load config -------------------------------------------------------
    let mut cfg = config::Config::load(&cli.config_path);

    // ---- interactive config review (unless --non-interactive) ---------------
    if !cli.non_interactive {
        review_config(&mut cfg);
        cfg.save(&cli.config_path);
    }

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
    let model_path = find_model(&cfg.model_search_dirs)?;
    let transcriber = Transcriber::new(&model_path, cfg.language.clone())?;

    // ---- init recorder ------------------------------------------------------
    let (recorder, device_filter) = Recorder::new(cfg.device.as_deref())?;
    if let Some(filter) = device_filter {
        cfg.device = Some(filter);
        cfg.save(&cli.config_path);
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

    // ---- transcription background thread ------------------------------------
    let tr = Arc::new(transcriber);
    let tr_clone = tr.clone();

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

            let debug_path = std::env::temp_dir().join("push-to-talk-debug.wav");
            if let Ok(mut writer) = hound::WavWriter::create(
                &debug_path,
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
                info!("💾 Saved debug WAV: {}", debug_path.display());
            }

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
fn review_config(cfg: &mut config::Config) {
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
    eprintln!("│ hotkey:             {}", cfg.hotkey);
    eprintln!(
        "│ model_search_dirs:  {:?}",
        cfg.model_search_dirs,
    );
    eprintln!("└─────────────────────────────────────────────────────");

    eprint!("\n✏  Edit config? [y/N]: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
        return;
    }

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
    eprintln!(
        "Current device: {}",
        cfg.device.as_deref().unwrap_or("<prompt on startup>")
    );
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
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.language = Some(val);
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
        // Validate
        match hotkey::parse_hotkey(&val) {
            Ok(_) => cfg.hotkey = val,
            Err(e) => eprintln!("⚠  Invalid hotkey: {e} — keeping current"),
        }
    }

    // -- Edit model search dirs --
    eprintln!();
    eprintln!("Model search directories (comma-separated):");
    let cur_dirs = cfg.model_search_dirs.join(", ");
    eprint!("Dirs [{cur_dirs}]: ");
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

/// Discover the Whisper model file using configurable search directories.
fn find_model(dirs: &[String]) -> Result<PathBuf> {
    // WHISPER_MODEL env var still works as an override
    if let Ok(p) = std::env::var("WHISPER_MODEL") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Ok(path);
        }
        warn!("WHISPER_MODEL={p} does not exist, scanning…");
    }

    for dir in dirs {
        if let Some(path) = scan_for_model(Path::new(dir)) {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No Whisper model (ggml-*.bin) found in: {dirs:?}\n\
         Download: curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin\n\
         Or add the directory to model_search_dirs in the config.",
        dirs = dirs,
    );
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
