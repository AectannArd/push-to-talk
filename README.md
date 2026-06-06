# push-to-talk 🎙

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](#requirements)
[![Release](https://img.shields.io/github/v/release/AectannArd/push-to-talk?logo=github)](https://github.com/AectannArd/push-to-talk/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/AectannArd/push-to-talk/release.yml?logo=github&label=build)](https://github.com/AectannArd/push-to-talk/actions)

Push-to-talk voice input for the CLI. Hold a hotkey, speak, release — text is
typed directly into the active window.

> Rust, Whisper.cpp (CUDA), cpal, rdev, enigo.

## Quick start

### Windows

```powershell
# 1. Download a Whisper model
mkdir -p ~/.push-to-talk/models
curl -Lo ~/.push-to-talk/models/ggml-base.bin `
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# 2. Build (requires CUDA Toolkit + CUDA_PATH)
cargo build --release

# 3. Run
.\target\release\push-to-talk.exe
```

### macOS (Apple Silicon / M-series)

```bash
# 1. Download a Whisper model
mkdir -p ~/.push-to-talk/models
curl -Lo ~/.push-to-talk/models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# 2. Build (Metal + CoreML acceleration, no extra toolchain needed)
cargo build --release

# 3. Grant Accessibility permission (required for paste functionality)
#    System Preferences → Privacy & Security → Accessibility
#    Add the push-to-talk binary
#    
#    ⚠️ Important: After each rebuild or reinstallation, macOS resets
#    Accessibility permissions. You must re-add the new binary to the list.

# 4. Run
./target/release/push-to-talk
```

## CLI flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Override config file location (default: `~/.push-to-talk/config.toml`) |
| `--non-interactive` | Skip interactive setup; exit with error if model is missing |
| `--debug-voice-record` | Save each recording as a timestamped WAV for debugging |

## Usage

```
╔══════════════════════════════════════════╗
║   🎙  Push-to-Talk CLI                  ║
║   Hold hotkey, speak, release.          ║
║   Text → auto-type → verify → Enter     ║
╚══════════════════════════════════════════╝
```

1. Start the binary — config review appears (skip with Enter)
2. Hold the configured hotkey (default: `Ctrl+Shift+T`)
3. Speak into your microphone
4. Release — Whisper transcribes, text is typed into the active window
5. Verify and hit **Enter**

**Indicators:** console prints `🔴 ● RECORDING ● 🔴` + terminal title changes;
system tray icon tooltip shows recording state.

## Configuration

Config file: `~/.push-to-talk/config.toml` (TOML format).
All fields are optional — defaults are used if absent.

```toml
device = "2"                  # audio input device (1-based index or substring)
language = "auto"             # Whisper language: auto, ru, en, ...
model = "/path/to/ggml.bin"   # explicit model path (skips directory scan)
model_search_dirs = [         # dirs scanned for ggml-*.bin if model is not set
    "~/.push-to-talk/models",
]
hotkey = "Ctrl+Shift+T"       # Mod+Mod+Key format (Ctrl, Shift, Alt, Win)
log_dir = "logs"              # log directory (env vars expanded: %APPDATA%, $HOME)
log_level = "error"           # trace, debug, info, warn, error
log_format = "text"           # text or json
log_retention_hours = 2       # rotated log files older than this are deleted
```

### Interactive setup

On each launch (unless `--non-interactive`), current config is displayed and you
can edit any field:

```
┌─ Current config ───────────────────────────────────
│ device:             2
│ language:           auto
│ model:              D:\models\ggml-large-v3.bin
│ hotkey:             Ctrl+Shift+T
│ model_search_dirs:  ["~/.push-to-talk/models"]
│ log_dir:            %APPDATA%\push-to-talk\logs
│ log_level:          error
│ log_format:         text
│ log_retention:      2h
└─────────────────────────────────────────────────────

✏  Edit config? [y/N]:
```

If the configured model file is missing, setup enters **force mode**:
1. First asks for model search directories
2. Scans those directories
3. Shows available models, lets you pick one
4. Then offers to review remaining settings

### Model discovery

Resolution order:
1. `model` field in config (exact path)
2. `WHISPER_MODEL` env var (exact path)
3. Scan `model_search_dirs` recursively for `ggml-*.bin` (first match wins)

On first run, the discovered model path is saved to config automatically.

## Logging

- **Console:** colorised output to stderr at the configured level
- **Files:** `log_dir/push-to-talk.YYYY-MM-DD-HH-mm.{log,json}` with 1‑minute rotation
- **Retention:** files older than `log_retention_hours` are deleted (background cleanup every 10 min)
- **Env vars in paths:** `%VAR%` (Windows) and `$VAR` / `${VAR}` (Unix) are expanded
- **Whisper output:** routed through `log::debug!` — visible only at `debug` level

## Requirements

| Component | Windows | macOS |
|-----------|---------|-------|
| Rust | 1.85+ | 1.85+ |
| C/C++ compiler | MSVC + CMake | Xcode CLT + CMake |
| GPU | CUDA Toolkit 12+ | Metal (built-in) |
| Global hotkey | Admin rights | Accessibility permission |
| Microphone | ✓ | ✓ |

To build without GPU acceleration, remove the platform GPU features from
`whisper-rs` in `Cargo.toml` (`cuda` on Windows, `metal`/`coreml` on macOS).

## Project structure

```
src/
├── main.rs           Entry point, Tauri commands, global state, logging
├── config.rs         Config struct, load/save, default path
├── recorder.rs       Audio capture (cpal), device enumeration, format conversion
├── transcriber.rs    Whisper transcription (whisper-rs, platform GPU backends)
└── voice_service.rs  Background service orchestrator, clipboard, transcription loop
```

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:
- Development setup
- Code style and testing
- Pull request process
- Reporting issues

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
