//! Build script: download ONNX Runtime native libraries for the target platform,
//! then run Tauri's build.
//!
//! The native libraries are cached in `ort-dylibs/{platform}/` and only
//! downloaded once — subsequent builds reuse the cached files.

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

fn main() {
    download_ort_libs();
    tauri_build::build();
}

/// Download ONNX Runtime shared libraries for the current target platform
/// into `ort-dylibs/{platform}/` if they are not already present.
fn download_ort_libs() {
    let Ok(target) = std::env::var("TARGET") else {
        return;
    };

    const ORT_VERSION: &str = "1.24.4";

    // Expected SHA256 hashes for the downloaded archives (must match ORT_VERSION).
    // Update these when bumping ORT_VERSION.
    const EXPECTED_WIN_SHA256: &str =
        "d2319fddfb6ea4db99ccc4b60c85c517bcd855721f5daa6a06d40d7cb2ee2357";
    const EXPECTED_MAC_SHA256: &str =
        "93787795f47e1eee369182e43ed51b9e5da0878ab0346aecf4258979b8bba989";

    // Parse architecture from the target triple (e.g. "x86_64-pc-windows-msvc" → "x86_64")
    let arch = target.split('-').next().unwrap_or("unknown");

    let (platform, _lib_name, archive_url) = if target.contains("windows") {
        let win_arch = match arch {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => {
                println!("cargo:warning=ONNX Runtime: unsupported Windows architecture '{other}' — only x86_64 and aarch64 are supported.");
                println!("cargo:warning=  Punctuation will be unavailable on this target.");
                return;
            }
        };
        (
            "windows",
            "onnxruntime.dll",
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-win-{win_arch}-{ORT_VERSION}.zip"),
        )
    } else if target.contains("apple") {
        // ONNX Runtime 1.24+ only ships arm64 macOS binaries — Intel Macs are not supported.
        if arch != "aarch64" {
            println!(
                "cargo:warning=ONNX Runtime 1.24+ only provides arm64 macOS binaries. \
                 Detected architecture '{arch}'. Punctuation will be unavailable."
            );
            return;
        }
        (
            "macos",
            "libonnxruntime.dylib",
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-arm64-{ORT_VERSION}.tgz"),
        )
    } else {
        return; // unsupported target — skip
    };

    // ONNX_RT_OUTPUT overrides the default; falls back to target/ort-dylibs
    let output_root = std::env::var("ONNX_RT_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target").join("ort-dylibs"));
    let dest_dir = output_root.join(platform);
    let version_marker = dest_dir.join(".ort-version");

    // Verify all wanted files are present (not just the marker)
    let wanted: &[&str] = match platform {
        "windows" => &["onnxruntime.dll", "onnxruntime_providers_shared.dll"],
        "macos" => &["libonnxruntime.dylib"],
        _ => &[],
    };

    // Check cache: all files present with correct version and sizes
    if version_marker.exists() {
        if let Ok(contents) = std::fs::read_to_string(&version_marker) {
            let cached = parse_version_marker(&contents);
            if cached.version == ORT_VERSION {
                let mut valid = true;
                for &name in wanted {
                    let path = dest_dir.join(name);
                    let actual_size = path
                        .metadata()
                        .map(|m| m.len())
                        .unwrap_or(0);
                    let expected_size = cached
                        .sizes
                        .get(name)
                        .copied()
                        .unwrap_or(0);
                    if actual_size == 0 || actual_size != expected_size {
                        if actual_size > 0 {
                            println!(
                                "cargo:warning=  {} size mismatch: expected {expected_size}, got {actual_size}",
                                name
                            );
                        }
                        valid = false;
                        break;
                    }
                }
                if valid {
                    println!(
                        "cargo:warning=ONNX Runtime {} libs already present in {}",
                        ORT_VERSION,
                        dest_dir.display()
                    );
                    return;
                }
            }
        }
    }
    // Stale or corrupted cache — clean up
    println!("cargo:warning=  Cleaning up stale ONNX Runtime cache...");
    for &name in wanted {
        let _ = std::fs::remove_file(dest_dir.join(name));
    }
    let _ = std::fs::remove_file(&version_marker);

    // Create destination directory
    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        println!(
            "cargo:warning=  Failed to create {}: {e}",
            dest_dir.display()
        );
        return;
    }

    println!("cargo:warning=Downloading ONNX Runtime for {platform}...");
    println!("cargo:warning=  {archive_url}");

    // Download archive into memory
    let archive_bytes = match ureq::get(archive_url).call() {
        Ok(resp) => {
            let mut buf = Vec::new();
            match resp.into_body().into_reader().read_to_end(&mut buf) {
                Ok(_) => buf,
                Err(e) => {
                    println!("cargo:warning=  Failed to read response: {e}");
                    return;
                }
            }
        }
        Err(e) => {
            println!("cargo:warning=  Download failed: {e}");
            println!("cargo:warning=  ONNX Runtime will not be bundled — punctuation will be unavailable.");
            return;
        }
    };

    println!(
        "cargo:warning=  Downloaded {:.1} MB",
        archive_bytes.len() as f64 / (1024.0 * 1024.0)
    );

    // Verify archive integrity before extraction
    let expected_hash = if platform == "windows" {
        EXPECTED_WIN_SHA256
    } else {
        EXPECTED_MAC_SHA256
    };
    let actual_hash = format!("{:x}", Sha256::digest(&archive_bytes));
    if actual_hash != expected_hash {
        println!("cargo:warning=  Archive checksum mismatch!");
        println!("cargo:warning=  Expected: {expected_hash}");
        println!("cargo:warning=  Got:      {actual_hash}");
        println!("cargo:warning=  ONNX Runtime will not be bundled — punctuation will be unavailable.");
        return;
    }
    println!("cargo:warning=  ✓ Archive checksum verified");

    // Extract in pure Rust — no external tar dependency
    let extract_result = if platform == "windows" {
        extract_zip(&archive_bytes, &dest_dir, wanted)
    } else {
        extract_tgz(&archive_bytes, &dest_dir, wanted)
    };
    if let Err(e) = extract_result {
        println!("cargo:warning=  Extraction failed: {e}");
        return;
    }

    // Verify all wanted files are present
    let mut all_ok = true;
    for &name in wanted {
        let path = dest_dir.join(name);
        if !(path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)) {
            println!("cargo:warning=  ✗ {name} missing or empty after extraction",);
            all_ok = false;
        }
    }
    if all_ok {
        // Write version sentinel with file sizes for integrity checks
        let mut marker_contents = format!("version:{ORT_VERSION}\n");
        for &name in wanted {
            let path = dest_dir.join(name);
            if let Ok(meta) = path.metadata() {
                marker_contents.push_str(&format!("{name}:{}\n", meta.len()));
            }
        }
        if let Err(e) = std::fs::write(&version_marker, &marker_contents) {
            println!("cargo:warning=  Failed to write version marker: {e}");
        }
        println!(
            "cargo:warning=  ✓ ONNX Runtime {} libs ready for bundling",
            ORT_VERSION
        );
    } else {
        println!("cargo:warning=  ✗ Extraction completed but some libs not found — check archive structure");
    }
}

