//! Push-to-Talk Tauri Application with Global State

#![windows_subsystem = "windows"]

mod config;
mod punctuator;
mod recorder;
mod transcriber;
mod voice_service;

use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Manager;
use tauri_plugin_single_instance;

// Global state accessible anywhere
static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub struct AppState {
    pub voice_service: Arc<Mutex<Option<voice_service::VoiceServiceHandle>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<config::Config>>,
    pub is_recording: Arc<AtomicBool>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
    pub punctuator: Arc<Mutex<Option<punctuator::Punctuator>>>,
}

/// Device info for frontend dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDto {
    pub id: String,
    pub name: String,
    pub config: String,
    pub is_default: bool,
}

/// Model info for frontend list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDto {
    pub filename: String,
    pub path: String,
    pub size: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            voice_service: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            config: Arc::new(Mutex::new(config::Config::default())),
            is_recording: Arc::new(AtomicBool::new(false)),
            last_transcription: Arc::new(Mutex::new(None)),
            punctuator: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

fn get_global_state() -> Option<&'static Arc<AppState>> {
    APP_STATE.get()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusDto {
    pub is_recording: bool,
    pub is_service_running: bool,
    pub hotkey: String,
    pub language: Option<String>,
    pub last_transcription: Option<String>,
}

#[tauri::command]
fn get_status() -> StatusDto {
    if let Some(state) = get_global_state() {
        let config = state.config.lock().unwrap();
        let is_running = *state.is_running.lock().unwrap();
        let is_recording = state.is_recording.load(Ordering::SeqCst);
        let last_transcription = state.last_transcription.lock().unwrap().clone();

        StatusDto {
            is_recording,
            is_service_running: is_running,
            hotkey: config.hotkey.clone(),
            language: config.language.clone(),
            last_transcription,
        }
    } else {
        StatusDto {
            is_recording: false,
            is_service_running: false,
            hotkey: String::new(),
            language: None,
            last_transcription: None,
        }
    }
}

#[tauri::command]
fn start_service() -> Result<(), String> {
    if let Some(state) = get_global_state() {
        // Lock in consistent order (config → is_running) to avoid deadlock
        // with get_status which acquires the same locks in the same order.
        let config = state.config.lock().unwrap().clone();
        let mut running = state.is_running.lock().unwrap();
        if *running {
            return Err("Service already running".to_string());
        }
        let last_transcription = state.last_transcription.clone();
        let is_recording = state.is_recording.clone();
        let app_config = state.config.clone();
        let punctuator = state.punctuator.clone();

        match voice_service::VoiceServiceHandle::start(
            config,
            last_transcription,
            is_recording,
            app_config,
            punctuator,
        ) {
            Ok(handle) => {
                *state.voice_service.lock().unwrap() = Some(handle);
                *running = true;
                Ok(())
            }
            Err(e) => Err(format!("Failed to start service: {}", e)),
        }
    } else {
        Err("State not initialized".to_string())
    }
}

#[tauri::command]
fn stop_service() -> Result<(), String> {
    if let Some(state) = get_global_state() {
        let mut running = state.is_running.lock().unwrap();
        if !*running {
            return Err("Service not running".to_string());
        }
        if let Some(handle) = state.voice_service.lock().unwrap().take() {
            handle.stop();
        }
        *running = false;
        // is_recording is reset by the handle's stop_recording() during teardown
        Ok(())
    } else {
        Err("State not initialized".to_string())
    }
}

