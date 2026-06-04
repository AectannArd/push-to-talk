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
fn save_config(cfg: config::Config) -> Result<(), String> {
    if let Some(state) = get_global_state() {
        let config_path = config::default_path();
        cfg.save(&config_path);
        *state.config.lock().unwrap() = cfg;
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
    
    let app_state = AppState::new();
    *app_state.config.lock().unwrap() = cfg;
    let app_state_arc = Arc::new(app_state);
    
    // Initialize global state BEFORE tauri::Builder
    let _ = APP_STATE.set(app_state_arc.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_service,
            stop_service,
            get_config,
            save_config,
            trigger_recording,
        ])
        .setup(move |_app| {
            // Window event handler - hide instead of close
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;

                let show_i = MenuItem::with_id(_app, "show", "Show", true, None::<&str>)?;
                let toggle_i = MenuItem::with_id(_app, "toggle", "Toggle Recording", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(_app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(_app, &[&show_i, &toggle_i, &quit_i])?;

                let mut tray_builder = TrayIconBuilder::new()
                    .menu(&menu)
                    .show_menu_on_left_click(false);
                
                if let Some(icon) = _app.default_window_icon().cloned() {
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
                
                let _tray = tray_builder.build(_app)?;
            }

            // Start the voice service
            let _ = start_service();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
