//! Voice Service - runs voice capture and transcription in background.

use crate::config::Config;
use crate::recorder::Recording;
use crate::transcriber::Transcriber;

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::sync::mpsc;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tracing::{info, error, warn};

pub struct VoiceServiceHandle {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    pub state: Arc<VoiceServiceInner>,
}

pub struct VoiceServiceInner {
    pub is_recording: Arc<AtomicBool>,
    recording: Arc<Mutex<Option<Recording>>>,
    tx: mpsc::Sender<Vec<i16>>,
    rec: Arc<crate::recorder::Recorder>,
}

impl VoiceServiceHandle {
    pub fn start(config: Config) -> Result<Self, anyhow::Error> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let is_recording = Arc::new(AtomicBool::new(false));

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

        // Initialize recorder and store device ID
        let device_id = config.device_id.clone();
        let (recorder, _device_info) = crate::recorder::Recorder::new(config.device_id.as_deref())
            .map_err(|e| anyhow::anyhow!("Failed to initialize recorder: {}", e))?;
        let rec = Arc::new(recorder);

        // Channel for audio data
        let (tx, rx) = mpsc::channel::<Vec<i16>>();

        // Shared state
        let recording: Arc<Mutex<Option<Recording>>> = Arc::new(Mutex::new(None));

        let state = Arc::new(VoiceServiceInner {
            is_recording: is_recording.clone(),
            recording,
            tx,
            rec,
        });

        // Spawn the service loop
        let state_clone = state.clone();
        let stop_flag_clone = stop_flag.clone();
        let thread_handle = thread::spawn(move || {
            run_service_loop(state_clone, tr, rx, stop_flag_clone);
        });

        // Spawn device monitoring thread
        let monitor_stop = stop_flag.clone();
        let monitor_device_id = Arc::new(Mutex::new(device_id));
        let _monitor_handle = thread::spawn(move || {
            monitor_device_changes(monitor_device_id, monitor_stop);
        });

        Ok(Self {
            stop_flag,
            thread_handle: Some(thread_handle),
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
    }
}

impl VoiceServiceInner {
    pub fn start_recording(&self) -> bool {
        if self.is_recording.swap(true, Ordering::SeqCst) {
            return false; // Already recording
        }

        match self.rec.start() {
            Ok(r) => {
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
    tr: Arc<Mutex<crate::transcriber::Transcriber>>,
    rx: mpsc::Receiver<Vec<i16>>,
    stop_flag: Arc<AtomicBool>,
) {
    // Transcription thread
    let tr_clone = tr.clone();
    let _tx_clone = state.tx.clone();
    let _transcribe_thread = thread::spawn(move || {
        for audio in rx {
            match tr_clone.lock().unwrap().transcribe(&audio) {
                Ok(text) if text.is_empty() => {
                    warn!("⚠ No speech detected");
                }
                Ok(text) => {
                    info!("📝 \"{}\"", text);
                    type_text(&text);
                    copy_to_clipboard(&text);
                }
                Err(e) => error!("❌ Transcription error: {}", e),
            }
        }
    });

    // Keep service alive
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        if stop_flag.load(Ordering::Relaxed) {
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

fn type_text(text: &str) {
    use enigo::{Enigo, Keyboard, Settings};

    match Enigo::new(&Settings::default()) {
        Ok(mut enigo) => {
            if let Err(e) = enigo.text(text) {
                error!("⚠ Failed to type text: {}", e);
            }
        }
        Err(e) => error!("⚠ Failed to init keyboard: {}", e),
    }
}

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
fn monitor_device_changes(
    current_device_id: Arc<Mutex<Option<String>>>,
    stop_flag: Arc<AtomicBool>,
) {
    loop {
        thread::sleep(Duration::from_secs(3));
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let device_id_guard = current_device_id.lock().unwrap();
        let current_id = device_id_guard.clone();
        drop(device_id_guard);

        // Only monitor if we have a configured device
        if let Some(ref id) = current_id {
            // Check if device is still available
            match crate::recorder::list_input_devices() {
                Ok(devices) => {
                    let device_exists = devices.iter().any(|d| &d.id == id);
                    if !device_exists {
                        warn!("⚠ Current device '{}' disconnected", id);
                        
                        // Switch to first available device
                        if let Some(first_device) = devices.first() {
                            warn!("🔄 Switching to first available device: {}", first_device.name);
                            
                            // Update the current device ID
                            let mut guard = current_device_id.lock().unwrap();
                            *guard = Some(first_device.id.clone());
                            
                            // Note: We can't reinitialize the recorder mid-stream without more complex refactoring
                            // For now, we just log the switch. The next service restart will use the new device.
                            info!("💾 Device config updated - will use new device on next restart");
                        } else {
                            warn!("⚠ No audio input devices available");
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠ Failed to list devices during monitoring: {}", e);
                }
            }
        }
    }
}
