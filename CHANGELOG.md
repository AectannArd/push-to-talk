# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Russian fine-tuned Whisper model (`ggml-large-v3-russian-f16.bin`) from `Pomni/whisper-large-v3-russian-ggml-allquants`
- Model catalog on backend (`get_downloadable_models` command) — each model stores its full HuggingFace URL, so branch/repo are per-model, not hardcoded
- whisper.cpp → `tracing` log bridge via FFI (`whisper_log_set`): diagnostics appear in log files at `debug`/`trace` level

### Changed
- **whisper-rs → whisper-cpp-plus** (0.15 → 0.1): `TranscriptionParams` → `FullParams` with `print_*` suppression flags
- `download_model` now accepts `model_id` (catalog ID) instead of arbitrary `model_name` + `repo` — unknown IDs are rejected
- i16→f32 audio normalization: `32768.0` divisor (canonical whisper.cpp convention) instead of `i16::MAX`
- Log file extension matches format: `.txt` for text, `.json` for JSON (previously always `.log`)

### Removed
- CoreML acceleration on macOS (whisper-cpp-plus uses Metal only)
- `log_backend` cargo feature (whisper-cpp-plus has no equivalent; diagnostics bridged via FFI)

### Fixed
- `log_format` config field now actually respected — previously ignored, file layer always used default format
- Model radio-button selection now re-renders immediately (snapshot optimisation reset on click)

## [0.1.0] - 2024-06-02

### Added
- Initial release
- Push-to-talk functionality with global hotkey
- Whisper.cpp integration for local transcription
- Auto-typing transcribed text
- Audio recording with cpal
- Configuration management with TOML config
- macOS, Linux, and Windows support
