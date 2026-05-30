use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default config file: `~/.push-to-talk/config.toml`
pub fn default_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join(".push-to-talk").join("config.toml")
}

fn default_model_dirs() -> Vec<String> {
    // Default: ~/.push-to-talk/models/ (next to config.toml) + current dir
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    vec![
        home.join(".push-to-talk").join("models").to_string_lossy().to_string(),
    ]
}

fn default_hotkey() -> String {
    "Ctrl+Shift+T".into()
}

fn default_log_dir() -> String {
    "logs".into()
}

fn default_log_level() -> String {
    "error".into()
}

fn default_retention_hours() -> u64 {
    2
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Audio device filter: substring or numeric index. None = prompt on startup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,

    /// Whisper language: "auto", "ru", "en", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Specific model file path. If set and exists, skip directory scanning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Directories to scan for ggml-*.bin model files (used if `model` is not set).
    #[serde(default = "default_model_dirs")]
    pub model_search_dirs: Vec<String>,

    /// Hotkey in format "Mod+Mod+Key", e.g. "Ctrl+Shift+T".
    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    /// Directory for rolling log files.
    #[serde(default = "default_log_dir")]
    pub log_dir: String,

    /// Log level: "trace", "debug", "info", "warn", "error".
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// How many hours to retain rotated log files before cleanup.
    #[serde(default = "default_retention_hours")]
    pub log_retention_hours: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: None,
            language: None,
            model: None,
            model_search_dirs: default_model_dirs(),
            hotkey: default_hotkey(),
            log_dir: default_log_dir(),
            log_level: default_log_level(),
            log_retention_hours: default_retention_hours(),
        }
    }
}

impl Config {
    /// Load config from `path`. Returns defaults if file doesn't exist or can't be parsed.
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            tracing::info!("📄 No config at {path}, using defaults", path = path.display());
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => {
                    tracing::info!("📄 Loaded config: {path}", path = path.display());
                    cfg
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config: {e} — using defaults");
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read config: {e} — using defaults");
                Self::default()
            }
        }
    }

    /// Persist config to `path`. Creates parent directories if needed.
    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    tracing::error!("Failed to create config dir: {e}");
                    return;
                }
            }
        }
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(path, &content) {
                    tracing::error!("Failed to write config: {e}");
                } else {
                    tracing::info!("💾 Config saved: {path}", path = path.display());
                }
            }
            Err(e) => tracing::error!("Failed to serialise config: {e}"),
        }
    }
}
