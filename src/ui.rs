//! User interface for config review and editing.

use crate::config;
use crate::hotkey;
use crate::recorder;
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Print current config and ask the user if they want to edit it.
pub fn review_config(cfg: &mut config::Config, force: bool) -> Result<()> {
    use std::io::Write;

    eprintln!();
    eprintln!("┌─ Current config ───────────────────────────────────");
    eprintln!(
        "│ device_id:          {}",
        cfg.device_id.as_deref().unwrap_or("<prompt on startup>")
    );
    eprintln!(
        "│ device_name:        {}",
        cfg.device_name.as_deref().unwrap_or("<not set>")
    );
    eprintln!(
        "│ language:           {}",
        cfg.language.as_deref().unwrap_or("auto")
    );
    eprintln!(
        "│ model:              {}",
        cfg.model.as_deref().unwrap_or("<not set>")
    );
    eprintln!("│ hotkey:             {}", cfg.hotkey);
    eprintln!("│ model_search_dirs:  {:?}", cfg.model_search_dirs);
    eprintln!("│ log_dir:            {}", cfg.log_dir);
    eprintln!("│ log_level:          {}", cfg.log_level);
    eprintln!("│ log_format:         {}", cfg.log_format);
    eprintln!("│ log_retention:      {}h", cfg.log_retention_hours);
    eprintln!("└─────────────────────────────────────────────────────");

    if !force {
        eprint!("\n✏  Edit config? [y/N]: ");
        std::io::stderr().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            return Ok(());
        }
    } else {
        eprintln!("\n⚠  Model not configured or file missing — setup required.");
    }

    let mut input = String::new();

    // -- Step 1: Model search dirs FIRST (before model selection) --
    eprintln!();
    eprintln!("─── Model search directories ─────────────────────────");
    eprintln!("These directories are scanned for ggml-*.bin files.");
    let cur_dirs = cfg.model_search_dirs.join(", ");
    eprint!("Directories (comma-separated) [{cur_dirs}]: ");
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.model_search_dirs = val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // -- Step 2: Scan with the (possibly updated) dirs, pick model --
    eprintln!();
    eprintln!("─── Model selection ──────────────────────────────────");
    let models = scan_all_models(&cfg.model_search_dirs);
    if models.is_empty() {
        eprintln!(
            "⚠  No ggml-*.bin models found in: {dirs:?}",
            dirs = cfg.model_search_dirs
        );
    } else {
        eprintln!("┌─ Available models ───────────────────────────────────");
        for (i, m) in models.iter().enumerate() {
            let mark = if Some(&m.to_string_lossy().to_string()) == cfg.model.as_ref() {
                " ← current"
            } else {
                ""
            };
            eprintln!("│ [{n}] {path}{mark}", n = i + 1, path = m.display());
        }
        eprintln!("└──────────────────────────────────────────────────────");
    }

    eprintln!();
    eprintln!("Options:");
    eprintln!("  1. Select from found models");
    eprintln!("  2. Download model from HuggingFace");
    eprintln!("  3. Skip (will retry on next start)");
    eprint!("Choose option [1]: ");
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let option = input.trim().to_string();

    match option.as_str() {
        "2" => {
            // Download from HuggingFace
            let has_configured_model = cfg.model.is_some();
            match download_model_from_huggingface(&cfg.model_search_dirs, has_configured_model) {
                Ok(Some(path)) => {
                    cfg.model = Some(path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    // User chose to skip
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("⚠  Failed to download model: {e}");
                    if cfg.model.is_none() && models.is_empty() {
                        return Ok(());
                    }
                }
            }
        }
        "3" => {
            eprintln!("⚠  No model selected — will retry on next start.");
            if cfg.model.is_none() && models.is_empty() {
                return Ok(());
            }
        }
        _ => {
            // Select from found models (default)
            if models.is_empty() {
                eprintln!("⚠  No models found. Please download a model first.");
                return Ok(());
            }

            // If no model is configured, pre-select the first one
            if cfg.model.is_none() && !models.is_empty() {
                cfg.model = Some(models[0].to_string_lossy().to_string());
                eprintln!("✓ Auto-selected first model: {}", models[0].display());
                eprintln!("  (enter index to choose a different model)");
            }

            let cur_model = cfg.model.as_deref().unwrap_or("<not set>");
            eprintln!("Current: {cur_model}");
            eprint!("Model index (or Enter to keep): ");
            std::io::stderr().flush().ok();
            input.clear();
            std::io::stdin().read_line(&mut input).ok();
            let val = input.trim().to_string();
            if !val.is_empty() {
                if let Ok(idx) = val.parse::<usize>() {
                    if idx >= 1 {
                        if let Some(m) = models.get(idx - 1) {
                            cfg.model = Some(m.to_string_lossy().to_string());
                        } else {
                            eprintln!("⚠  Invalid index");
                        }
                    } else {
                        eprintln!("⚠  Invalid index");
                    }
                } else {
                    eprintln!("⚠  Enter a numeric index");
                }
            }
        }
    }

    if force && cfg.model.is_none() {
        eprintln!("⚠  No model selected — will retry on next start.");
        return Ok(());
    }

    // -- Remaining settings (skip in force mode unless user wants to) --
    if force {
        eprintln!();
        eprintln!("─── Remaining settings ───────────────────────────────");
        eprint!("Review other settings? [y/N]: ");
        std::io::stderr().flush().ok();
        input.clear();
        std::io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            return Ok(());
        }
    }

    edit_remaining_settings(cfg);
    Ok(())
}

