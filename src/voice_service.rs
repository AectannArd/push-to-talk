//! Voice Service - runs voice capture and transcription on background threads.
//!
//! ## Thread architecture
//!
//! ```text
//! Tauri UI thread
//!   │
//!   ├─→ cmd_tx.send(StartRecording)  ──→  Worker thread
//!   ├─→ cmd_tx.send(StopRecording)   ──→    ├── Recorder (Arc<Mutex<>>)
//!   └─→ cmd_tx.send(Shutdown)        ──→    ├── Recording state
//!                                             └── audio_tx.send(audio)
//!                                                       │
//!                                                       ▼
//!                                            Transcription thread
//!                                              ├── Transcriber
//!                                              ├── Punctuator (optional)
//!                                              ├── Clipboard
//!                                              └── Paste
//!
//! Monitor thread
//!   ├── Polls device list every 3s
//!   ├── On disconnect: cmd_tx.send(StopRecording) → replaces Recorder
//!   └── Exits on stop_flag
//! ```

use crate::config::Config;
use crate::recorder::Recording;
use crate::transcriber::Transcriber;

use std::path::Path;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

// ── Commands sent from UI/monitor threads to the worker ──────────────

enum Command {
    StartRecording,
    StopRecording,
    Shutdown,
}

// ── Public handle (returned to main.rs) ──────────────────────────────

pub struct VoiceServiceHandle {
    cmd_tx: mpsc::Sender<Command>,
    monitor_stop: Arc<AtomicBool>,
    worker_handle: Option<thread::JoinHandle<()>>,
    transcribe_handle: Option<thread::JoinHandle<()>>,
    monitor_handle: Option<thread::JoinHandle<()>>,
    pub state: Arc<VoiceServiceInner>,
}

/// Status fields shared between threads for polling (read-only from outside).
pub struct VoiceServiceInner {
    pub is_recording: Arc<AtomicBool>,
    pub should_paste: Arc<AtomicBool>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
    pub transcriber: Arc<Mutex<crate::transcriber::Transcriber>>,
}

// ── Public API ───────────────────────────────────────────────────────

impl VoiceServiceHandle {
    /// Spawn all background threads and return a handle.
    ///
    /// # Threads spawned
    /// - **Worker**: processes Start/Stop commands, owns the Recorder and
    ///   Recording lifecycle.
    /// - **Transcription**: receives raw audio, runs whisper + punctuation,
    ///   handles clipboard + paste.
    /// - **Monitor**: polls device list, auto-switches on disconnect.
    pub fn start(
        config: Config,
        last_transcription: Arc<Mutex<Option<String>>>,
        app_is_recording: Arc<AtomicBool>,
        app_config: Arc<Mutex<Config>>,
        punctuator: Arc<Mutex<Option<crate::punctuator::Punctuator>>>,
    ) -> Result<Self, anyhow::Error> {
        // ── Load model ───────────────────────────────────────────────
        let model_path = match find_model(&config) {
            Some(p) => p,
            None => return Err(anyhow::anyhow!("Model not found")),
        };

        let transcriber = Transcriber::new(&model_path, config.language.clone())
            .map_err(|e| anyhow::anyhow!("Failed to load transcriber: {}", e))?;
        let tr = Arc::new(Mutex::new(transcriber));

        // ── Initialize recorder ──────────────────────────────────────
        let (recorder, _device_info) = crate::recorder::Recorder::new(config.device_id.as_deref())
            .map_err(|e| anyhow::anyhow!("Failed to initialize recorder: {}", e))?;
        let rec = Arc::new(Mutex::new(recorder));

        // ── Channels ─────────────────────────────────────────────────
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        // Clone for the monitor so it can send StopRecording on device disconnect
        let cmd_tx_for_monitor = cmd_tx.clone();

        // ── Shared state (polling + language hot-swap) ───────────────
        let should_paste = Arc::new(AtomicBool::new(false));
        let state = Arc::new(VoiceServiceInner {
            is_recording: app_is_recording,
            should_paste: should_paste.clone(),
            last_transcription,
            transcriber: tr.clone(),
        });

        // ── Transcription thread ─────────────────────────────────────
        let tr_clone = tr.clone();
        let last_tr = state.last_transcription.clone();
        let should_paste_tx = should_paste.clone();
        let punc = punctuator.clone();

        let transcribe_handle = thread::spawn(move || {
            for audio in audio_rx {
                match tr_clone.lock().unwrap().transcribe(&audio) {
                    Ok(text) if text.is_empty() => {
                        warn!("⚠ No speech detected");
                    }
                    Ok(text) => {
                        info!("📝 Raw: \"{}\"", text);

                        let final_text = match punc.lock().unwrap().as_mut() {
                            Some(p) => match p.punctuate(&text) {
                                Ok(punctuated) if !punctuated.is_empty() => {
                                    info!("📝 Punctuated: \"{}\"", punctuated);
                                    punctuated
                                }
                                Ok(_) => {
                                    warn!("⚠ Punctuation returned empty — using raw text");
                                    text
                                }
                                Err(e) => {
                                    warn!("⚠ Punctuation failed: {} — using raw text", e);
                                    text
                                }
                            },
                            None => text,
                        };

                        *last_tr.lock().unwrap() = Some(final_text.clone());
                        if should_paste_tx.load(Ordering::Relaxed) {
                            copy_to_clipboard(&final_text);
                            info!("✅ Text copied to clipboard");
                            thread::sleep(Duration::from_millis(100));
                            paste_from_clipboard();
                            info!("✅ Paste completed");
                        }
                    }
                    Err(e) => error!("❌ Transcription error: {}", e),
                }
            }
            info!("👋 Transcription thread exiting (channel closed)");
        });

        // ── Worker thread (recording lifecycle) ──────────────────────
        let worker_rec = rec.clone();
        let worker_is_recording = state.is_recording.clone();
        let worker_handle = thread::spawn(move || {
            run_worker(cmd_rx, audio_tx, worker_rec, worker_is_recording);
        });

        // ── Monitor thread (device changes) ──────────────────────────
        let monitor_stop = Arc::new(AtomicBool::new(false));
        let monitor_stop_clone = monitor_stop.clone();
        let monitor_handle = thread::spawn(move || {
            monitor_device_changes(cmd_tx_for_monitor, rec, app_config, monitor_stop_clone);
        });

        Ok(Self {
            cmd_tx,
            monitor_stop,
            worker_handle: Some(worker_handle),
            transcribe_handle: Some(transcribe_handle),
            monitor_handle: Some(monitor_handle),
            state,
        })
    }

