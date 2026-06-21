//! Push-to-Talk Tauri Application — entry point.
//!
//! Sets up logging, global state, ONNX Runtime discovery, Tauri plugins,
//! system tray, window event handlers, and the global hotkey.

#![windows_subsystem = "windows"]

mod commands;
mod config;
mod hotkey;
mod logging;
mod models;
mod punctuator;
mod recorder;
mod state;
mod transcriber;
mod voice_service;

use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

fn main() {
    // Install panic hook to log panics before exiting
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("🚨 PANIC: {}", panic_info);
        eprintln!("🚨 PANIC: {}", panic_info);
    }));

    // Discover ONNX Runtime native library for punctuation restoration.
    // Priority: ORT_DYLIB_PATH env → next to binary (production bundle) →
    // target/ort-dylibs (development).
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        // Platform-specific library name
        #[cfg(target_os = "windows")]
        let lib_name = "onnxruntime.dll";
        #[cfg(target_os = "macos")]
        let lib_name = "libonnxruntime.dylib";

        let mut candidates: Vec<PathBuf> = Vec::new();

        // 1. Next to the executable (production / Tauri bundle)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(exe_dir.join(lib_name));
                // macOS: also check Contents/Resources/ one level up
                #[cfg(target_os = "macos")]
                if let Some(res_dir) = exe_dir.parent().map(|p| p.join("Resources")) {
                    candidates.push(res_dir.join(lib_name));
                }
            }
        }

        // 2. Default build-script output (development)
        #[cfg(target_os = "windows")]
        let platform_subdir = "windows";
        #[cfg(target_os = "macos")]
        let platform_subdir = "macos";
        candidates.push(
            PathBuf::from("target")
                .join("ort-dylibs")
                .join(platform_subdir)
                .join(lib_name),
        );

        // 3. ONNX_RT_OUTPUT override (same env var build.rs uses)
        if let Ok(root) = std::env::var("ONNX_RT_OUTPUT") {
            candidates.push(
                PathBuf::from(&root)
                    .join(platform_subdir)
                    .join(lib_name),
            );
        }

        for lib_path in &candidates {
            if lib_path.exists() {
                std::env::set_var("ORT_DYLIB_PATH", lib_path);
                break;
            }
        }
    }

    let config_path = config::default_path();
    let cfg = config::Config::load(&config_path);

    // Initialize logging first
    logging::init(&cfg);

    let app_state = AppState::new();
    *app_state.config.lock().unwrap() = cfg;
    let app_state_arc = Arc::new(app_state);

    // Initialize global state BEFORE tauri::Builder
    state::init_global_state(app_state_arc.clone());

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
            commands::get_status,
            commands::start_service,
            commands::stop_service,
            commands::get_config,
            commands::save_config,
            commands::trigger_recording,
            commands::hide_window,
            commands::list_audio_devices,
            commands::get_current_device,
            commands::scan_models,
            commands::get_downloadable_models,
            commands::download_model,
            commands::check_punctuation_model,
            commands::download_punctuation_model,
            commands::frontend_log,
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
            {
                let config = app_state_arc.config.lock().unwrap();
                let hotkey = config.hotkey.clone();
                drop(config);

                if !hotkey.is_empty() {
                    use tauri_plugin_global_shortcut::{
                        GlobalShortcutExt, Shortcut, ShortcutEvent,
                    };
                    let normalized = hotkey::normalize_hotkey(&hotkey);
                    if let Ok(shortcut) = normalized.parse::<Shortcut>() {
                        let shortcut_handler = move |_app: &tauri::AppHandle,
                                                     _id: &Shortcut,
                                                     event: ShortcutEvent| {
                            hotkey::handle_shortcut_event(event);
                        };
                        app.global_shortcut()
                            .on_shortcut(shortcut, shortcut_handler)
                            .unwrap_or_else(|e| {
                                tracing::error!(
                                    "❌ Failed to register hotkey '{}': {}",
                                    normalized,
                                    e
                                )
                            });
                        tracing::warn!("🎹 Global hotkey registered: {}", normalized);
                    } else {
                        tracing::error!("❌ Invalid hotkey format: {}", normalized);
                    }
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
            let _ = commands::start_service();

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
