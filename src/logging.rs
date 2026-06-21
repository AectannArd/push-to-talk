//! Logging initialisation, rotation, and retention cleanup.
//!
//! Sets up a `tracing-subscriber` registry with:
//! - Console layer (always human-readable, INFO+)
//! - File layer (minutely rotation, text or JSON depending on config)
//!
//! ONNX Runtime BFC arena / session diagnostics are suppressed to `warn`
//! unless explicitly requested via `RUST_LOG`.

use crate::config;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise the tracing subscriber with console + file layers.
pub fn init(config: &config::Config) {
    let log_dir = Path::new(&config.log_dir);
    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory: {}", e);
    }

    // Choose file extension and format variant based on config
    let (file_suffix, is_json) = match config.log_format.as_str() {
        "json" => ("json", true),
        _ => ("log", false), // default: human-readable text
    };

    // Create minutely rolling file appender
    // Filename format: push-to-talk.YYYY-MM-DD-HH-MM.{log,json}
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::MINUTELY)
        .filename_prefix("push-to-talk")
        .filename_suffix(file_suffix)
        .build(log_dir)
        .expect("Failed to create file appender");

    // Default filter: app at configured level, but suppress ONNX Runtime's
    // extremely verbose BFC arena / session diagnostics unless explicitly
    // requested via RUST_LOG=ort=debug or similar.
    let default_filter = format!("{},ort=warn,onnxruntime=warn", config.log_level);
    let file_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&default_filter));

    // Console layer — always human-readable text, INFO level or higher
    let console_default = "info,ort=warn,onnxruntime=warn";
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(console_default));

    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(console_filter);

    // File layer — text or JSON depending on config, no ANSI escape codes
    let file_layer = {
        let layer = fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)
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
    start_cleanup_thread(log_dir.to_path_buf());
}

// ── Log retention ────────────────────────────────────────────────────

fn start_cleanup_thread(log_dir: PathBuf) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
        // Re-read retention from config so changes take effect without restart.
        let cfg = config::Config::load(&config::default_path());
        cleanup_old_logs(&log_dir, cfg.log_retention_hours);
    });
}

/// Remove log files older than `retention_hours` from `log_dir`.
pub fn cleanup_old_logs(log_dir: &Path, retention_hours: u64) {
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
