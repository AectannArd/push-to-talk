//! Build script: download ONNX Runtime native libraries for the target platform,
//! then run Tauri's build.
//!
//! The native libraries are cached in `ort-dylibs/{platform}/` and only
//! downloaded once — subsequent builds reuse the cached files.

use std::io::{Read, Write};
use std::path::PathBuf;
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

    let (platform, lib_name, archive_url) = if target.contains("windows") {
        (
            "windows",
            "onnxruntime.dll",
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-win-x64-{ORT_VERSION}.zip"),
        )
    } else if target.contains("apple") {
        (
            "macos",
            "libonnxruntime.dylib",
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-arm64-{ORT_VERSION}.tgz"),
        )
    } else {
        return; // unsupported target — skip
    };

    let dest_dir = PathBuf::from("ort-dylibs").join(platform);
    let marker = dest_dir.join(lib_name);

    // Already downloaded — nothing to do
    if marker.exists() && marker.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        println!("cargo:warning=ONNX Runtime libs already present in {}", dest_dir.display());
        return;
    }

    // Create destination directory
    let _ = std::fs::create_dir_all(&dest_dir);

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

    println!("cargo:warning=  Downloaded {:.1} MB", archive_bytes.len() as f64 / (1024.0 * 1024.0));

    // Extract using `tar` — works for both .zip and .tar.gz on all modern platforms.
    // Windows 10 build 17063+ includes tar with zip support.
    let extract_dir = dest_dir.join("_extract");
    let _ = std::fs::create_dir_all(&extract_dir);

    let status = Command::new("tar")
        .arg("xf")
        .arg("-")
        .arg("-C")
        .arg(&extract_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            let stdin = child.stdin.as_mut().unwrap();
            stdin.write_all(&archive_bytes)?;
            child.wait()
        });

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=  tar exited with {s}");
            let _ = std::fs::remove_dir_all(&extract_dir);
            return;
        }
        Err(e) => {
            println!("cargo:warning=  tar failed: {e}");
            let _ = std::fs::remove_dir_all(&extract_dir);
            return;
        }
    }

    // Find and copy the needed files from the extracted tree
    copy_ort_files(&extract_dir, &dest_dir, platform);

    // Clean up extraction directory
    let _ = std::fs::remove_dir_all(&extract_dir);

    if marker.exists() && marker.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        println!("cargo:warning=  ✓ ONNX Runtime libs ready for bundling");
    } else {
        println!("cargo:warning=  ✗ Extraction completed but libs not found — check archive structure");
    }
}

/// Walk the extracted directory and copy ONNX Runtime shared libraries
/// into the destination directory.
fn copy_ort_files(src: &PathBuf, dest: &PathBuf, platform: &str) {
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
        }
    }
}

/// Recursively search `dir` for a file named `name`.
/// Skips the directory part matching `_extract` (archive root) to avoid
/// matching deeply nested paths.
fn find_file(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    // Walk the directory tree manually to avoid walkdir dependency
    let mut stack = vec![dir.clone()];
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
