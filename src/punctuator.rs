//! Punctuation and case restoration via ONNX Runtime.
//!
//! Runs `kontur-ai/sbert_punc_case_ru` — a BERT-large token classification model
//! (427M params) that predicts a joint case+punctuation label for each word.
//! Built on top of `ai-forever/sbert_large_nlu_ru`.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// 12 classification labels (matching the Python model's `id2label` mapping).
/// Each label encodes both the word case and the trailing punctuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PuncLabel {
    LowerO = 1,
    LowerPeriod = 2,
    LowerComma = 3,
    LowerQuestion = 4,
    UpperO = 5,
    UpperPeriod = 6,
    UpperComma = 7,
    UpperQuestion = 8,
    UpperTotalO = 9,
    UpperTotalPeriod = 10,
    UpperTotalComma = 11,
    UpperTotalQuestion = 12,
}

impl PuncLabel {
    fn from_id(id: i64) -> Self {
        match id {
            1 => PuncLabel::LowerO,
            2 => PuncLabel::LowerPeriod,
            3 => PuncLabel::LowerComma,
            4 => PuncLabel::LowerQuestion,
            5 => PuncLabel::UpperO,
            6 => PuncLabel::UpperPeriod,
            7 => PuncLabel::UpperComma,
            8 => PuncLabel::UpperQuestion,
            9 => PuncLabel::UpperTotalO,
            10 => PuncLabel::UpperTotalPeriod,
            11 => PuncLabel::UpperTotalComma,
            12 => PuncLabel::UpperTotalQuestion,
            _ => PuncLabel::LowerO, // fallback for label -100 (padding) or unknown
        }
    }
}

/// BERT token classification punctuator backed by ONNX Runtime.
///
/// `Session` and `Tokenizer` are both `Send + Sync`, so `Punctuator` can be
/// safely shared across threads via `Arc`.
pub struct Punctuator {
    session: ort::session::Session,
    tokenizer: tokenizers::Tokenizer,
    max_seq_len: usize,
}

