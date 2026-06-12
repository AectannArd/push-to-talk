# Push-to-Talk 🎤

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](#requirements)

Desktop push-to-talk voice input. Press a global hotkey, speak, release —
text is transcribed locally via Whisper and pasted into the active window.
Built with **Rust**, **Tauri v2**, and **Whisper.cpp**.

## Quick start

### Windows

1. Download a Whisper model (or use the in-app downloader)
   ```powershell
   mkdir -p $env:USERPROFILE\.push-to-talk\models
   curl -Lo $env:USERPROFILE\.push-to-talk\models\ggml-base.bin `
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
   ```

2. Build (requires CUDA Toolkit 12+ + `CUDA_PATH`)
   ```powershell
   cargo build --release
   ```

3. Run
   ```powershell
   .\target\release\push-to-talk.exe
   ```

### Installer (Windows)

```powershell
cargo tauri build
```

Produces two installers:
- **NSIS** — `target\release\bundle\nsis\Push-to-Talk_0.1.0_x64-setup.exe`
- **MSI** — `target\release\bundle\msi\Push-to-Talk_0.1.0_x64_en-US.msi`

NSIS and WiX are downloaded automatically by Tauri — no manual installation needed.
The installer configures Start Menu shortcuts and registers an uninstaller.

### macOS (Apple Silicon / M-series)

```bash
# 1. Download a Whisper model
mkdir -p ~/.push-to-talk/models
curl -Lo ~/.push-to-talk/models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# 2. Build (Metal GPU acceleration, no extra toolchain)
cargo build --release

# 3. Grant Accessibility permission (required for auto-paste)
#    System Preferences → Privacy & Security → Accessibility
#    Add push-to-talk.app to the list

# 4. Run
./target/release/push-to-talk
```

## Usage

### Global hotkey (push-to-talk anywhere)

Press the configured hotkey (default: `Insert`), speak into your microphone,
release — Whisper transcribes locally and the text is **automatically pasted**
into the currently focused application.

On Windows, `Ctrl+V` is simulated via `keybd_event`. On macOS, `Cmd+V` is
sent via AppleScript.

### UI recording button

Click **🎤 Start Recording** in the configuration window, speak, click **⏹ Stop**.
The transcription appears in the text field next to the button — no clipboard
interaction, no auto-paste.

### System tray

Close the window → app minimizes to tray. Right-click the tray icon:

- **Configure** — show the configuration window
- **Quit** — exit the application

The app stays resident in the tray, listening for the global hotkey.

## Configuration

All settings are managed through the GUI. Config file:
`~/.push-to-talk/config.toml` (TOML).

```toml
device_id = "wasapi:{...}"           # audio input device system ID
device_name = "Microphone Array"     # human-readable device name
language = "auto"                    # "auto", "ru", "en", ...
model = "D:\\models\\ggml-medium.bin" # explicit model path
model_search_dirs = ["D:\\development\\models"]
hotkey = "Insert"                    # global shortcut
log_dir = "C:\\Users\\...\\logs"
log_level = "warn"                   # trace, debug, info, warn, error
log_format = "text"                  # text (.txt) or json (.json) — file extension matches format
log_retention_hours = 1
window_hidden = false                # start minimized to tray
```

### Configuration panels

| Section | Contents |
|---------|----------|
| 🎛️ Common | Recording button + transcription, available models (clickable radio buttons), audio input device |
| 🎤 Audio & Transcription | Hotkey, language |
| 🧠 Whisper Model | Model search directories, download model dropdown |
| 📝 Logging | Log directory, level, format, retention |

### Model management

- **Auto-scanning** — model directories are scanned every 5 seconds in the background
- **Selection** — click a model in the list (○/● radio) to use it for transcription
- **Download** — choose a model from the dropdown and click ⬇️ Download; the catalog of available models is maintained on the backend (`DownloadableModel` entries with full HuggingFace URLs), so each model can come from any repo/branch. Already-downloaded models are hidden from the list
- **Resolution** — `model` field in config → scan `model_search_dirs` → first match wins

### Language switching

Changing the language in the UI takes effect **immediately** — on the very next
transcription, without restarting the app.

## Architecture

```
src/
├── main.rs           Tauri entry point, IPC commands, global state, tray, logging
├── config.rs         TOML config at ~/.push-to-talk/config.toml
├── recorder.rs       Audio capture via cpal (i16 on Windows, f32 on macOS/Linux)
├── transcriber.rs    Whisper.cpp wrapper (whisper-cpp-plus), log bridge, greedy decoding
├── punctuator.rs     ONNX Runtime BERT model for punctuation/case restoration
└── voice_service.rs  Background orchestrator: recorder + transcriber + clipboard
ui/                   React + TypeScript frontend (Vite + Bootstrap Morph dark theme)
├── src/components/   UI components (StatusBar, ConfigForm, ModelSelector, etc.)
├── src/hooks/        React hooks (useConfig, useStatus, useModels, useDevices)
├── src/services/     Tauri IPC wrapper
├── src/i18n/         49-language translations (EN, RU, DE, FR, ES, ...)
└── public/           Static assets (favicon, download icon)
```

## Platform support

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| GPU acceleration | CUDA | Metal | CPU only |
| Audio format | i16 (WASAPI) | f32 (CoreAudio) | f32 (ALSA/Pulse) |
| Auto-paste | `keybd_event` Ctrl+V | AppleScript Cmd+V | Manual |
| System tray | ✓ | ✓ | ✓ |
| Installer | NSIS + MSI | DMG | — |

## Device resilience

If the configured audio device disconnects while recording:
- Recording is **force-stopped immediately**
- The app **switches to the first available device** automatically
- The new device is persisted to config
- The user must press the hotkey again to start a new recording

## Logging

- **Files:** `log_dir/push-to-talk.YYYY-MM-DD-HH-mm.{txt,json}` with 1-minute rotation
- **Format:** controlled by `log_format` in config — `"text"` produces `.txt` files (human-readable), `"json"` produces `.json` files (structured)
- **Diagnostics:** whisper.cpp internal logging is bridged to `tracing` — at `debug` level, model diagnostics appear in log files
- **Retention:** files older than `log_retention_hours` are cleaned up hourly
- **Levels:** default `warn` — shows hotkey registration and diagnostics in log files
- **Console:** suppressed on Windows (`#![windows_subsystem = "windows"]`)

## Requirements

| Component | Windows | macOS |
|-----------|---------|-------|
| Rust | 1.85+ | 1.85+ |
| C/C++ compiler | MSVC + CMake | Xcode CLT + CMake |
| GPU | CUDA Toolkit 12+ | Metal (built-in) |
| Global hotkey | — | Accessibility permission |
| Microphone | ✓ | ✓ |

To build without GPU acceleration, remove the platform GPU features from
`whisper-cpp-plus` in `Cargo.toml` (`cuda` on Windows, `metal` on macOS).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache License 2.0 — see [LICENSE](LICENSE).