#[tauri::command]
fn get_config() -> config::Config {
    get_global_state()
        .map(|s| s.config.lock().unwrap().clone())
        .unwrap_or_default()
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, mut cfg: config::Config) -> Result<(), String> {
    tracing::debug!("Save config called");

    let Some(state) = get_global_state() else {
        tracing::error!("💾 save_config: State not initialized");
        return Err("State not initialized".to_string());
    };

    // Normalize hotkey (handle old rdev-era aliases like "Ins" → "Insert")
    cfg.hotkey = normalize_hotkey(&cfg.hotkey);

    let config_path = config::default_path();
    cfg.save(&config_path); // save() handles errors internally

    // Update config and detect changes
    let (old_hotkey, old_language) = {
        let mut config = state.config.lock().unwrap();
        let old_hk = normalize_hotkey(&config.hotkey);
        let old_lang = config.language.clone();
        *config = cfg.clone();
        (old_hk, old_lang)
    };

    // Re-register global hotkey if it changed
    let new_hotkey = cfg.hotkey.clone();
    if new_hotkey != old_hotkey {
        use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

        // Unregister old hotkey
        if !old_hotkey.is_empty() {
            if let Ok(shortcut) = old_hotkey.parse::<Shortcut>() {
                let _ = app.global_shortcut().unregister(shortcut);
            }
        }

        // Register new hotkey
        if !new_hotkey.is_empty() {
            let normalized = normalize_hotkey(&new_hotkey);
            if let Ok(shortcut) = normalized.parse::<Shortcut>() {
                let shortcut_handler =
                    move |_app: &tauri::AppHandle, _id: &Shortcut, event: ShortcutEvent| {
                        handle_shortcut_event(event);
                    };
                app.global_shortcut()
                    .on_shortcut(shortcut, shortcut_handler)
                    .unwrap_or_else(|e| {
                        tracing::error!("❌ Failed to register hotkey '{}': {}", normalized, e)
                    });
                tracing::warn!("🎹 Global hotkey re-registered: {}", normalized);
            }
        }
    }

    // Update transcriber language immediately if changed
    let new_language = cfg.language.clone();
    if new_language != old_language {
        if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
            handle
                .state
                .transcriber
                .lock()
                .unwrap()
                .set_language(new_language.clone());
            tracing::info!("🌐 Language updated to {:?} (immediate)", new_language);
        }
    }

    // Reload punctuator if punctuation_enabled changed
    // (takes effect after service restart — same pattern as hotkey changes)
    {
        let old_punc_enabled = {
            let mut guard = state.punctuator.lock().unwrap();
            let was_enabled = guard.is_some();
            // If disabled now or was disabled before, clear/reload
            if !cfg.punctuation_enabled {
                *guard = None;
            }
            was_enabled
        };
        if cfg.punctuation_enabled && !old_punc_enabled {
            // Try to load the punctuator
            match punctuator::Punctuator::from_config(&cfg) {
                Ok(punc) => {
                    tracing::info!("✅ Punctuation restoration enabled (via config save)");
                    *state.punctuator.lock().unwrap() = Some(punc);
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠ Punctuation model not available: {} — \
                         enable will take effect once model is placed",
                        e
                    );
                }
            }
        }
    }

    tracing::debug!("💾 Config saved successfully");
    Ok(())
}

