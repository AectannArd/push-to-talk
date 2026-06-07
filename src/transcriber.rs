use anyhow::{Context, Result};
use std::ffi::CStr;
use std::path::Path;
use std::sync::Once;
use whisper_cpp_plus::whisper_cpp_plus_sys::ggml_log_level;

/// Thin wrapper around whisper-cpp-plus for push-to-talk transcription.
pub struct Transcriber {
    ctx: whisper_cpp_plus::WhisperContext,
    /// Language from config, or None for auto-detect.
    language: Option<String>,
}

/// Install whisper.cpp → tracing log bridge once per process.
///
/// Routes whisper.cpp/ggml diagnostics through `tracing` so they appear in log
/// files at the appropriate levels. Without this, whisper.cpp prints everything
/// directly to stderr, bypassing the application's logging infrastructure.
fn install_whisper_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        whisper_cpp_plus::whisper_cpp_plus_sys::whisper_log_set(
            Some(whisper_log_bridge),
            std::ptr::null_mut(),
        );
        tracing::debug!("🔧 whisper.cpp log bridge installed");
    });
}

/// FFI callback: receives whisper.cpp/ggml log lines and forwards them to tracing.
unsafe extern "C" fn whisper_log_bridge(
    level: ggml_log_level,
    text: *const std::ffi::c_char,
    _user_data: *mut std::ffi::c_void,
) {
    if text.is_null() {
        return;
    }
    let msg = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let trimmed = msg.trim_end();
    if trimmed.is_empty() {
        return;
    }
    match level {
        4 => tracing::error!("🧠 whisper: {trimmed}"), // GGML_LOG_LEVEL_ERROR
        3 => tracing::warn!("🧠 whisper: {trimmed}"),  // GGML_LOG_LEVEL_WARN
        1 | 2 => tracing::debug!("🧠 whisper: {trimmed}"), // DEBUG | INFO
        _ => tracing::trace!("🧠 whisper: {trimmed}"),
    }
}

impl Transcriber {
    /// Load a Whisper model from `model_path` (e.g. `ggml-base.bin`).
    ///
    /// `language` is an optional ISO 639-1 code from config (None = auto-detect).
    pub fn new(model_path: &Path, language: Option<String>) -> Result<Self> {
        if !model_path.exists() {
            anyhow::bail!(
                "Whisper model not found at `{}`.\n\
                 Download a ggml model from https://huggingface.co/ggerganov/whisper.cpp\n\
                 Recommended: ggml-base.bin (~142\u{202f}MB) or ggml-tiny.bin (~78\u{202f}MB).",
                model_path.display()
            );
        }

        // Install once: whisper.cpp diagnostics → tracing log files
        install_whisper_logging();

        let ctx = whisper_cpp_plus::WhisperContext::new(model_path)
            .context("Failed to load Whisper model — the file may be corrupted or unsupported.")?;

        tracing::info!("✅ Whisper model loaded: {}", model_path.display());
        Ok(Self { ctx, language })
    }

    /// Update the language hint without reloading the model.
    /// Takes effect on the very next `transcribe()` call.
    pub fn set_language(&mut self, language: Option<String>) {
        self.language = language;
    }

    /// Transcribe 16 kHz mono PCM-i16 samples into text.
    pub fn transcribe(&self, audio: &[i16]) -> Result<String> {
        let duration_s = audio.len() as f64 / 16_000.0;
        tracing::info!(
            "🔍 Transcribing {n} samples ({dur:.2}s)",
            n = audio.len(),
            dur = duration_s,
        );

        let peak = audio.iter().map(|&s| s.abs()).max().unwrap_or(0);
        if peak < 50 {
            tracing::warn!(
                "🔍 Audio peak is only {peak} (out of {max}) — \
                 audio may be too quiet or empty",
                max = i16::MAX,
            );
        }

        // Convert i16 → f32 (whisper-cpp-plus expects 16kHz mono f32)
        let f32_samples: Vec<f32> = audio.iter().map(|&s| s as f32 / 32768.0f32).collect();

        let f32_min = f32_samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let f32_max = f32_samples
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        tracing::info!("🔍 f32 range: [{f32_min:.6}, {f32_max:.6}]");

        // Use FullParams directly to access print_* suppression flags.
        // TranscriptionParamsBuilder does not expose them.
        let mut params =
            whisper_cpp_plus::FullParams::new(whisper_cpp_plus::SamplingStrategy::Greedy {
                best_of: 1,
            });
        params = params
            .n_threads(
                std::thread::available_parallelism()
                    .map(|n| n.get() as i32)
                    .unwrap_or(2),
            )
            .print_progress(false)
            .print_timestamps(false)
            .print_special(false)
            .print_realtime(false);

        params = match self.language.as_deref() {
            Some("auto") | None => params.language("auto"),
            Some(code) => {
                tracing::info!("🌐 Language forced: {code}");
                params.language(code)
            }
        };

        let result = self
            .ctx
            .transcribe_with_full_params(&f32_samples, params)
            .context("whisper_full failed")?;

        let text: String = result.segments.iter().map(|s| s.text.as_str()).collect();
        let trimmed = text.trim().to_string();

        if trimmed.is_empty() {
            let n_segments = result.segments.len();
            tracing::warn!(
                "🔍 Empty transcription result — {n_segments} segments, \
                 {dur:.2}s of audio",
                dur = duration_s,
            );
        }

        Ok(trimmed)
    }
}
