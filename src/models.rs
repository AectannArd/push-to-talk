//! Whisper and punctuation model management: catalog, discovery, download.
//!
//! The static catalog (`MODEL_CATALOG`) defines all downloadable Whisper models.
//! Punctuation model URLs are also defined here. File-system discovery scans
//! configured directories for `ggml-*.bin` (Whisper) and `punctuator/model.onnx`
//! (punctuation) files.

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ── Whisper model catalog & download URLs ────────────────────────────

/// Entry in the downloadable-model catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadableModel {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub url: String,
}

/// Static catalog of all models available for download.
pub static MODEL_CATALOG: std::sync::LazyLock<Vec<DownloadableModel>> =
    std::sync::LazyLock::new(|| {
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
    });

// ── Punctuation model ────────────────────────────────────────────────

/// HuggingFace URLs for the punctuation restoration model.
pub const PUNCTUATION_MODEL_URL: &str =
    "https://huggingface.co/Aectann/punctuation-case-model/resolve/main/model.onnx";
pub const PUNCTUATION_TOKENIZER_URL: &str =
    "https://huggingface.co/Aectann/punctuation-case-model/resolve/main/tokenizer.json";

/// Result of checking whether the punctuation model is present locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PunctuationModelStatus {
    pub found: bool,
    pub model_path: Option<String>,
    pub onnx_url: String,
    pub tokenizer_url: String,
}

// ── File-system discovery ────────────────────────────────────────────

/// Enumerate all `ggml-*.bin` model files found in the given directories.
pub fn list_ggml_models(dirs: &[String]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for dir in dirs {
        let expanded = shellexpand::tilde(dir);
        let base = Path::new(expanded.as_ref());
        // Primary: <dir>/transcriber/
        let transcriber_dir = base.join("transcriber");
        scan_ggml_dir(&transcriber_dir, &mut paths, &mut seen);
        // Backward compat: <dir>/
        scan_ggml_dir(base, &mut paths, &mut seen);
    }
    paths
}

fn scan_ggml_dir(dir: &Path, paths: &mut Vec<PathBuf>, seen: &mut HashSet<String>) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with("ggml-") && name_str.ends_with(".bin") && seen.insert(name_str) {
                paths.push(entry.path());
            }
        }
    }
}

/// Find a usable Whisper model from config.
///
/// Resolution order:
/// 1. `config.model` — explicit path, must exist
/// 2. Scan `config.model_search_dirs` — first `ggml-*.bin` found wins
pub fn find_whisper_model(config: &Config) -> Option<PathBuf> {
    if let Some(ref model) = config.model {
        if Path::new(model).exists() {
            return Some(PathBuf::from(model));
        }
    }
    list_ggml_models(&config.model_search_dirs)
        .into_iter()
        .next()
}

// ── Utilities ────────────────────────────────────────────────────────

/// Format a byte count as a human-readable string.
pub fn format_size(bytes: u64) -> String {
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