#[tauri::command]
fn trigger_recording() -> Result<(), String> {
    if let Some(state) = get_global_state() {
        toggle_recording_inner(state);
        Ok(())
    } else {
        Err("State not initialized".to_string())
    }
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn list_audio_devices() -> Result<Vec<DeviceDto>, String> {
    match crate::recorder::list_input_devices() {
        Ok(devices) => Ok(devices
            .into_iter()
            .map(|d| DeviceDto {
                id: d.id,
                name: d.name,
                config: d.config,
                is_default: d.is_default,
            })
            .collect()),
        Err(e) => Err(format!("Failed to list devices: {}", e)),
    }
}

#[tauri::command]
fn get_current_device() -> Result<Option<DeviceDto>, String> {
    if let Some(state) = get_global_state() {
        let config = state.config.lock().unwrap();
        if let Some(ref device_id) = config.device_id {
            // Try to find the device in the current list
            match crate::recorder::list_input_devices() {
                Ok(devices) => {
                    if let Some(device) = devices.into_iter().find(|d| &d.id == device_id) {
                        return Ok(Some(DeviceDto {
                            id: device.id,
                            name: device.name,
                            config: device.config,
                            is_default: device.is_default,
                        }));
                    }
                    // Device not found - return the stored config anyway
                    return Ok(Some(DeviceDto {
                        id: device_id.clone(),
                        name: config.device_name.clone().unwrap_or_default(),
                        config: String::new(),
                        is_default: false,
                    }));
                }
                Err(_) => {
                    return Ok(Some(DeviceDto {
                        id: device_id.clone(),
                        name: config.device_name.clone().unwrap_or_default(),
                        config: String::new(),
                        is_default: false,
                    }));
                }
            }
        }
        Ok(None)
    } else {
        Err("State not initialized".to_string())
    }
}

#[tauri::command]
fn scan_models(model_search_dirs: Vec<String>) -> Result<Vec<ModelDto>, String> {
    let mut models: Vec<ModelDto> = voice_service::list_ggml_models(&model_search_dirs)
        .into_iter()
        .filter_map(|path| {
            let filename = path.file_name()?.to_str()?.to_string();
            let metadata = path.metadata().ok()?;
            let size = format_size(metadata.len());
            Some(ModelDto {
                filename,
                path: path.to_string_lossy().to_string(),
                size,
            })
        })
        .collect();

    // Remove duplicates and sort
    models.sort_by(|a, b| a.filename.cmp(&b.filename));
    models.dedup_by(|a, b| a.filename == b.filename);

    Ok(models)
}

/// Entry in the downloadable-model catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadableModel {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub url: String,
}

/// Static catalog of all models available for download.
static MODEL_CATALOG: std::sync::LazyLock<Vec<DownloadableModel>> = std::sync::LazyLock::new(
    || {
        vec![
            DownloadableModel {
                id: "tiny".into(),
                name: "ggml-tiny.bin".into(),
                desc: "ggml-tiny.bin (41 MB)".into(),
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
                    .into(),
            },
            DownloadableModel {
                id: "base".into(),
                name: "ggml-base.bin".into(),
                desc: "ggml-base.bin (74 MB)".into(),
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
                    .into(),
            },
            DownloadableModel {
                id: "small".into(),
                name: "ggml-small.bin".into(),
                desc: "ggml-small.bin (244 MB)".into(),
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
                    .into(),
            },
            DownloadableModel {
                id: "medium".into(),
                name: "ggml-medium.bin".into(),
                desc: "ggml-medium.bin (769 MB)".into(),
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
                    .into(),
            },
            DownloadableModel {
                id: "large-v3".into(),
                name: "ggml-large-v3.bin".into(),
                desc: "ggml-large-v3.bin (3.1 GB)".into(),
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"
                    .into(),
            },
            DownloadableModel {
                id: "large-v3-russian".into(),
                name: "ggml-large-v3-russian-f16.bin".into(),
                desc: "Russian large-v3 f16 (3.1 GB)".into(),
                url: "https://huggingface.co/Pomni/whisper-large-v3-russian-ggml-allquants/resolve/main/ggml-large-v3-russian-f16.bin".into(),
            },
        ]
    },
);

#[tauri::command]
fn get_downloadable_models() -> Vec<DownloadableModel> {
    MODEL_CATALOG.clone()
}

#[tauri::command]
async fn download_model(model_id: String, target_dir: String) -> Result<String, String> {
    use futures_util::StreamExt;
    use reqwest::Client;
    use tokio::io::AsyncWriteExt;

    // Look up model in catalog — rejects unknown IDs
    let entry = MODEL_CATALOG
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Unknown model ID: '{model_id}'"))?
        .clone();

    let target_path = shellexpand::tilde(&target_dir).to_string();
    let target_path = Path::new(&target_path).join(&entry.name);

    // Write to a temporary .part file, then rename on success.
    // This prevents scan_models from discovering a zero-byte or partial file
    // mid-download and exposing it as a selectable (but broken) model.
    let part_path = target_path.with_extension("part");

    // Create directory if it doesn't exist
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let client = Client::new();
    let response = client
        .get(&entry.url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }

    let total_size = response.content_length();

    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded += chunk.len() as u64;
        if let Some(total) = total_size {
            let progress = (downloaded as f64 / total as f64 * 100.0) as u32;
            tracing::info!("⬇️ Downloading {}: {}%", entry.name, progress);
        } else {
            let mb = downloaded as f64 / (1024.0 * 1024.0);
            tracing::info!("⬇️ Downloading {}: {:.1} MB", entry.name, mb);
        }
    }

    // Atomically promote the completed download
    std::fs::rename(&part_path, &target_path)
        .map_err(|e| format!("Failed to finalize download: {}", e))?;

    Ok(format!(
        "Downloaded {} to {}",
        entry.name,
        target_path.display()
    ))
}