/// Interactive editing of all non-model settings.
pub fn edit_remaining_settings(cfg: &mut config::Config) {
    let mp = MultiProgress::new();

    // -- Device: show current, offer to change --
    let devices = match recorder::list_input_devices() {
        Ok(devs) => devs,
        Err(e) => {
            eprintln!("⚠  Could not list devices: {e}");
            eprintln!("  Keeping current device");
            return;
        }
    };

    eprintln!();
    eprintln!("Device:");
    eprintln!("┌─ Available input devices ────────────────────────────");
    for d in &devices {
        let is_current = cfg.device_id.as_ref().map_or(false, |id| id == &d.id);
        let marker = if is_current { " (current)" } else if d.is_default { " (default)" } else { "" };
        eprintln!("│ [{n}] {name}", n = d.index + 1, name = d.name);
        eprintln!("│     ID: {id} | {cfg}{marker}", id = d.id, cfg = d.config);
    }
    eprintln!("└──────────────────────────────────────────────────────");

    // Find current device number for the prompt
    let current_num = cfg.device_id.as_ref().and_then(|id| {
        devices.iter().find(|d| &d.id == id).map(|d| d.index + 1)
    }).unwrap_or(1);

    let pb = mp.add(ProgressBar::new(1));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("🎙  Device number [{current_num}]: {wide_bar:.cyan/blue} {pos}/{len}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
    );
    pb.set_length(devices.len() as u64);
    pb.set_position((current_num - 1) as u64);

    eprint!("Enter device number (or press Enter for {current_num}): ");
    std::io::stderr().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();

    if !val.is_empty() {
        if let Ok(idx) = val.parse::<usize>() {
            if idx >= 1 && idx <= devices.len() {
                let d = &devices[idx - 1];
                cfg.device_id = Some(d.id.clone());
                cfg.device_name = Some(d.name.clone());
                pb.set_position((idx - 1) as u64);
                pb.finish_with_message("✓ Device updated");
            } else {
                pb.finish_with_message("⚠  Invalid index – keeping current device");
            }
        } else {
            pb.finish_with_message("⚠  Invalid input – keeping current device");
        }
    } else {
        pb.finish_with_message(format!("✓ Keeping device {current_num}"));
    }

    // -- Edit language --
    eprintln!();
    eprintln!("Language (ISO 639-1 code, or 'auto'):");
    let cur_lang = cfg.language.as_deref().unwrap_or("auto");
    eprint!("Language [{cur_lang}]: ");
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let lang_val = input.trim().to_string();
    if !lang_val.is_empty() {
        cfg.language = Some(lang_val);
    }

    // -- Edit hotkey --
    eprintln!();
    eprintln!("Hotkey (format: Mod+Mod+Key, e.g. Ctrl+Shift+T):");
    eprintln!(
        "Modifiers: Ctrl, Shift, Alt, {win}",
        win = mod_win_label().trim_end_matches('+'),
    );
    eprintln!();
    eprintln!("┌─ Supported keys ──────────────────────────────────────");
    for (name, aliases) in hotkey::supported_keys() {
        if aliases.len() == 1 {
            eprintln!("│ {name}");
        } else {
            eprintln!("│ {name}  ({})", aliases[1..].join(", "));
        }
    }
    eprintln!("└──────────────────────────────────────────────────────");
    eprint!("\nHotkey [{}]: ", cfg.hotkey);
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        match hotkey::parse_hotkey(&val) {
            Ok(_) => cfg.hotkey = val,
            Err(e) => eprintln!("⚠  Invalid hotkey: {e} — keeping current"),
        }
    }

    // -- Edit log dir --
    eprintln!();
    eprintln!("Log directory:");
    eprint!("Log dir [{}]: ", cfg.log_dir);
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        cfg.log_dir = val;
    }

    // -- Edit log level --
    eprintln!();
    eprintln!("Log level (trace, debug, info, warn, error):");
    eprint!("Log level [{}]: ", cfg.log_level);
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        let lv = val.to_lowercase();
        if matches!(lv.as_str(), "trace" | "debug" | "info" | "warn" | "error") {
            cfg.log_level = lv;
        } else {
            eprintln!("⚠  Invalid level — keeping current");
        }
    }

    // -- Edit log format --
    eprintln!();
    eprintln!("Log format (text or json):");
    eprint!("Format [{}]: ", cfg.log_format);
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_lowercase();
    if !val.is_empty() {
        if matches!(val.as_str(), "text" | "json") {
            cfg.log_format = val;
        } else {
            eprintln!("⚠  Invalid format — use 'text' or 'json'");
        }
    }

    // -- Edit log retention --
    eprintln!();
    eprintln!("Log retention in hours (rotated files older than this are deleted):");
    eprint!("Retention hours [{}]: ", cfg.log_retention_hours);
    std::io::stderr().flush().ok();
    input.clear();
    std::io::stdin().read_line(&mut input).ok();
    let val = input.trim().to_string();
    if !val.is_empty() {
        if let Ok(n) = val.parse::<u64>() {
            if n > 0 {
                cfg.log_retention_hours = n;
            } else {
                eprintln!("⚠  Must be at least 1");
            }
        } else {
            eprintln!("⚠  Not a number");
        }
    }
}

