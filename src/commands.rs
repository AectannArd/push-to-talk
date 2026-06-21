//! Tauri IPC command handlers — the boundary between frontend and backend.
//!
//! Each function is a `#[tauri::command]` invoked by the React frontend via
//! `window.__TAURI__.core.invoke()`. Commands are thin: they extract state,
//! delegate to domain modules, and return results.

use crate::config;
use crate::hotkey;
use crate::models::{self, PunctuationModelStatus};
use crate::punctuator;
use crate::recorder;
use crate::state::{self, DeviceDto, ModelDto, StatusDto};
use crate::voice_service;
use std::path::Path;

// ── Service lifecycle ────────────────────────────────────────────────

#[tauri::command]
pub fn get_status() -> StatusDto {
    if let Some(state) = state::get_global_state() {
        let config = state.config.lock().unwrap();
        let is_running = *state.is_running.lock().unwrap();
        let is_recording = state.is_recording.load(std::sync::atomic::Ordering::SeqCst);
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
pub fn start_service() -> Result<(), String> {
    if let Some(state) = state::get_global_state() {
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
pub fn stop_service() -> Result<(), String> {
    if let Some(state) = state::get_global_state() {
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
pub fn get_config() -> config::Config {
    state::get_global_state()
        .map(|s| s.config.lock().unwrap().clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, mut cfg: config::Config) -> Result<(), String> {
    tracing::debug!("Save config called");

    let Some(state) = state::get_global_state() else {
        tracing::error!("💾 save_config: State not initialized");
        return Err("State not initialized".to_string());
    };

    // Normalize hotkey (handle old rdev-era aliases like "Ins" → "Insert")
    cfg.hotkey = hotkey::normalize_hotkey(&cfg.hotkey);

    let config_path = config::default_path();
    cfg.save(&config_path); // save() handles errors internally

    // Update config and detect changes
    let (old_hotkey, old_language) = {
        let mut config = state.config.lock().unwrap();
        let old_hk = hotkey::normalize_hotkey(&config.hotkey);
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
            let normalized = hotkey::normalize_hotkey(&new_hotkey);
            if let Ok(shortcut) = normalized.parse::<Shortcut>() {
                let shortcut_handler =
                    move |_app: &tauri::AppHandle, _id: &Shortcut, event: ShortcutEvent| {
                        hotkey::handle_shortcut_event(event);
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

// ── Recording ────────────────────────────────────────────────────────

#[tauri::command]
pub fn trigger_recording() -> Result<(), String> {
    if let Some(state) = state::get_global_state() {
        hotkey::toggle_recording_inner(state);
        Ok(())
    } else {
        Err("State not initialized".to_string())
    }
}

#[tauri::command]
pub fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

// ── Audio devices ────────────────────────────────────────────────────

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<DeviceDto>, String> {
    match recorder::list_input_devices() {
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
pub fn get_current_device() -> Result<Option<DeviceDto>, String> {
    if let Some(state) = state::get_global_state() {
        let config = state.config.lock().unwrap();
        if let Some(ref device_id) = config.device_id {
            // Try to find the device in the current list
            match recorder::list_input_devices() {
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

// ── Model management ─────────────────────────────────────────────────

#[tauri::command]
pub fn scan_models(model_search_dirs: Vec<String>) -> Result<Vec<ModelDto>, String> {
    let mut models: Vec<ModelDto> = models::list_ggml_models(&model_search_dirs)
        .into_iter()
        .filter_map(|path| {
            let filename = path.file_name()?.to_str()?.to_string();
            let metadata = path.metadata().ok()?;
            let size = models::format_size(metadata.len());
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

#[tauri::command]
pub fn get_downloadable_models() -> Vec<models::DownloadableModel> {
    models::MODEL_CATALOG.clone()
}

#[tauri::command]
pub async fn download_model(model_id: String, target_dir: String) -> Result<String, String> {
    use futures_util::StreamExt;
    use reqwest::Client;
    use tokio::io::AsyncWriteExt;

    // Look up model in catalog — rejects unknown IDs
    let entry = models::MODEL_CATALOG
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

// ── Punctuation model ────────────────────────────────────────────────

#[tauri::command]
pub fn check_punctuation_model(model_search_dirs: Vec<String>) -> PunctuationModelStatus {
    let model_path = punctuator::find_punctuation_model(&model_search_dirs);
    PunctuationModelStatus {
        found: model_path.is_some(),
        model_path: model_path.map(|p| p.to_string_lossy().to_string()),
        onnx_url: models::PUNCTUATION_MODEL_URL.to_string(),
        tokenizer_url: models::PUNCTUATION_TOKENIZER_URL.to_string(),
    }
}

#[tauri::command]
pub async fn download_punctuation_model(target_dir: String) -> Result<String, String> {
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
        (
            "model.onnx",
            models::PUNCTUATION_MODEL_URL,
            "model.onnx.part",
        ),
        (
            "tokenizer.json",
            models::PUNCTUATION_TOKENIZER_URL,
            "tokenizer.json.part",
        ),
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
            return Err(format!(
                "Download failed for {name}: HTTP {}",
                response.status()
            ));
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

// ── Frontend logging ─────────────────────────────────────────────────

/// Receive log messages from the frontend and forward to tracing.
#[tauri::command]
pub fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!("🌐 {}", message),
        "warn" => tracing::warn!("🌐 {}", message),
        "info" => tracing::info!("🌐 {}", message),
        "debug" => tracing::debug!("🌐 {}", message),
        "trace" => tracing::trace!("🌐 {}", message),
        _ => tracing::info!("🌐 {}", message),
    }
}
