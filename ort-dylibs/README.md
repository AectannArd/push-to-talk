# ONNX Runtime native libraries for push-to-talk

This directory holds ONNX Runtime shared libraries (.dll / .dylib / .so)
so Tauri can bundle them into the final application.

## How it works

1. **build.rs** downloads the correct ONNX Runtime release for the target platform
   from [Microsoft ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)
   and caches them here.
2. **Tauri** copies the files into the app bundle via platform-specific configs
   (`tauri.windows.conf.json`, `tauri.macos.conf.json`, `tauri.linux.conf.json`).
3. **At runtime**, `main()` discovers the DLL next to the executable and sets
   `ORT_DYLIB_PATH` before any ONNX code runs.
4. **Graceful degradation**: if the DLL or model is missing, punctuation is
   silently disabled and raw text is used after transcription.

## Platform files

| Platform | Files |
|----------|-------|
| Windows  | `windows/onnxruntime.dll`, `windows/onnxruntime_providers_shared.dll` |
| macOS    | `macos/libonnxruntime.dylib` |
| Linux    | `linux/libonnxruntime.so` |

## Development (`cargo run`)

When running via `cargo run`, the DLL must be next to the debug binary:

```bash
cp ort-dylibs/windows/onnxruntime.dll target/debug/
cp ort-dylibs/windows/onnxruntime_providers_shared.dll target/debug/
```

Or set the environment variable:

```bash
export ORT_DYLIB_PATH=/path/to/ort-dylibs/windows/onnxruntime.dll
```

## Punctuation model

The ONNX model (`model.onnx` + `tokenizer.json`) is **not** stored here.
It is published as a separate HuggingFace repository:

```
https://huggingface.co/Aectann/punctuation-case-model
```

Users download it via the Push-to-Talk UI (toggle punctuation, confirm download)
or manually:

```bash
hf download Aectann/punctuation-case-model model.onnx tokenizer.json \
  --local-dir ~/.push-to-talk/models/punctuator
```

The model is automatically discovered by scanning `<model_search_dir>/punctuator/model.onnx`.
Original model: [kontur-ai/sbert_punc_case_ru](https://huggingface.co/kontur-ai/sbert_punc_case_ru)
built on [ai-forever/sbert_large_nlu_ru](https://huggingface.co/ai-forever/sbert_large_nlu_ru).