/// Download a Whisper model from HuggingFace.
/// Returns Ok(Some(path)) if downloaded, Ok(None) if skipped, Err if failed.
fn download_model_from_huggingface(
    search_dirs: &[String],
    has_configured_model: bool,
) -> Result<Option<PathBuf>> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::{Read, Write};

    // Define available models
    let models: Vec<(&str, &str, &str)> = vec![
        ("1", "ggml-tiny.bin", "~78 MB  - Fast, lower accuracy"),
        ("2", "ggml-base.bin", "~142 MB - Balanced (recommended)"),
        ("3", "ggml-small.bin", "~244 MB - Better accuracy"),
        ("4", "ggml-medium.bin", "~769 MB - High accuracy"),
        ("5", "ggml-large-v3.bin", "~1.5 GB - Best accuracy"),
    ];

    // Determine target directory
    let target_dir = if !search_dirs.is_empty() {
        PathBuf::from(&search_dirs[0])
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".push-to-talk").join("models")
    };

    // Check which models are already downloaded
    let mut downloaded_models: Vec<String> = Vec::new();
    if target_dir.exists() {
        for entry in std::fs::read_dir(&target_dir).ok().into_iter().flatten() {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("ggml-") && name.ends_with(".bin") {
                    downloaded_models.push(name);
                }
            }
        }
    }

    // Filter out already downloaded models
    let available_models: Vec<_> = models
        .iter()
        .filter(|(_, name, _)| !downloaded_models.iter().any(|d| d == *name))
        .collect();

    eprintln!();
    if available_models.is_empty() {
        eprintln!("✓ All models are already downloaded.");
        eprintln!("Existing models in {}:", target_dir.display());
        for model in &downloaded_models {
            eprintln!("  • {model}");
        }
        return Ok(None);
    }

    eprintln!("Available models from ggerganov/whisper.cpp:");
    for (idx, name, desc) in available_models.iter().map(|(i, n, d)| (*i, *n, *d)) {
        eprintln!("  {idx}. {name:<18} {desc}");
    }

    // Show skip option if model is already configured
    if has_configured_model {
        eprintln!("  0. Skip download (keep current model)");
    }

    eprint!("Choose model [2]: ");
    std::io::stderr().flush().ok();

    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice).ok();
    let choice = choice.trim();

    // Check for skip option
    if choice == "0" && has_configured_model {
        eprintln!("✓ Skipping download.");
        return Ok(None);
    }

    // Find selected model
    let model_name = available_models
        .iter()
        .find(|(idx, _, _)| *idx == choice)
        .map(|(_, name, _)| *name)
        .unwrap_or("ggml-base.bin");

    // Check if selected model is already downloaded (shouldn't happen, but just in case)
    std::fs::create_dir_all(&target_dir)?;
    let target_path = target_dir.join(model_name);

    if target_path.exists() {
        eprintln!("✓ Model already exists at: {}", target_path.display());
        return Ok(Some(target_path));
    }

    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}");

    eprintln!();
    eprintln!("Downloading {model_name} from HuggingFace...");
    eprintln!("URL: {url}");
    eprintln!();

    let mut response = reqwest::blocking::get(&url)?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download: HTTP {}", response.status());
    }

    let total_size = response
        .content_length()
        .ok_or_else(|| anyhow::anyhow!("Failed to get content length"))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?,
    );

    let mut file = std::fs::File::create(&target_path)?;
    let mut downloaded: u64 = 0;

    let mut buffer = vec![0u8; 8192];
    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    eprintln!();
    eprintln!("✓ Model saved to: {}", target_path.display());

    Ok(Some(target_path))
}

