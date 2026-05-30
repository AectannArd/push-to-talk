# push-to-talk 🎙

Push-to-talk voice input for the CLI. Hold a hotkey, speak, release — text is
typed directly into the active window.

> Rust, Whisper.cpp (CUDA), cpal, rdev, enigo.

## Quick start

```powershell
# 1. Download a Whisper model to ~/.push-to-talk/models/
mkdir -p ~/.push-to-talk/models
curl -Lo ~/.push-to-talk/models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# 2. Build (requires CUDA Toolkit + CUDA_PATH)
cargo build --release

# 3. Run — interactive setup on first launch
.\target\release\push-to-talk.exe
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

- **Rust** 1.85+
- **CMake** + MSVC C++ compiler (whisper.cpp build)
- **CUDA Toolkit** + `CUDA_PATH` env var (GPU acceleration)
- **Microphone**

To build without CUDA, remove the `cuda` feature from `whisper-rs` in `Cargo.toml`.

## Project structure

```
src/
├── main.rs           CLI args, config orchestration, hotkey loop, transcription dispatch
├── config.rs         Config struct, load/save, default path
├── hotkey.rs         Hotkey string parser ("Ctrl+Shift+T" → keys + modifiers)
├── indicator.rs      Console recording indicator (terminal title + inline marker)
├── recorder.rs       Audio capture (cpal), device enumeration, format conversion
├── transcriber.rs    Whisper transcription (whisper-rs / CUDA / log_backend)
└── tray.rs           System tray icon with recording state tooltip (Windows)
```

## License

Unlicense — do whatever you want.
