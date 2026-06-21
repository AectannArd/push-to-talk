//! Global application state and DTOs shared across modules.
//!
//! `AppState` is stored in a `OnceLock<Arc<AppState>>` set before
//! `tauri::Builder`. All IPC commands access it via `get_global_state()`.
//! Lock ordering: always acquire `config` before `is_running` to avoid
//! deadlocks with `get_status`.

use crate::punctuator;
use crate::voice_service;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock};

/// Global state — set once before Tauri builder, read everywhere.
static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub fn get_global_state() -> Option<&'static Arc<AppState>> {
    APP_STATE.get()
}

/// Set the global state. Must be called exactly once before Tauri runs.
pub fn init_global_state(state: Arc<AppState>) {
    let _ = APP_STATE.set(state);
}

pub struct AppState {
    pub voice_service: Arc<Mutex<Option<voice_service::VoiceServiceHandle>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<crate::config::Config>>,
    pub is_recording: Arc<AtomicBool>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
    pub punctuator: Arc<Mutex<Option<punctuator::Punctuator>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            voice_service: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            config: Arc::new(Mutex::new(crate::config::Config::default())),
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

// ── DTOs (frontend IPC types) ────────────────────────────────────────

/// Device info for the frontend device dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDto {
    pub id: String,
    pub name: String,
    pub config: String,
    pub is_default: bool,
}

/// Model info for the frontend model list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDto {
    pub filename: String,
    pub path: String,
    pub size: String,
}

/// Service status polled by the frontend every 2 seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusDto {
    pub is_recording: bool,
    pub is_service_running: bool,
    pub hotkey: String,
    pub language: Option<String>,
    pub last_transcription: Option<String>,
}