// ── Punctuation model download ───────────────────────────────────

const PUNCTUATION_MODEL_URL: &str =
    "https://huggingface.co/Aectann/punctuation-case-model/resolve/main/model.onnx";
const PUNCTUATION_TOKENIZER_URL: &str =
    "https://huggingface.co/Aectann/punctuation-case-model/resolve/main/tokenizer.json";

/// Result of checking whether the punctuation model is present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PunctuationModelStatus {
    pub found: bool,
    pub model_path: Option<String>,
    pub onnx_url: String,
    pub tokenizer_url: String,
}

#[tauri::command]
fn check_punctuation_model(model_search_dirs: Vec<String>) -> PunctuationModelStatus {
    let model_path = punctuator::find_punctuation_model(&model_search_dirs);
    PunctuationModelStatus {
        found: model_path.is_some(),
        model_path: model_path.map(|p| p.to_string_lossy().to_string()),
        onnx_url: PUNCTUATION_MODEL_URL.to_string(),
        tokenizer_url: PUNCTUATION_TOKENIZER_URL.to_string(),
    }
}

#[tauri::command]
async fn download_punctuation_model(target_dir: String) -> Result<String, String> {
    use futures_util::StreamExt;
    use reqwest::Client;
    use tokio::io::AsyncWriteExt;

    let target_dir = shellexpand::tilde(&target_dir).to_string();
    let target_dir = Path::new(&target_dir).join("punctuator");

    // Create target directory
    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let files: &[(&str, &str, &str)] = &[
        ("model.onnx", PUNCTUATION_MODEL_URL, "model.onnx.part"),
        ("tokenizer.json", PUNCTUATION_TOKENIZER_URL, "tokenizer.json.part"),
    ];

    let client = Client::new();
    let mut results = Vec::new();

    for (name, url, part_ext) in files {
        let target_path = target_dir.join(name);
        let part_path = target_dir.join(part_ext);

        tracing::info!("⬇️ Downloading {} ({})...", name, target_dir.display());

        let response = client
            .get(*url)
            .send()
            .await
            .map_err(|e| format!("Failed to start download for {name}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Download failed for {name}: HTTP {}", response.status()));
        }

        let total_size = response.content_length();

        let mut file = tokio::fs::File::create(&part_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;

            downloaded += chunk.len() as u64;
            if let Some(total) = total_size {
                let progress = (downloaded as f64 / total as f64 * 100.0) as u32;
                tracing::info!("⬇️ Downloading {name}: {progress}%");
            } else {
                let mb = downloaded as f64 / (1024.0 * 1024.0);
                tracing::info!("⬇️ Downloading {name}: {:.1} MB", mb);
            }
        }

        // Atomically promote
        std::fs::rename(&part_path, &target_path)
            .map_err(|e| format!("Failed to finalize {name}: {}", e))?;

        let size_mb = target_path
            .metadata()
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
        tracing::info!("✅ Downloaded {name} ({size_mb:.0} MB)");
        results.push(format!("{name} ({size_mb:.0} MB)"));
    }

    Ok(format!(
        "Punctuation model downloaded to {}",
        target_dir.display()
    ))
}