/// Discover the Whisper model file.
///
/// 1. `WHISPER_MODEL` env var (exact path)
/// 2. `explicit_path` from config (exact path)
/// 3. Scan `search_dirs` for `ggml-*.bin`, pick the first one
pub fn find_model(explicit_path: &Option<String>, search_dirs: &[String]) -> Result<PathBuf> {
    use tracing::{info, warn};

    // 1. Env var override
    if let Ok(p) = std::env::var("WHISPER_MODEL") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Ok(path);
        }
        warn!("WHISPER_MODEL={p} does not exist");
    }

    // 2. Explicit model from config
    if let Some(p) = explicit_path {
        let path = PathBuf::from(p);
        if path.exists() {
            info!("🔍 Using model from config: {}", path.display());
            return Ok(path);
        }
        warn!("Configured model not found at {p}, scanning…");
    }

    // 3. Scan directories
    for dir in search_dirs {
        if let Some(path) = scan_for_model(Path::new(dir)) {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No Whisper model (ggml-*.bin) found in: {dirs:?}\n\
         Download: curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        dirs = search_dirs,
    )
}

/// Scan directories for all ggml-*.bin files, returning full paths.
pub fn scan_all_models(dirs: &[String]) -> Vec<PathBuf> {
    let mut models = Vec::new();
    for dir in dirs {
        let root = Path::new(dir);
        if !root.exists() {
            continue;
        }
        collect_models(root, 0, &mut models);
    }
    models
}

fn collect_models(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_models(&path, depth + 1, out);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ggml-") && n.ends_with(".bin"))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
}

/// Recursively scan `root` (up to 3 levels deep) for `ggml-*.bin` files.
fn scan_for_model(root: &Path) -> Option<PathBuf> {
    use tracing::info;

    if !root.exists() {
        return None;
    }

    fn walk(dir: &Path, depth: u32) -> Option<PathBuf> {
        if depth > 3 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path, depth + 1) {
                    return Some(found);
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("ggml-") && n.ends_with(".bin"))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
        None
    }

    let found = walk(root, 0);
    if let Some(ref p) = found {
        info!("🔍 Found model: {}", p.display());
    }
    found
}

/// Platform-appropriate label for the Win/Cmd modifier key.
fn mod_win_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+"
    } else {
        "Win+"
    }
}
