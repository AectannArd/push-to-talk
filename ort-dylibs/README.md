Place ONNX Runtime native libraries here for Tauri bundling.

Download the appropriate release for your target platform from:
  https://github.com/microsoft/onnxruntime/releases

## Platform-specific files needed:

### Windows (x64)
  ort-dylibs/windows/
    onnxruntime.dll                  (~20 MB)
    onnxruntime_providers_shared.dll (~1 MB)

  Download: onnxruntime-win-x64-{version}.zip
  Extract lib/onnxruntime.dll and lib/onnxruntime_providers_shared.dll

### macOS (Universal2 — x64 + ARM64)
  ort-dylibs/macos/
    libonnxruntime.dylib             (~40 MB universal)

  Download: onnxruntime-osx-universal2-{version}.tgz
  Extract lib/libonnxruntime.{version}.dylib → rename to libonnxruntime.dylib
  (or copy the symlink target)

### Linux (x64)
  ort-dylibs/linux/
    libonnxruntime.so                (~20 MB)

  Download: onnxruntime-linux-x64-{version}.tgz
  Extract lib/libonnxruntime.so.{version} → rename to libonnxruntime.so
  (or copy the symlink target)

## How it works:

1. Tauri bundles the platform-specific library via tauri.{platform}.conf.json
2. At runtime, main() discovers the library next to the exe (or in Contents/Resources/ on macOS)
3. ORT_DYLIB_PATH env var is set → ort crate loads it via dlopen/LoadLibrary
4. If the DLL/dylib is missing or invalid, punctuation gracefully degrades (raw text used)

## Development (cargo run):

Place the libraries in the appropriate subdirectory, then set ORT_DYLIB_PATH:
  Windows: $env:ORT_DYLIB_PATH = "D:\path\to\ort-dylibs\windows\onnxruntime.dll"
  macOS:   export ORT_DYLIB_PATH = "/path/to/ort-dylibs/macos/libonnxruntime.dylib"
  Linux:   export ORT_DYLIB_PATH = "/path/to/ort-dylibs/linux/libonnxruntime.so"

Or rely on auto-discovery: copy the library next to target/debug/push-to-talk.exe
