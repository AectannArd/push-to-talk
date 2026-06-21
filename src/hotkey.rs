//! Global hotkey normalisation and shortcut event handling.
//!
//! Translates legacy key-name aliases (from the old rdev-based CLI) to
//! W3C-spec names expected by `tauri-plugin-global-shortcut`. Handles
//! press/release events from the global shortcut plugin.

use crate::state::{self, AppState};
use std::sync::atomic::Ordering;

/// Normalize key names from the old rdev-based CLI to keyboard_types::Code format.
/// The old CLI used aliases like "Ins", "Del", "Esc" while global-hotkey
/// expects W3C spec names like "Insert", "Delete", "Escape".
pub fn normalize_hotkey(raw: &str) -> String {
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
pub fn handle_shortcut_event(event: tauri_plugin_global_shortcut::ShortcutEvent) {
    use tauri_plugin_global_shortcut::ShortcutState;

    let Some(state) = state::get_global_state() else {
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

/// Toggle recording on/off (used by the UI recording button).
pub fn toggle_recording_inner(state: &AppState) {
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
