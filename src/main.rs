//! Push-to-Talk Tauri Application with Global State

mod config;
mod recorder;
mod transcriber;
mod voice_service;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tauri::Manager;
use once_cell::sync::OnceCell;

// Global state accessible anywhere
static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();

pub struct AppState {
    pub voice_service: Arc<Mutex<Option<voice_service::VoiceServiceHandle>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<config::Config>>,
    pub is_recording: Arc<AtomicBool>,
}

/// Device info for frontend dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDto {
    pub id: String,
    pub name: String,
    pub config: String,
    pub is_default: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            voice_service: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            config: Arc::new(Mutex::new(config::Config::default())),
            is_recording: Arc::new(AtomicBool::new(false)),
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
}

#[tauri::command]
fn get_status() -> StatusDto {
    if let Some(state) = get_global_state() {
        let config = state.config.lock().unwrap();
        let is_running = *state.is_running.lock().unwrap();
        let is_recording = state.is_recording.load(Ordering::SeqCst);

        StatusDto {
            is_recording,
            is_service_running: is_running,
            hotkey: config.hotkey.clone(),
            language: config.language.clone(),
        }
    } else {
        StatusDto {
            is_recording: false,
            is_service_running: false,
            hotkey: String::new(),
            language: None,
        }
    }
}

#[tauri::command]
fn start_service() -> Result<(), String> {
    if let Some(state) = get_global_state() {
        let mut running = state.is_running.lock().unwrap();
        if *running {
            return Err("Service already running".to_string());
        }
        let config = state.config.lock().unwrap().clone();
        
        match voice_service::VoiceServiceHandle::start(config) {
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
        state.is_recording.store(false, Ordering::SeqCst);
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
fn save_config(app: tauri::AppHandle, cfg: config::Config) -> Result<(), String> {
    if let Some(state) = get_global_state() {
        let config_path = config::default_path();
        cfg.save(&config_path);
        
        // Update config and re-register hotkey
        let old_hotkey = {
            let mut config = state.config.lock().unwrap();
            let old = config.hotkey.clone();
            *config = cfg.clone();
            old
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
                if let Ok(shortcut) = new_hotkey.parse::<Shortcut>() {
                    let shortcut_handler = move |_app: &tauri::AppHandle, _id: &Shortcut, _event: ShortcutEvent| {
                        let _ = trigger_recording();
                    };
                    app.global_shortcut()
                        .on_shortcut(shortcut, shortcut_handler)
                        .unwrap_or_else(|e| tracing::warn!("Failed to register hotkey '{}': {}", new_hotkey, e));
                    tracing::info!("🎹 Global hotkey re-registered: {}", new_hotkey);
                }
            }
        }
        
        Ok(())
    } else {
        Err("State not initialized".to_string())
    }
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

fn init_logging(config: &config::Config) {
    use std::fs;
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let log_dir = std::path::Path::new(&config.log_dir);
    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory: {}", e);
    }

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("push-to-talk")
        .filename_suffix(config.log_format.clone())
        .build(log_dir)
        .expect("Failed to create file appender");

    let file_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Console layer
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // File layer with dynamic formatting
    let registry = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_filter);

    match config.log_format.as_str() {
        "json" => {
            let file_layer = fmt::layer()
                .with_writer(file_appender)
                .json()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false);
            registry.with(file_layer).init();
        }
        _ => {
            let file_layer = fmt::layer()
                .with_writer(file_appender)
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false);
            registry.with(file_layer).init();
        }
    }

    tracing::info!("📝 Logging initialized to {}", log_dir.display());
}

fn toggle_recording_inner(state: &AppState) {
    if !*state.is_running.lock().unwrap() {
        return;
    }
    
    let was_recording = state.is_recording.swap(true, Ordering::SeqCst);
    
    if was_recording {
        state.is_recording.store(false, Ordering::SeqCst);
        if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
            let _ = handle.stop_recording();
        }
        tracing::info!("🛑 Stopping recording");
    } else {
        if let Some(handle) = state.voice_service.lock().unwrap().as_ref() {
            let _ = handle.start_recording();
        }
        tracing::info!("🎤 Starting recording");
    }
}

fn main() {
    let config_path = config::default_path();
    let cfg = config::Config::load(&config_path);

    // Initialize logging first
    init_logging(&cfg);

    let app_state = AppState::new();
    *app_state.config.lock().unwrap() = cfg;
    let app_state_arc = Arc::new(app_state);

    // Initialize global state BEFORE tauri::Builder
    let _ = APP_STATE.set(app_state_arc.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_service,
            stop_service,
            get_config,
            save_config,
            trigger_recording,
            list_audio_devices,
            get_current_device,
        ])
        .setup(move |app| {
            // Register global hotkey
            let config = app_state_arc.config.lock().unwrap();
            let hotkey = config.hotkey.clone();
            drop(config);

            if !hotkey.is_empty() {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};
                if let Ok(shortcut) = hotkey.parse::<Shortcut>() {
                    let shortcut_handler = move |_app: &tauri::AppHandle, _id: &Shortcut, _event: ShortcutEvent| {
                        let _ = trigger_recording();
                    };
                    app.global_shortcut()
                        .on_shortcut(shortcut, shortcut_handler)
                        .unwrap_or_else(|e| tracing::warn!("Failed to register hotkey '{}': {}", hotkey, e));
                    tracing::info!("🎹 Global hotkey registered: {}", hotkey);
                } else {
                    tracing::warn!("Invalid hotkey format: {}", hotkey);
                }
            }

            // Window event handler - hide instead of close
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;

                let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
                let toggle_i = MenuItem::with_id(app, "toggle", "Toggle Recording", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &toggle_i, &quit_i])?;

                let mut tray_builder = TrayIconBuilder::new()
                    .menu(&menu)
                    .show_menu_on_left_click(false);

                if let Some(icon) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon);
                }

                tray_builder = tray_builder.on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "toggle" => {
                            let _ = trigger_recording();
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                });

                let _tray = tray_builder.build(app)?;
            }

            // Start the voice service
            let _ = start_service();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
