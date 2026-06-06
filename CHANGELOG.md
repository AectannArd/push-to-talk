# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Interactive device selection with system IDs (stable across reboots)
- Colored CLI wizard using `console` and `indicatif` libraries
- Progress bars for model download and initialization
- Hotkey hold detection to prevent repeat triggers
- HuggingFace model download from config UI

### Changed
- Config now stores `device_id` (system ID) instead of just device name
- UI migrated to Tauri WebView frontend (`ui/`)
- Improved error messages and user feedback

### Fixed
- Cyclic key repeat on hotkey hold
- Device selection persistence across reboots

## [0.1.0] - 2024-06-02

### Added
- Initial release
- Push-to-talk functionality with global hotkey
- Whisper.cpp integration for local transcription
- Auto-typing transcribed text
- Audio recording with cpal
- Configuration management with TOML config
- macOS, Linux, and Windows support
