# push-to-talk 🎙

Push-to-talk voice input for the CLI. Hold a hotkey, speak, release — text lands in your clipboard, ready to paste and send.

> Built with Rust, Whisper.cpp (CUDA-accelerated), and cpal.

## Quick start

```powershell
# 1. Build with CUDA (requires CUDA Toolkit + CUDA_PATH set)
cargo build --release

# 2. Run — model is auto-discovered from D:\development\models\
.\target\release\push-to-talk.exe
```

## Usage

```
╔══════════════════════════════════════════╗
║   🎙  Push-to-Talk CLI                  ║
║   Hold Ctrl+Shift+T, speak, release.    ║
║   Text → clipboard → Ctrl+V → Enter     ║
╚══════════════════════════════════════════╝
```

1. Start the binary
2. Hold **Ctrl+Shift+T**
3. Speak into your microphone
4. Release — text gets transcribed and copied to clipboard
5. Press **Ctrl+V** in your CLI/chat to paste
6. Verify the transcription, hit **Enter**

## Requirements

- **Rust** 1.85+
- **CMake** + MSVC C++ compiler (for building whisper.cpp)
- **CUDA Toolkit** (for GPU acceleration)
- **Microphone**

## Model auto-discovery

Resolution order:
1. `WHISPER_MODEL` environment variable (exact path)
2. `D:\development\models\` — scanned recursively for `ggml-*.bin`
3. Current directory — scanned for `ggml-*.bin`

```powershell
# Override model path:
$env:WHISPER_MODEL = "D:\models\ggml-medium.bin"
```

## GPU

CUDA is enabled by default via the `whisper-rs/cuda` feature.  
Built and tested with:
- CUDA Toolkit 13.0
- NVIDIA GeForce RTX 3050 Ti Laptop GPU (compute capability 8.6)
- Whisper large v3 model (~3 GB VRAM)

To build without CUDA, remove the `cuda` feature from `whisper-rs` in `Cargo.toml`.

## Architecture

```
Ctrl+Shift+T held         →  cpal captures mic at 16 kHz mono
Ctrl+Shift+T released     →  whisper.cpp (CUDA) transcribes → clipboard
```

- `src/main.rs` — global hotkey loop (`rdev`), clipboard (`arboard`), model discovery
- `src/recorder.rs` — microphone capture (`cpal`)
- `src/transcriber.rs` — Whisper transcription (`whisper-rs` / CUDA)