/// Receive log messages from frontend
#[tauri::command]
fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!("🌐 {}", message),
        "warn" => tracing::warn!("🌐 {}", message),
        "info" => tracing::info!("🌐 {}", message),
        "debug" => tracing::debug!("🌐 {}", message),
        "trace" => tracing::trace!("🌐 {}", message),
        _ => tracing::info!("🌐 {}", message),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn init_logging(config: &config::Config) {
    use std::fs;
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let log_dir = std::path::Path::new(&config.log_dir);
    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory: {}", e);
    }

    // Choose file extension and format variant based on config
    let (file_suffix, is_json) = match config.log_format.as_str() {
        "json" => ("json", true),
        _ => ("txt", false), // default: human-readable text
    };

    // Create minutely rolling file appender
    // Filename format: push-to-talk.YYYY-MM-DD-HH-MM.{txt,json}
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::MINUTELY)
        .filename_prefix("push-to-talk")
        .filename_suffix(file_suffix)
        .build(log_dir)
        .expect("Failed to create file appender");

    let file_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Console layer — always human-readable text, INFO level or higher
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(console_filter);

    // File layer — text or JSON depending on config
    let file_layer = {
        let layer = fmt::layer()
            .with_writer(file_appender)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false);
        if is_json {
            layer.json().with_filter(file_filter).boxed()
        } else {
            layer.with_filter(file_filter).boxed()
        }
    };

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("📝 Logging initialized to {}", log_dir.display());

    // Start log cleanup thread
    start_log_cleanup(log_dir.to_path_buf());
}

fn start_log_cleanup(log_dir: std::path::PathBuf) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            // Re-read retention from config so changes take effect without restart.
            let cfg = config::Config::load(&config::default_path());
            cleanup_old_logs(&log_dir, cfg.log_retention_hours);
        }
    });
}

fn cleanup_old_logs(log_dir: &std::path::Path, retention_hours: u64) {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let retention_secs = retention_hours * 3600;

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_secs = modified
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        if now - modified_secs > retention_secs {
                            let _ = fs::remove_file(entry.path());
                            tracing::info!("🗑️ Cleaned up old log: {:?}", entry.path());
                        }
                    }
                }
            }
        }
    }
}

fn toggle_recording_inner(state: &AppState) {
    if !*state.is_running.lock().unwrap() {
        return;
    }

    let is_recording = state.is_recording.load(Ordering::SeqCst);

    if is_recording {
        if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
            let _ = handle.stop_recording();
        }
        tracing::info!("🛑 Stopping recording");
    } else {
        if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
            handle.state.should_paste.store(false, Ordering::Relaxed);
            let _ = handle.start_recording();
        }
        tracing::info!("🎤 Starting recording");
    }
}

/// Normalize key names from the old rdev-based CLI to keyboard_types::Code format.
/// The old CLI used aliases like "Ins", "Del", "Esc" while global-hotkey
/// expects W3C spec names like "Insert", "Delete", "Escape".
fn normalize_hotkey(raw: &str) -> String {
    let mut parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();
    if let Some(key) = parts.last_mut() {
        let normalized = match key.to_lowercase().as_str() {
            "ins" => "Insert",
            "del" => "Delete",
            "esc" => "Escape",
            "enter" => "Enter",
            "return" => "Enter",
            "back" => "Backspace",
            "pgup" => "PageUp",
            "pgdn" => "PageDown",
            "prtsc" => "PrintScreen",
            "caps" => "CapsLock",
            "pause" => "Pause",
            "break" => "Pause",
            // Leave everything else unchanged (single letters, digits, F-keys, etc.)
            _ => return raw.to_string(),
        };
        *key = normalized;
    }
    parts.join("+")
}