    /// Whether a recording is currently in progress.
    pub fn is_recording(&self) -> bool {
        self.state.is_recording.load(Ordering::Relaxed)
    }

    /// Tell the worker to begin capturing audio.
    pub fn start_recording(&self) -> bool {
        self.state.is_recording.store(true, Ordering::SeqCst);
        self.cmd_tx.send(Command::StartRecording).is_ok()
    }

    /// Tell the worker to stop capturing and send audio to transcription.
    pub fn stop_recording(&self) -> bool {
        self.state.is_recording.store(false, Ordering::SeqCst);
        self.cmd_tx.send(Command::StopRecording).is_ok()
    }

    /// Shut down all threads gracefully.
    pub fn stop(self) {
        // Signal monitor to exit
        self.monitor_stop.store(true, Ordering::Relaxed);
        // Signal worker to exit
        let _ = self.cmd_tx.send(Command::Shutdown);
        // Drop our handle's sender so the worker's recv loop terminates
        drop(self.cmd_tx);

        if let Some(handle) = self.worker_handle {
            let _ = handle.join();
        }
        // Worker dropped audio_tx → transcription thread sees closed channel
        if let Some(handle) = self.transcribe_handle {
            let _ = handle.join();
        }
        if let Some(handle) = self.monitor_handle {
            let _ = handle.join();
        }
    }
}

// ── Worker thread ────────────────────────────────────────────────────

fn run_worker(
    cmd_rx: mpsc::Receiver<Command>,
    audio_tx: mpsc::Sender<Vec<i16>>,
    rec: Arc<Mutex<crate::recorder::Recorder>>,
    is_recording: Arc<AtomicBool>,
) {
    let mut recording: Option<Recording> = None;

    for cmd in cmd_rx {
        match cmd {
            Command::StartRecording => {
                if recording.is_some() {
                    warn!("⚠ Worker: already recording — ignoring StartRecording");
                    continue;
                }
                let rec_guard = rec.lock().unwrap();
                match rec_guard.start() {
                    Ok(r) => {
                        drop(rec_guard);
                        recording = Some(r);
                        info!("🎤 Recording...");
                    }
                    Err(e) => {
                        error!("❌ Failed to start recording: {}", e);
                        is_recording.store(false, Ordering::SeqCst);
                    }
                }
            }

            Command::StopRecording => {
                let audio = match recording.take() {
                    Some(r) => {
                        info!("🛑 Stopping recording...");
                        r.stop()
                    }
                    None => {
                        warn!("⚠ Worker: StopRecording but no active recording");
                        Vec::new()
                    }
                };

                if !audio.is_empty() {
                    let _ = audio_tx.send(audio);
                }
            }

            Command::Shutdown => {
                // Force-stop any active recording before exit
                if let Some(r) = recording.take() {
                    let _ = r.stop();
                }
                info!("👋 Worker thread exiting");
                break;
            }
        }
    }
    // audio_tx dropped here → transcription thread exits
}

// ── Monitor thread ───────────────────────────────────────────────────

