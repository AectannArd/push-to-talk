use anyhow::{Context, Result};
use std::path::Path;

/// Thin wrapper around whisper-rs for push-to-talk transcription.
pub struct Transcriber {
    ctx: whisper_rs::WhisperContext,
    /// Language from config, or None for auto-detect.
    language: Option<String>,
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
                 Recommended: ggml-base.bin (~142 MB) or ggml-tiny.bin (~78 MB).",
                model_path.display()
            );
        }

        // Route whisper.cpp/ggml output through the `log` crate at debug level.
        // With log_level >= info, this output won't reach the console.
        whisper_rs::install_logging_hooks();

        let params = whisper_rs::WhisperContextParameters::default();

        let ctx =
            whisper_rs::WhisperContext::new_with_params(&model_path.to_string_lossy(), params)
                .context(
                    "Failed to load Whisper model — the file may be corrupted or unsupported.",
                )?;

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

        let mut inter_samples = vec![0.0f32; audio.len()];
        whisper_rs::convert_integer_to_float_audio(audio, &mut inter_samples)
            .context("Failed to convert i16 → f32 audio")?;

        let f32_min = inter_samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let f32_max = inter_samples
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        tracing::info!("🔍 f32 range: [{f32_min:.6}, {f32_max:.6}]");

        let mut state = self.ctx.create_state()?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(
            std::thread::available_parallelism()
                .map(|n| n.get() as i32)
                .unwrap_or(2),
        );
        params.set_translate(false);

        match self.language.as_deref() {
            Some("auto") | None => {
                params.set_language(Some("auto"));
            }
            Some(code) => {
                params.set_language(Some(code));
                tracing::info!("🌐 Language forced: {code}");
            }
        }

        // Suppress all whisper.cpp output to stderr
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let ret = state.full(params, &inter_samples[..])?;
        tracing::info!("🔍 whisper_full returned: {ret}");

        let n_segments = state.full_n_segments();
        let lang_id = state.full_lang_id_from_state();
        tracing::info!("🔍 Segments: {n_segments}, lang_id: {lang_id}");

        for i in 0..n_segments {
            if let Some(seg) = state.get_segment(i) {
                tracing::info!(
                    "🔍 seg[{i}]: \"{text}\" (no_speech_prob={nsp:.4})",
                    text = seg.to_string(),
                    nsp = seg.no_speech_probability(),
                );
            }
        }

        let text: String = state.as_iter().map(|seg| seg.to_string()).collect();

        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            tracing::warn!(
                "🔍 Empty transcription result — {n_segments} segments, \
                 {dur:.2}s of audio",
                n_segments = n_segments,
                dur = duration_s,
            );
        }

        Ok(trimmed)
    }
}
