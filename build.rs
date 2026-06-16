//! Build script: download ONNX Runtime native libraries for the target platform,
//! then run Tauri's build.
//!
//! The native libraries are cached in `ort-dylibs/{platform}/` and only
//! downloaded once — subsequent builds reuse the cached files.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

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

    let dest_dir = PathBuf::from("ort-dylibs").join(platform);
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

    // Extract using `tar` — works for both .zip and .tar.gz on all modern platforms.
    // Windows 10 build 17063+ includes tar with zip support.
    let extract_dir = dest_dir.join("_extract");
    if let Err(e) = std::fs::create_dir_all(&extract_dir) {
        println!(
            "cargo:warning=  Failed to create {}: {e}",
            extract_dir.display()
        );
        return;
    }

    let mut tar_child = match Command::new("tar")
        .arg("xf")
        .arg("-")
        .arg("-C")
        .arg(&extract_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            println!("cargo:warning=  Failed to spawn tar: {e}");
            if let Err(e) = std::fs::remove_dir_all(&extract_dir) {
                println!(
                    "cargo:warning=  Failed to clean up {}: {e}",
                    extract_dir.display()
                );
            }
            return;
        }
    };

    // Feed archive bytes to tar's stdin
    if let Some(stdin) = tar_child.stdin.as_mut() {
        if let Err(e) = stdin.write_all(&archive_bytes) {
            println!("cargo:warning=  Failed to write archive to tar stdin: {e}");
            if let Err(e) = std::fs::remove_dir_all(&extract_dir) {
                println!(
                    "cargo:warning=  Failed to clean up {}: {e}",
                    extract_dir.display()
                );
            }
            return;
        }
    }

    let status = tar_child.wait();

    // Log any stderr output from tar (non-fatal warnings)
    if let Some(stderr) = tar_child.stderr.as_mut() {
        let mut buf = String::new();
        if std::io::Read::read_to_string(stderr, &mut buf).is_ok() && !buf.trim().is_empty() {
            println!("cargo:warning=  tar stderr: {}", buf.trim());
        }
    }

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=  tar exited with {s}");
            if let Err(e) = std::fs::remove_dir_all(&extract_dir) {
                println!(
                    "cargo:warning=  Failed to clean up {}: {e}",
                    extract_dir.display()
                );
            }
            return;
        }
        Err(e) => {
            println!("cargo:warning=  tar wait failed: {e}");
            if let Err(e) = std::fs::remove_dir_all(&extract_dir) {
                println!(
                    "cargo:warning=  Failed to clean up {}: {e}",
                    extract_dir.display()
                );
            }
            return;
        }
    }

    // Find and copy the needed files from the extracted tree
    copy_ort_files(&extract_dir, &dest_dir, platform);

    // Clean up extraction directory
    if let Err(e) = std::fs::remove_dir_all(&extract_dir) {
        println!(
            "cargo:warning=  Failed to clean up {}: {e}",
            extract_dir.display()
        );
    }

    // Verify all wanted files are present, not just the marker
    let wanted: &[&str] = match platform {
        "windows" => &["onnxruntime.dll", "onnxruntime_providers_shared.dll"],
        "macos" => &["libonnxruntime.dylib"],
        _ => &[],
    };
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

/// Walk the extracted directory and copy ONNX Runtime shared libraries
/// into the destination directory.
fn copy_ort_files(src: &Path, dest: &Path, platform: &str) {
    let wanted: &[&str] = match platform {
        "windows" => &["onnxruntime.dll", "onnxruntime_providers_shared.dll"],
        "macos" => &["libonnxruntime.dylib"],
        _ => return,
    };

    // Recursively search for each wanted file
    for &name in wanted {
        if let Some(found) = find_file(src, name) {
            let dst = dest.join(name);
            if let Err(e) = std::fs::copy(&found, &dst) {
                println!("cargo:warning=  Failed to copy {}: {e}", found.display());
            } else {
                println!("cargo:warning=  ✓ {name}");
            }
        } else {
            println!("cargo:warning=  ✗ {name} not found in archive — archive structure may have changed");
        }
    }
}

/// Recursively search `dir` for a file named `name`.
/// Skips the directory part matching `_extract` (archive root) to avoid
/// matching deeply nested paths.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    // Walk the directory tree manually to avoid walkdir dependency
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        } else if let Some(fname) = path.file_name() {
            if fname == name {
                return Some(path);
            }
        }
    }
    None
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