/// Handle global shortcut press/release events.
/// Press starts recording, release stops it.
/// If the monitor force-stopped recording (device disconnect), release is a no-op
/// and the user must press again to start on the new device.
fn handle_shortcut_event(event: tauri_plugin_global_shortcut::ShortcutEvent) {
    use tauri_plugin_global_shortcut::ShortcutState;

    let Some(state) = get_global_state() else {
        tracing::error!("🚨 Shortcut: global state not initialized");
        return;
    };

    if !*state.is_running.lock().unwrap() {
        tracing::warn!("🚨 Shortcut: service not running, ignoring hotkey");
        return;
    }

    tracing::warn!(
        "⌨️ Shortcut event: state={:?}, is_recording={}",
        event.state,
        state.is_recording.load(Ordering::Relaxed)
    );

    match event.state {
        ShortcutState::Pressed => {
            if !state.is_recording.load(Ordering::SeqCst) {
                if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
                    handle.state.should_paste.store(true, Ordering::Relaxed);
                    let _ = handle.start_recording();
                }
                tracing::info!("🎤 Hotkey press — starting recording");
            }
        }
        ShortcutState::Released => {
            if state.is_recording.load(Ordering::SeqCst) {
                if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
                    let _ = handle.stop_recording();
                }
                tracing::info!("🛑 Hotkey release — stopping recording");
            }
        }
    }
}

