mod config;
mod hotkey;
mod indicator;
mod recorder;
mod transcriber;
mod tray;
mod ui;

use anyhow::Result;
use clap::Parser;
use recorder::Recorder;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use tracing::{error, info, warn};
use transcriber::Transcriber;

// ---- CLI -----------------------------------------------------------------

/// Push-to-talk voice input for CLI using Whisper.
///
/// Hold the configured hotkey, speak, and release to transcribe audio to text.
/// The transcribed text is automatically typed into the active window.
#[derive(Parser, Debug)]
#[command(name = "push-to-talk")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the config file
    #[arg(long, value_name = "PATH", default_value_os_t = config::default_path())]
    config: PathBuf,

    /// Run in non-interactive mode (no prompts, fail on missing config)
    #[arg(long)]
    non_interactive: bool,

    /// Save voice recordings to the voice-records directory for debugging
    #[arg(long)]
    debug_voice_record: bool,
}

// ---- main ----------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    // ---- load config -------------------------------------------------------
    let cfg_path = cli.config.clone();
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
        ui::review_config(&mut cfg, model_missing)?;
        cfg.save(&cfg_path);
    }

    // ---- expand env vars in log path, create dir -----------------------------
    let log_dir = expand_env_vars(&cfg.log_dir);
    std::fs::create_dir_all(&log_dir)?;

    // ---- init tracing subscriber (uses final config values) ------------------
    let log_ext: &'static str = if cfg.log_format == "json" {
        "json"
    } else {
        "log"
    };
    let rolling = RollingFileWriter::new(&log_dir, "push-to-talk", log_ext);
    let (non_blocking, _flush_guard) = tracing_appender::non_blocking(rolling);

    let filter = tracing_subscriber::EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| {
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
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(600));
            cleanup_old_logs(&cleanup_dir, cleanup_hours);
        }
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
        cfg.log_level, cfg.log_format, cfg.log_retention_hours,
    );

    // ---- parse hotkey from config -------------------------------------------
    let parsed_hotkey = hotkey::parse_hotkey(&cfg.hotkey)
        .map_err(|e| anyhow::anyhow!("Invalid hotkey '{}': {e}", cfg.hotkey))?;
    let win_label = if parsed_hotkey.needs_win {
        mod_win_label()
    } else {
        ""
    };
    info!(
        "⌨  Hotkey: {}{}{}{}{}",
        if parsed_hotkey.needs_ctrl {
            "Ctrl+"
        } else {
            ""
        },
        if parsed_hotkey.needs_shift {
            "Shift+"
        } else {
            ""
        },
        if parsed_hotkey.needs_alt { "Alt+" } else { "" },
        win_label,
        cfg.hotkey.split('+').last().unwrap_or("?"),
    );

    // ---- recording indicators (console + system tray) -----------------------
    let indicator = indicator::spawn();
    let tray_icon = tray::spawn();

    // ---- load Whisper model ------------------------------------------------
    let model_path = ui::find_model(&cfg.model, &cfg.model_search_dirs)?;
    // If model was auto-detected (not from config), save it
    if cfg.model.is_none() {
        cfg.model = Some(model_path.to_string_lossy().to_string());
        cfg.save(&cfg_path);
    }
    let transcriber = Transcriber::new(&model_path, cfg.language.clone())?;

    // ---- init recorder ------------------------------------------------------
    let (recorder, device_info) = Recorder::new(cfg.device_id.as_deref())?;
    if let Some((id, name)) = device_info {
        cfg.device_id = Some(id);
        cfg.device_name = Some(name);
        cfg.save(&cfg_path);
    }

    // ---- shared state (callback ← → transcription thread) --------------------
    let ctrl_held = Arc::new(AtomicBool::new(false));
    let shift_held = Arc::new(AtomicBool::new(false));
    let alt_held = Arc::new(AtomicBool::new(false));
    let win_held = Arc::new(AtomicBool::new(false));
    let trigger_held = Arc::new(AtomicBool::new(false));  // track trigger key state
    let is_recording = Arc::new(AtomicBool::new(false));
    let recording: Arc<Mutex<Option<recorder::Recording>>> = Arc::new(Mutex::new(None));

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
    // Wrap in Mutex for thread-safety: whisper-rs with Metal/CoreML on macOS is not thread-safe
    let tr = Arc::new(Mutex::new(transcriber));
    let tr_clone = tr.clone();
    let save_wav = cli.debug_voice_record;
    let wav_dir = voice_records_dir.clone();

    std::thread::spawn(move || {
        for audio in rx {
            let peak = audio.iter().map(|&s| s.abs()).max().unwrap_or(0);
            let rms = (audio.iter().map(|&s| s as f64 * s as f64).sum::<f64>()
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
            match tr_clone.lock().unwrap().transcribe(&audio) {
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
    let cb_trigger = trigger_held.clone();
    let cb_is_rec = is_recording.clone();
    let cb_rec = recording.clone();
    let rec = Arc::new(recorder);
    let trigger_key = parsed_hotkey.key;
    let need_ctrl = parsed_hotkey.needs_ctrl;
    let need_shift = parsed_hotkey.needs_shift;
    let need_alt = parsed_hotkey.needs_alt;
    let need_win = parsed_hotkey.needs_win;
    let tray = tray_icon;
    let ind = indicator;

    // ---- global-hotkey event loop -------------------------------------------
    if let Err(e) = rdev::listen(move |event| {
        match event.event_type {
            rdev::EventType::KeyPress(key) => {
                if update_modifier_state(&key, true, &cb_ctrl, &cb_shift, &cb_alt, &cb_win) {
                    return;
                }
                // Only trigger on initial press, not on repeat
                if key == trigger_key
                    && !cb_trigger.load(Ordering::SeqCst)  // prevent repeat
                    && modifier_match(need_ctrl, cb_ctrl.load(Ordering::SeqCst))
                    && modifier_match(need_shift, cb_shift.load(Ordering::SeqCst))
                    && modifier_match(need_alt, cb_alt.load(Ordering::SeqCst))
                    && modifier_match(need_win, cb_win.load(Ordering::SeqCst))
                    && !cb_is_rec.load(Ordering::SeqCst)
                {
                    cb_trigger.store(true, Ordering::SeqCst);  // mark as held
                    cb_is_rec.store(true, Ordering::SeqCst);
                    tray.set_recording(true);
                    ind.set_visible(true);
                    match rec.start() {
                        Ok(r) => {
                            *cb_rec.lock().unwrap() = Some(r);
                            info!("🎤 Recording... (release to stop)");
                        }
                        Err(e) => {
                            error!("❌ Failed to start recording: {e}");
                            cb_is_rec.store(false, Ordering::SeqCst);
                            cb_trigger.store(false, Ordering::SeqCst);
                            tray.set_recording(false);
                            ind.set_visible(false);
                        }
                    }
                }
            }

            rdev::EventType::KeyRelease(key) => {
                update_modifier_state(&key, false, &cb_ctrl, &cb_shift, &cb_alt, &cb_win);
                if key == trigger_key && cb_trigger.load(Ordering::SeqCst) {
                    cb_trigger.store(false, Ordering::SeqCst);  // mark as released
                    // Log immediately to catch crash point
                    info!("🛑 Key released, stopping recording...");

                    cb_is_rec.store(false, Ordering::SeqCst);
                    tray.set_recording(false);
                    ind.set_visible(false);

                    // Safely stop recording and get audio buffer
                    let audio = {
                        let mut guard = match cb_rec.lock() {
                            Ok(g) => g,
                            Err(e) => {
                                error!("❌ Mutex poisoned on recording channel: {e}");
                                return;
                            }
                        };
                        match guard.take() {
                            Some(r) => {
                                info!("🛑 Stopping audio stream...");
                                r.stop()
                            }
                            None => {
                                warn!("⚠  Recording was None — already taken?");
                                Vec::new()
                            }
                        }
                    };

                    if audio.is_empty() {
                        warn!("⚠  No audio captured — recording too short.");
                        return;
                    }

                    info!(
                        "🛑 Captured {:.1}s — transcribing…",
                        audio.len() as f64 / 16_000.0
                    );

                    if let Err(e) = tx.send(audio) {
                        error!("❌ Failed to send audio to transcription thread: {e}");
                    }
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
            if path.extension().map_or(true, |e| e != "log" && e != "json") {
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
/// Platform-appropriate label for the Win/Cmd modifier key.
const fn mod_win_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+"
    } else {
        "Win+"
    }
}

fn expand_env_vars(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            // Windows-style %VAR%
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let var: String = chars[i + 1..i + 1 + end].iter().collect();
                let value = std::env::var(&var).unwrap_or_else(|_| format!("%{var}%"));
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
                    let value = std::env::var(&var).unwrap_or_else(|_| format!("${{{var}}}"));
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
                    let value = std::env::var(&var).unwrap_or_else(|_| format!("${var}"));
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
