//! Voice Service - runs voice capture and transcription in background.

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

pub struct VoiceServiceHandle {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    monitor_handle: Option<thread::JoinHandle<()>>,
    pub state: Arc<VoiceServiceInner>,
}

pub struct VoiceServiceInner {
    pub is_recording: Arc<AtomicBool>,
    pub should_paste: Arc<AtomicBool>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
    pub transcriber: Arc<Mutex<crate::transcriber::Transcriber>>,
    recording: Arc<Mutex<Option<Recording>>>,
    tx: mpsc::Sender<Vec<i16>>,
    rec: Arc<Mutex<crate::recorder::Recorder>>,
}

impl VoiceServiceHandle {
    pub fn start(
        config: Config,
        last_transcription: Arc<Mutex<Option<String>>>,
        app_is_recording: Arc<AtomicBool>,
        app_config: Arc<Mutex<Config>>,
    ) -> Result<Self, anyhow::Error> {
        let stop_flag = Arc::new(AtomicBool::new(false));

        // Load model first
        let model_path = match find_model(&config) {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("Model not found"));
            }
        };

        let transcriber = Transcriber::new(&model_path, config.language.clone())
            .map_err(|e| anyhow::anyhow!("Failed to load transcriber: {}", e))?;
        let tr = Arc::new(Mutex::new(transcriber));

        // Initialize recorder
        let (recorder, _device_info) = crate::recorder::Recorder::new(config.device_id.as_deref())
            .map_err(|e| anyhow::anyhow!("Failed to initialize recorder: {}", e))?;
        let rec = Arc::new(Mutex::new(recorder));

        // Channel for audio data
        let (tx, rx) = mpsc::channel::<Vec<i16>>();

        // Shared state
        let recording: Arc<Mutex<Option<Recording>>> = Arc::new(Mutex::new(None));

        let state = Arc::new(VoiceServiceInner {
            is_recording: app_is_recording,
            should_paste: Arc::new(AtomicBool::new(false)),
            last_transcription,
            transcriber: tr.clone(),
            recording,
            tx,
            rec,
        });

        // Spawn the service loop
        let state_clone = state.clone();
        let stop_flag_clone = stop_flag.clone();
        let thread_handle = thread::spawn(move || {
            run_service_loop(state_clone, rx, stop_flag_clone);
        });

        // Spawn device monitoring thread
        let monitor_state = state.clone();
        let monitor_config = app_config;
        let monitor_stop = stop_flag.clone();
        let monitor_handle = thread::spawn(move || {
            monitor_device_changes(monitor_state, monitor_config, monitor_stop);
        });

        Ok(Self {
            stop_flag,
            thread_handle: Some(thread_handle),
            monitor_handle: Some(monitor_handle),
            state,
        })
    }

    pub fn is_recording(&self) -> bool {
        self.state.is_recording.load(Ordering::Relaxed)
    }

    pub fn start_recording(&self) -> bool {
        self.state.start_recording()
    }

    pub fn stop_recording(&self) -> bool {
        self.state.stop_recording()
    }

    pub fn stop(self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle {
            let _ = handle.join();
        }
        if let Some(handle) = self.monitor_handle {
            let _ = handle.join();
        }
    }
}

impl VoiceServiceInner {
    pub fn start_recording(&self) -> bool {
        if self.is_recording.swap(true, Ordering::SeqCst) {
            return false; // Already recording
        }

        let rec_guard = self.rec.lock().unwrap();
        match rec_guard.start() {
            Ok(r) => {
                drop(rec_guard);
                *self.recording.lock().unwrap() = Some(r);
                info!("🎤 Recording...");
                true
            }
            Err(e) => {
                error!("❌ Failed to start recording: {}", e);
                self.is_recording.store(false, Ordering::SeqCst);
                false
            }
        }
    }

    pub fn stop_recording(&self) -> bool {
        if !self.is_recording.swap(false, Ordering::SeqCst) {
            return false; // Wasn't recording
        }

        let audio = {
            let mut guard = self.recording.lock().unwrap();
            match guard.take() {
                Some(r) => {
                    info!("🛑 Stopping recording...");
                    r.stop()
                }
                None => Vec::new(),
            }
        };

        if !audio.is_empty() {
            let _ = self.tx.send(audio);
            true
        } else {
            false
        }
    }
}

