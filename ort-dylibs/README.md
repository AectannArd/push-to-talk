# ONNX Runtime native libraries for push-to-talk

The ONNX Runtime shared libraries are downloaded automatically by `build.rs`
into `target/ort-dylibs/{platform}/` during the first build.

## Location

Default: `target/ort-dylibs/`

Override with the `ONNX_RT_OUTPUT` environment variable:

```bash
ONNX_RT_OUTPUT=/custom/path cargo build
```

## How it works

1. **build.rs** downloads the correct ONNX Runtime release for the target platform
   from [Microsoft ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)
   and caches them in the output directory.
2. **Tauri** copies the files into the app bundle via platform-specific configs
   (`tauri.windows.conf.json`, `tauri.macos.conf.json`).
3. **At runtime**, `main()` discovers the DLL next to the executable (production)
   or in `target/ort-dylibs/` (development) and sets `ORT_DYLIB_PATH` before
   any ONNX code runs.
4. **Graceful degradation**: if the DLL or model is missing, punctuation is
   silently disabled and raw text is used after transcription.

## Platform files

| Platform | Files |
|----------|-------|
| Windows  | `onnxruntime.dll`, `onnxruntime_providers_shared.dll` |
| macOS    | `libonnxruntime.dylib` |

## Punctuation model

The ONNX model (`model.onnx` + `tokenizer.json`) is published as a separate
HuggingFace repository:

```
https://huggingface.co/Aectann/punctuation-case-model
```

Users download it via the Push-to-Talk UI or manually:

```bash
hf download Aectann/punctuation-case-model model.onnx tokenizer.json \
  --local-dir ~/.push-to-talk/models/punctuator
```

The model is automatically discovered by scanning `<model_search_dir>/punctuator/model.onnx`.
Original model: [kontur-ai/sbert_punc_case_ru](https://huggingface.co/kontur-ai/sbert_punc_case_ru)
built on [ai-forever/sbert_large_nlu_ru](https://huggingface.co/ai-forever/sbert_large_nlu_ru).