impl Punctuator {
    /// Load the ONNX model and tokenizer from the given file paths.
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load tokenizer {}: {}",
                tokenizer_path.display(),
                e
            )
        })?;

        let session = ort::session::Session::builder()
            .map_err(|e| anyhow::anyhow!("Failed to create ONNX session builder: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow::anyhow!("Failed to set intra threads: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| {
                anyhow::anyhow!("Failed to load ONNX model {}: {}", model_path.display(), e)
            })?;

        tracing::info!(
            "✅ Punctuator loaded: model={}, tokenizer={}",
            model_path.display(),
            tokenizer_path.display()
        );

        Ok(Self {
            session,
            tokenizer,
            max_seq_len: 512,
        })
    }

    /// Create a Punctuator by auto-discovering model files from config.
    ///
    /// Searches `<model_search_dir>/punctuator/model.onnx` in each configured
    /// search directory. Falls back to `punctuation_model_path` if set in config.
    pub fn from_config(config: &crate::config::Config) -> Result<Self> {
        let model_path = if let Some(ref p) = config.punctuation_model_path {
            let path = PathBuf::from(shellexpand::tilde(p).as_ref());
            if path.exists() {
                path
            } else {
                anyhow::bail!(
                    "Punctuation model not found at configured path: {}",
                    path.display()
                );
            }
        } else {
            find_punctuation_model(&config.model_search_dirs).context(
                "No punctuation model found. Place model.onnx in <model_dir>/punctuator/",
            )?
        };

        let tokenizer_path = if let Some(ref p) = config.punctuation_tokenizer_path {
            let path = PathBuf::from(shellexpand::tilde(p).as_ref());
            if path.exists() {
                path
            } else {
                anyhow::bail!(
                    "Punctuation tokenizer not found at configured path: {}",
                    path.display()
                );
            }
        } else {
            // Look for tokenizer.json next to the model
            model_path
                .parent()
                .map(|p| p.join("tokenizer.json"))
                .filter(|p| p.exists())
                .context("No tokenizer.json found next to punctuation model.")?
        };

        Self::new(&model_path, &tokenizer_path)
    }

    /// Apply punctuation and case restoration to transcribed text.
    ///
    /// Returns the transformed text, or the original text unchanged if processing
    /// produces an empty result.
    pub fn punctuate(&mut self, text: &str) -> Result<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        // Split into words (same as Python: text.strip().lower().split())
        let lower = trimmed.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        if words.is_empty() {
            return Ok(String::new());
        }

        // Handle long texts by recursive splitting (mirrors Python approach)
        if self.count_tokens(&words) > self.max_seq_len {
            return self.punctuate_long(&words);
        }

        self.punctuate_batch(&words)
    }

    /// Count how many tokens the word list would produce (including [CLS]/[SEP]).
    /// Does not need `&mut self` — only tokenization, no inference.
    fn count_tokens(&self, words: &[&str]) -> usize {
        let owned: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        let input = tokenizers::EncodeInput::Single(tokenizers::InputSequence::PreTokenizedOwned(
            std::borrow::Cow::Owned(owned),
        ));
        match self.tokenizer.encode(input, true) {
            Ok(enc) => enc.len(),
            Err(_) => usize::MAX, // conservatively assume too long on error
        }
    }

    /// Recursively split a long word list at midpoint and punctuate each half.
    fn punctuate_long(&mut self, words: &[&str]) -> Result<String> {
        let mid = words.len() / 2;
        if mid == 0 {
            // Single very long word — return as-is (lowercased)
            return Ok(words.join(" "));
        }
        let left = self.punctuate(&words[..mid].join(" "))?;
        let right = self.punctuate(&words[mid..].join(" "))?;
        Ok(format!("{} {}", left, right))
    }

    /// Core inference pipeline for a batch that fits within the token limit.
    fn punctuate_batch(&mut self, words: &[&str]) -> Result<String> {
        // 1. Tokenize with word-level splitting (equivalent to is_split_into_words=True)
        let owned: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        let input = tokenizers::EncodeInput::Single(tokenizers::InputSequence::PreTokenizedOwned(
            std::borrow::Cow::Owned(owned),
        ));
        let encoding = self
            .tokenizer
            .encode(input, true) // add_special_tokens ([CLS] / [SEP])
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let word_ids: Vec<Option<u32>> = encoding.get_word_ids().to_vec();

        let seq_len = token_ids.len();

        // 2. Build ONNX input tensors [1, seq_len] using Vec-based API (avoids ndarray version issues)
        let shape = vec![1i64, seq_len as i64];
        let input_ids_tensor =
            ort::value::Tensor::from_array((shape.clone(), token_ids.into_boxed_slice()))
                .map_err(|e| anyhow::anyhow!("Failed to create input_ids tensor: {}", e))?;
        let attention_tensor =
            ort::value::Tensor::from_array((shape, attention_mask.into_boxed_slice()))
                .map_err(|e| anyhow::anyhow!("Failed to create attention_mask tensor: {}", e))?;

        // 3. Run ONNX inference
        let outputs = self
            .session
            .run(
                ort::inputs!["input_ids" => input_ids_tensor, "attention_mask" => attention_tensor],
            )
            .map_err(|e| anyhow::anyhow!("ONNX inference failed: {}", e))?;

        // 4. Extract logits: shape [1, seq_len, 13], data as &[f32]
        let (_shape, logits_slice) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("Failed to extract logits tensor: {}", e))?;

        let num_labels: usize = 13;

        // 5. Argmax per position → predicted label IDs
        let predictions: Vec<i64> = (0..seq_len)
            .map(|pos| {
                let start = pos * num_labels;
                let end = start + num_labels;
                if end > logits_slice.len() {
                    return 0i64; // safety: out of bounds → ignore label
                }
                let slice = &logits_slice[start..end];
                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0usize;
                for (i, &v) in slice.iter().enumerate() {
                    if v > max_val {
                        max_val = v;
                        max_idx = i;
                    }
                }
                max_idx as i64
            })
            .collect();

        // 6. Apply labels to words (first subword prediction only)
        let mut result =
            String::with_capacity(words.iter().map(|w| w.len()).sum::<usize>() + words.len() * 4);

        for (word_idx, &word) in words.iter().enumerate() {
            // Find the first token position belonging to this word
            let word_idx_u32 = word_idx as u32;
            let label_pos = word_ids
                .iter()
                .position(|opt: &Option<u32>| *opt == Some(word_idx_u32));

            let label = match label_pos {
                Some(pos) if pos < predictions.len() => PuncLabel::from_id(predictions[pos]),
                _ => PuncLabel::LowerO, // fallback: lowercased, no punctuation
            };

            let annotated = apply_label(word, label);

            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&annotated);
        }

        Ok(result)
    }
}