/// Extract wanted files from a .zip archive into `dest_dir`.
fn extract_zip(data: &[u8], dest_dir: &Path, wanted: &[&str]) -> Result<(), String> {
    let cursor = std::io::Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {e}"))?;

    let wanted_set: HashSet<&str> = wanted.iter().copied().collect();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry {i}: {e}"))?;
        let name = entry.name().to_string();
        // Skip directory entries and non-wanted files
        if entry.is_dir() {
            continue;
        }
        // Match by filename (e.g. "onnxruntime.dll" inside "onnxruntime-win-x64-1.24.4/lib/")
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if wanted_set.contains(file_name) {
            let dst = dest_dir.join(file_name);
            let mut out =
                std::fs::File::create(&dst).map_err(|e| format!("Failed to create {file_name}: {e}"))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Failed to extract {file_name}: {e}"))?;
            println!("cargo:warning=  ✓ {file_name}");
        }
    }
    Ok(())
}

/// Extract wanted files from a .tar.gz archive into `dest_dir`.
fn extract_tgz(data: &[u8], dest_dir: &Path, wanted: &[&str]) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(gz);
    let wanted_set: HashSet<&str> = wanted.iter().copied().collect();

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {e}"))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {e}"))?;
        let file_name = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(String::from)))
            .unwrap_or_default();
        if wanted_set.contains(file_name.as_str()) {
            let dst = dest_dir.join(&file_name);
            entry
                .unpack(&dst)
                .map_err(|e| format!("Failed to extract {file_name}: {e}"))?;
            println!("cargo:warning=  ✓ {file_name}");
        }
    }
    Ok(())
}

/// Parsed contents of the `.ort-version` cache sentinel.
struct CachedVersion {
    version: String,
    sizes: HashMap<String, u64>,
}

/// Parse a `.ort-version` file of the form:
///
/// ```text
/// version:1.24.4
/// onnxruntime.dll:14203464
/// onnxruntime_providers_shared.dll:22088
/// ```
///
/// Lines using the old format (plain version string only) are treated as
/// having an empty sizes map, forcing a re-download to populate them.
fn parse_version_marker(contents: &str) -> CachedVersion {
    let mut version = String::new();
    let mut sizes = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(ver) = line.strip_prefix("version:") {
            version = ver.to_string();
        } else if let Some((name, size_str)) = line.split_once(':') {
            if let Ok(size) = size_str.parse::<u64>() {
                sizes.insert(name.to_string(), size);
            }
        } else {
            // Old format: plain version string, no sizes
            version = line.to_string();
        }
    }

    CachedVersion { version, sizes }
}