fn run_service_loop(
    state: Arc<VoiceServiceInner>,
    rx: mpsc::Receiver<Vec<i16>>,
    stop_flag: Arc<AtomicBool>,
) {
    // Transcription thread
    let tr_clone = state.transcriber.clone();
    let last_transcription = state.last_transcription.clone();
    let should_paste = state.should_paste.clone();
    let _tx_clone = state.tx.clone();
    let _transcribe_thread = thread::spawn(move || {
        for audio in rx {
            match tr_clone.lock().unwrap().transcribe(&audio) {
                Ok(text) if text.is_empty() => {
                    warn!("⚠ No speech detected");
                }
                Ok(text) => {
                    info!("📝 \"{}\"", text);
                    // Update last transcription (always)
                    *last_transcription.lock().unwrap() = Some(text.clone());
                    // Copy & paste only when triggered by global hotkey
                    if should_paste.load(Ordering::Relaxed) {
                        copy_to_clipboard(&text);
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

    info!("✅ Voice service loop started");

    // Keep service alive
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        if stop_flag.load(Ordering::Relaxed) {
            info!("🛑 Stop flag received, exiting voice service loop");
            break;
        }
    }

    info!("👋 Voice service stopped");
}

fn find_model(config: &Config) -> Option<std::path::PathBuf> {
    if let Some(ref model) = config.model {
        if Path::new(model).exists() {
            return Some(std::path::PathBuf::from(model));
        }
    }

    for dir in &config.model_search_dirs {
        let path = std::path::Path::new(dir);
        if path.exists() {
            for entry in std::fs::read_dir(path).ok()? {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("ggml-") && name.ends_with(".bin") {
                    return Some(entry.path());
                }
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn paste_from_clipboard() {
    use std::process::Command;

    info!("⌨️ Using AppleScript for paste operation...");

    // Use AppleScript to simulate Cmd+V paste
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

    // Give clipboard time to settle
    thread::sleep(Duration::from_millis(50));

    info!("⌨️ Simulating Ctrl+V via keybd_event...");

    unsafe {
        // Press Ctrl
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS::default(), 0);
        // Press V
        keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS::default(), 0);
        // Release V
        keybd_event(VK_V.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        // Release Ctrl
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }

    info!("✅ Paste completed");
}

#[cfg(target_os = "linux")]
fn paste_from_clipboard() {
    // On Linux, paste is handled by the frontend or user manually
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

/// Monitor device changes and switch to first available device if current device is lost.
/// If recording is active when the device disconnects, force-stop it immediately.
fn monitor_device_changes(
    state: Arc<VoiceServiceInner>,
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
            // Check if device is still available
            match crate::recorder::list_input_devices() {
                Ok(devices) => {
                    if devices.is_empty() {
                        warn!("⚠ No audio input devices available");
                        continue;
                    }

                    let device_exists = devices.iter().any(|d| &d.id == id);
                    if device_exists {
                        continue; // Device still connected, nothing to do
                    }

                    warn!("⚠ Current device '{}' disconnected", id);

                    // 1. If currently recording, force-stop immediately
                    if state.is_recording.load(Ordering::SeqCst) {
                        warn!("⚠ Device disconnected during active recording — forcing stop");
                        state.stop_recording();
                    }

                    // 2. Build a new Recorder for the first available device
                    let first = &devices[0];
                    warn!("🔄 Switching to: {}", first.name);

                    match crate::recorder::Recorder::new(Some(&first.id)) {
                        Ok((new_recorder, _)) => {
                            // 3. Atomically swap in the new recorder
                            {
                                let mut rec_guard = state.rec.lock().unwrap();
                                *rec_guard = new_recorder;
                            }
                            info!("✅ Recorder reinitialized for: {}", first.name);
                        }
                        Err(e) => {
                            error!("❌ Failed to create recorder for new device: {}", e);
                            continue; // Skip config update — keep trying on next poll
                        }
                    }

                    // 4. Persist the new device to config
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