/// Apply a predicted label to a word: adjust case and append punctuation.
fn apply_label(word: &str, label: PuncLabel) -> String {
    // Apply case transformation
    let cased = match label {
        PuncLabel::UpperTotalO
        | PuncLabel::UpperTotalPeriod
        | PuncLabel::UpperTotalComma
        | PuncLabel::UpperTotalQuestion => word.to_uppercase(),

        PuncLabel::UpperO
        | PuncLabel::UpperPeriod
        | PuncLabel::UpperComma
        | PuncLabel::UpperQuestion => {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }

        // LowerO, LowerPeriod, LowerComma, LowerQuestion, or fallback
        _ => word.to_lowercase(),
    };

    // Append punctuation
    let punct = match label {
        PuncLabel::LowerPeriod | PuncLabel::UpperPeriod | PuncLabel::UpperTotalPeriod => ".",

        PuncLabel::LowerComma | PuncLabel::UpperComma | PuncLabel::UpperTotalComma => ",",

        PuncLabel::LowerQuestion | PuncLabel::UpperQuestion | PuncLabel::UpperTotalQuestion => "?",

        _ => "",
    };

    format!("{}{}", cased, punct)
}

/// Scan model search directories for a punctuation ONNX model.
///
/// Looks for `<dir>/punctuator/model.onnx` in each directory.
pub fn find_punctuation_model(dirs: &[String]) -> Option<PathBuf> {
    for dir in dirs {
        let expanded = shellexpand::tilde(dir);
        let base = Path::new(expanded.as_ref());
        let candidate = base.join("punctuator").join("model.onnx");
        if candidate.exists() {
            tracing::info!("🔍 Found punctuation model: {}", candidate.display());
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_from_id_all_classes() {
        assert_eq!(PuncLabel::from_id(1), PuncLabel::LowerO);
        assert_eq!(PuncLabel::from_id(5), PuncLabel::UpperO);
        assert_eq!(PuncLabel::from_id(9), PuncLabel::UpperTotalO);
        assert_eq!(PuncLabel::from_id(12), PuncLabel::UpperTotalQuestion);
    }

    #[test]
    fn test_label_from_id_fallback() {
        assert_eq!(PuncLabel::from_id(-100), PuncLabel::LowerO);
        assert_eq!(PuncLabel::from_id(99), PuncLabel::LowerO);
    }

    #[test]
    fn test_apply_label_lower_o() {
        assert_eq!(apply_label("привет", PuncLabel::LowerO), "привет");
    }

    #[test]
    fn test_apply_label_upper_period() {
        assert_eq!(apply_label("привет", PuncLabel::UpperPeriod), "Привет.");
    }

    #[test]
    fn test_apply_label_upper_total_question() {
        assert_eq!(
            apply_label("привет", PuncLabel::UpperTotalQuestion),
            "ПРИВЕТ?"
        );
    }

    #[test]
    fn test_apply_label_lower_comma() {
        assert_eq!(apply_label("мир", PuncLabel::LowerComma), "мир,");
    }

    #[test]
    fn test_apply_label_upper_o() {
        assert_eq!(apply_label("москва", PuncLabel::UpperO), "Москва");
    }

    #[test]
    fn test_apply_label_lower_question() {
        assert_eq!(apply_label("что", PuncLabel::LowerQuestion), "что?");
    }

    #[test]
    fn test_apply_label_upper_total_o() {
        assert_eq!(apply_label("ру", PuncLabel::UpperTotalO), "РУ");
    }
}