/// Monitor device changes and switch to first available device if current
/// device is lost. If recording is active when the device disconnects,
/// force-stop it via the worker command channel.
fn monitor_device_changes(
    cmd_tx: mpsc::Sender<Command>,
    rec: Arc<Mutex<crate::recorder::Recorder>>,
    config: Arc<Mutex<Config>>,
    stop_flag: Arc<AtomicBool>,
) {
    loop {
        thread::sleep(Duration::from_secs(3));
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        // Read current device ID from config
        let current_device_id = {
            let cfg = config.lock().unwrap();
            cfg.device_id.clone()
        };

        // Only monitor if we have a configured device
        if let Some(ref id) = current_device_id {
            match crate::recorder::list_input_devices() {
                Ok(devices) => {
                    if devices.is_empty() {
                        warn!("⚠ No audio input devices available");
                        continue;
                    }

                    let device_exists = devices.iter().any(|d| &d.id == id);
                    if device_exists {
                        continue; // Device still connected
                    }

                    warn!("⚠ Current device '{}' disconnected", id);

                    // 1. If recording, force-stop via worker
                    let _ = cmd_tx.send(Command::StopRecording);

                    // 2. Build a new Recorder for the first available device
                    let first = &devices[0];
                    warn!("🔄 Switching to: {}", first.name);

                    match crate::recorder::Recorder::new(Some(&first.id)) {
                        Ok((new_recorder, _)) => {
                            let mut rec_guard = rec.lock().unwrap();
                            *rec_guard = new_recorder;
                            info!("✅ Recorder reinitialized for: {}", first.name);
                        }
                        Err(e) => {
                            error!("❌ Failed to create recorder for new device: {}", e);
                            continue;
                        }
                    }

                    // 3. Persist the new device to config
                    {
                        let mut cfg = config.lock().unwrap();
                        cfg.device_id = Some(first.id.clone());
                        cfg.device_name = Some(first.name.clone());
                        let config_path = crate::config::default_path();
                        cfg.save(&config_path);
                    }
                    info!("💾 Config updated with new device: {}", first.name);
                }
                Err(e) => {
                    warn!("⚠ Failed to list devices during monitoring: {}", e);
                }
            }
        }
    }
}

// ── Model discovery ──────────────────────────────────────────────────

/// Enumerate all ggml-*.bin model files found in the given directories.
pub fn list_ggml_models(dirs: &[String]) -> Vec<std::path::PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for dir in dirs {
        let expanded = shellexpand::tilde(dir);
        let base = std::path::Path::new(expanded.as_ref());
        // Primary: <dir>/transcriber/
        let transcriber_dir = base.join("transcriber");
        scan_ggml_dir(&transcriber_dir, &mut paths, &mut seen);
        // Backward compat: <dir>/
        scan_ggml_dir(base, &mut paths, &mut seen);
    }
    paths
}

fn scan_ggml_dir(
    dir: &std::path::Path,
    paths: &mut Vec<std::path::PathBuf>,
    seen: &mut std::collections::HashSet<String>,
) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with("ggml-") && name_str.ends_with(".bin") {
                if seen.insert(name_str) {
                    paths.push(entry.path());
                }
            }
        }
    }
}

fn find_model(config: &Config) -> Option<std::path::PathBuf> {
    if let Some(ref model) = config.model {
        if Path::new(model).exists() {
            return Some(std::path::PathBuf::from(model));
        }
    }
    list_ggml_models(&config.model_search_dirs)
        .into_iter()
        .next()
}

// ── Platform-specific clipboard & paste ──────────────────────────────

#[cfg(target_os = "macos")]
fn paste_from_clipboard() {
    use std::process::Command;

    info!("⌨️ Using AppleScript for paste operation...");

    let script = r#"
        tell application "System Events"
            keystroke "v" using command down
        end tell
    "#;

    match Command::new("osascript").arg("-e").arg(script).output() {
        Ok(output) => {
            if output.status.success() {
                info!("⌨️ AppleScript paste executed successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("⚠ AppleScript paste failed: {}", stderr);
                error!("⚠️ Grant Accessibility permission: System Preferences → Privacy & Security → Accessibility");
                error!("⚠️ Add BOTH your terminal AND push-to-talk.app to the list");
            }
        }
        Err(e) => {
            error!("⚠ Failed to execute AppleScript: {}", e);
        }
    }
}

#[cfg(target_os = "windows")]
fn paste_from_clipboard() {
    use std::thread;
    use std::time::Duration;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };

    thread::sleep(Duration::from_millis(50));

    info!("⌨️ Simulating Ctrl+V via keybd_event...");

    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS::default(), 0);
        keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS::default(), 0);
        keybd_event(VK_V.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }

    info!("✅ Paste completed");
}

#[cfg(target_os = "macos")]
fn copy_to_clipboard(text: &str) {
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::NSString;

    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let ns_string = NSString::from_str(text);
        let result =
            pasteboard.setString_forType(&ns_string, objc2_app_kit::NSPasteboardTypeString);
        if !result {
            warn!("⚠ Failed to set clipboard text");
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_to_clipboard(text: &str) {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clip) => {
            let _ = clip.set_text(text);
        }
        Err(e) => warn!("⚠ Clipboard error: {}", e),
    }
}
