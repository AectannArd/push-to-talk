/** Matches Rust Config struct (config.rs) */
export interface Config {
  device_id: string | null;
  device_name: string | null;
  language: string | null;
  model: string | null;
  model_search_dirs: string[];
  hotkey: string;
  log_dir: string;
  log_level: string;
  log_format: string;
  log_retention_hours: number;
  punctuation_enabled: boolean;
  ui_language: string;
}

/** Matches Rust StatusDto (main.rs) */
export interface Status {
  is_recording: boolean;
  is_service_running: boolean;
  hotkey: string;
  language: string | null;
  last_transcription: string | null;
}

/** Matches Rust DeviceDto (main.rs) */
export interface Device {
  id: string;
  name: string;
  config: string;
  is_default: boolean;
}

/** Matches Rust ModelDto (main.rs) */
export interface Model {
  filename: string;
  path: string;
  size: string;
}

/** Matches Rust DownloadableModel (main.rs) */
export interface DownloadableModel {
  id: string;
  name: string;
  desc: string;
  url: string;
}

/** Matches Rust PunctuationModelStatus (main.rs) */
export interface PunctuationModelStatus {
  found: boolean;
  model_path: string | null;
  onnx_url: string;
  tokenizer_url: string;
}