fn main() {
    // Install panic hook to log panics before exiting
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("🚨 PANIC: {}", panic_info);
        eprintln!("🚨 PANIC: {}", panic_info);
    }));

    // Discover ONNX Runtime native library for punctuation restoration.
    // Tauri bundles it as a resource; location varies by platform:
    //   Windows: next to the .exe
    //   macOS:   Contents/Resources/ (one level up from MacOS/ binary)
    //   Linux:   next to the binary
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Platform-specific library name and location
                #[cfg(target_os = "windows")]
                let (lib_name, search_dir) = ("onnxruntime.dll", exe_dir.to_path_buf());

                #[cfg(target_os = "macos")]
                let (lib_name, search_dir) = {
                    // Binary: Contents/MacOS/push-to-talk
                    // Resources: Contents/Resources/libonnxruntime.dylib
                    let res_dir = exe_dir.parent().map(|p| p.join("Resources"));
                    ("libonnxruntime.dylib", res_dir.unwrap_or_else(|| exe_dir.to_path_buf()))
                };

                #[cfg(target_os = "linux")]
                let (lib_name, search_dir) = ("libonnxruntime.so", exe_dir.to_path_buf());

                let lib_path = search_dir.join(lib_name);
                if lib_path.exists() {
                    std::env::set_var("ORT_DYLIB_PATH", lib_path);
                    // Note: tracing is not initialized yet — the punctuator init
                    // will log success/failure once logging is up.
                }
            }
        }
    }

    let config_path = config::default_path();
    let cfg = config::Config::load(&config_path);

    // Initialize logging first
    init_logging(&cfg);

    let app_state = AppState::new();
    *app_state.config.lock().unwrap() = cfg;
    let app_state_arc = Arc::new(app_state);

    // Initialize global state BEFORE tauri::Builder
    let _ = APP_STATE.set(app_state_arc.clone());

    // Initialize punctuation restoration if enabled in config
    {
        let cfg = app_state_arc.config.lock().unwrap();
        if cfg.punctuation_enabled {
            let result = punctuator::Punctuator::from_config(&cfg);
            drop(cfg); // release lock before storing punctuator
            match result {
                Ok(punc) => {
                    tracing::info!("✅ Punctuation restoration enabled");
                    *app_state_arc.punctuator.lock().unwrap() = Some(punc);
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠ Punctuation restoration unavailable: {} — \
                         transcriptions will not be punctuated",
                        e
                    );
                }
            }
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_single_instance::Builder::new()
                .callback(|app, _argv, _cwd| {
                    tracing::info!("🔄 Another instance was launched - focusing existing window");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_service,
            stop_service,
            get_config,
            save_config,
            trigger_recording,
            hide_window,
            list_audio_devices,
            get_current_device,
            scan_models,
            get_downloadable_models,
            download_model,
            check_punctuation_model,
            download_punctuation_model,
            frontend_log,
        ])
        .setup(move |app| {
            // Prevent app from exiting when window is closed (tray app behavior)
            {
                let config_clone = app_state_arc.config.clone();
                if let Some(window) = app.get_webview_window("main") {
                    let window_clone = window.clone();
                    window.on_window_event(move |event| {
                        match event {
                            tauri::WindowEvent::CloseRequested { api, .. } => {
                                // Prevent window close, hide instead
                                api.prevent_close();
                                let _ = window_clone.hide();
                                // Save window state to config
                                let mut config = config_clone.lock().unwrap();
                                config.window_hidden = true;
                                let config_path = crate::config::default_path();
                                config.save(&config_path);
                                tracing::info!(
                                    "🪟 Window hidden - state saved (window_hidden=true)"
                                );
                            }
                            tauri::WindowEvent::Destroyed => {
                                tracing::warn!("⚠️ Window destroyed!");
                            }
                            tauri::WindowEvent::Focused(false) => {
                                tracing::debug!("🪟 Window lost focus");
                            }
                            _ => {}
                        }
                    });
                    tracing::info!("🪟 Window close handler registered");
                } else {
                    tracing::warn!("⚠️ Could not get main window for close handler");
                }
            }

            // Restore window state from config (window created hidden)
            {
                let config = app_state_arc.config.lock().unwrap();
                let window_hidden = config.window_hidden;
                drop(config);

                if !window_hidden {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        tracing::info!("🪟 Window shown on startup (restored from config)");
                    }
                }
            }

            // Register global hotkey
            let config = app_state_arc.config.lock().unwrap();
            let hotkey = config.hotkey.clone();
            drop(config);

            if !hotkey.is_empty() {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};
                let normalized = normalize_hotkey(&hotkey);
                if let Ok(shortcut) = normalized.parse::<Shortcut>() {
                    let shortcut_handler =
                        move |_app: &tauri::AppHandle, _id: &Shortcut, event: ShortcutEvent| {
                            handle_shortcut_event(event);
                        };
                    app.global_shortcut()
                        .on_shortcut(shortcut, shortcut_handler)
                        .unwrap_or_else(|e| {
                            tracing::error!("❌ Failed to register hotkey '{}': {}", normalized, e)
                        });
                    tracing::warn!("🎹 Global hotkey registered: {}", normalized);
                } else {
                    tracing::error!("❌ Invalid hotkey format: {}", normalized);
                }
            }

            // System tray with menu (Configure / Quit) + double-click to show
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{TrayIconBuilder, TrayIconEvent};

                let config_clone = app_state_arc.config.clone();
                let config_dbl = app_state_arc.config.clone();
                let app_handle = app.handle().clone();
                let show_i = MenuItem::with_id(app, "show", "Configure", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

                let mut tray_builder = TrayIconBuilder::new()
                    .menu(&menu)
                    .show_menu_on_left_click(false);

                if let Some(icon) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon);
                }

                tray_builder = tray_builder
                    .on_tray_icon_event(move |_tray, event| {
                        if let TrayIconEvent::DoubleClick { .. } = event {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                if !window.is_visible().unwrap_or(false) {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let mut config = config_dbl.lock().unwrap();
                                    config.window_hidden = false;
                                    let config_path = crate::config::default_path();
                                    config.save(&config_path);
                                    tracing::info!(
                                        "🪟 Window shown via double-click (window_hidden=false)"
                                    );
                                }
                            }
                        }
                    })
                    .on_menu_event(move |app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                // Reset window_hidden state
                                let mut config = config_clone.lock().unwrap();
                                config.window_hidden = false;
                                let config_path = crate::config::default_path();
                                config.save(&config_path);
                                tracing::info!(
                                    "🪟 Window shown - state saved (window_hidden=false)"
                                );
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    });

                let _tray = tray_builder.build(app)?;
            }

            // Start the voice service
            let _ = start_service();

            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!("🚨 Tauri application error: {}", e);
            eprintln!("🚨 Tauri application error: {}", e);
        });

    tracing::info!("👋 Tauri event loop exited");
    eprintln!("👋 Tauri event loop exited");
}
