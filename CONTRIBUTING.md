# Contributing to push-to-talk

Thank you for considering contributing to push-to-talk! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Pull Request Guidelines](#pull-request-guidelines)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Keep discussions on-topic

## Getting Started

1. **Fork** the repository
2. **Clone** your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/push-to-talk.git
   cd push-to-talk
   ```
3. **Create a branch** for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- **Rust** 1.85 or later ([install via rustup](https://rustup.rs/))
- **Platform-specific dependencies:**

  **macOS:**
  ```bash
  # No additional dependencies required
  ```

  **Windows:**
  ```bash
  # No additional dependencies required
  ```

### macOS Permissions

The application requires the following macOS permissions to function correctly:

| Permission | Purpose | Required For |
|------------|---------|--------------|
| **Accessibility** | Global hotkey detection (rdev) | Capturing modifier keys (Ctrl, Shift, Alt, Cmd) |
| **Input Monitoring** | Keyboard input simulation (enigo) | Typing transcribed text into active applications |
| **Automation** | AppleScript-based paste fallback | Reliable text insertion via System Events |

#### Granting Permissions

1. **System Settings → Privacy & Security → Accessibility**
   - Add your terminal app (e.g., iTerm2, Terminal) or `push-to-talk.app`
   - Toggle the switch to enable

2. **System Settings → Privacy & Security → Input Monitoring**
   - Add your terminal app or `push-to-talk.app`
   - Toggle the switch to enable

3. **System Settings → Privacy & Security → Automation**
   - Add your terminal app or `push-to-talk.app`
   - Enable "System Events" checkbox

#### Resetting Permissions (Troubleshooting)

If text insertion stops working or permissions become corrupted, reset them:

```bash
# Reset Accessibility permissions
tccutil reset Accessibility org.moonlightflame.push-to-talk

# Reset Input Monitoring permissions
tccutil reset InputMonitoring org.moonlightflame.push-to-talk

# Reset Automation permissions
tccutil reset Automation org.moonlightflame.push-to-talk

# Reset all permissions for the app
tccutil reset All org.moonlightflame.push-to-talk
```

**Note:** The bundle identifier `org.moonlightflame.push-to-talk` is used when running from a `.app` bundle. For CLI usage, permissions are granted to the terminal application itself (e.g., `com.googlecode.iterm2` for iTerm2).

After resetting, restart the application and re-grant permissions when prompted.

#### Verifying Permissions

Check if permissions are granted:

```bash
# Check Accessibility (returns 0 if granted)
osascript -e 'tell application "System Events" to keystroke "tab"' 2>&1

# Check if app is in Accessibility list
defaults read com.apple.universalaccessAssistiveApplications 2>/dev/null | grep -i "terminal\|iterm\|push-to-talk"
```

#### Common Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| Hotkey not working | Missing Accessibility | Grant Accessibility permission |
| Text not appearing in apps | Missing Input Monitoring | Grant Input Monitoring permission |
| AppleScript errors | Missing Automation | Grant Automation → System Events |
| Permissions reset after update | macOS security | Re-grant all permissions |

### GUI Configuration (macOS)

A Tauri-based GUI is embedded in the main binary for configuration on macOS.

**Opening the GUI:**

```bash
# Open the configuration window
push-to-talk --gui
```

**Features:**
- Modern gradient UI design
- All configuration fields editable
- Save configuration with validation
- Built with Tauri 2.x + HTML/CSS/JavaScript

**Note:** First build downloads WebKit and can take 10-20 minutes.

### Menu Bar Tray (macOS)

The application includes a menu bar tray icon on macOS for quick access.

**Features:**
- 🎤 Microphone icon in menu bar
- Status indicator (Idle / ● Recording)
- Configure... menu item (opens Tauri GUI)
- Quit application menu item

### ONNX Runtime (Punctuation)

Push-to-talk uses ONNX Runtime for punctuation and case restoration. The native
libraries are downloaded automatically by `build.rs` during the first build and
cached in `target/ort-dylibs/{platform}/`.

**Default location:** `target/ort-dylibs/`

| Platform | Files |
|----------|-------|
| Windows  | `target/ort-dylibs/windows/onnxruntime.dll`, `onnxruntime_providers_shared.dll` |
| macOS    | `target/ort-dylibs/macos/libonnxruntime.dylib` |

Override the output directory with the `ONNX_RT_OUTPUT` environment variable:

```bash
ONNX_RT_OUTPUT=/custom/path cargo build
```

During development (`cargo run` / `cargo tauri dev`), the DLL is discovered
automatically from `target/ort-dylibs/`. In production builds, Tauri bundles
the DLLs next to the executable.

The punctuation model (`model.onnx` + `tokenizer.json`) is published separately
on HuggingFace at [Aectann/punctuation-case-model](https://huggingface.co/Aectann/punctuation-case-model).
Users download it via the Push-to-Talk UI or manually into
`~/.push-to-talk/models/punctuator/`. If the DLL or model is missing,
punctuation is silently disabled and raw transcription text is used.

### Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the application
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings
```

## Making Changes

### Code Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` before committing
- Run `cargo clippy` to catch common mistakes
- Keep functions focused and small (< 50 lines preferred)

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add device selection wizard
fix: prevent cyclic key repeat on hotkey hold
docs: update README with installation instructions
refactor: extract UI code to separate module
chore: update dependencies
```

### Testing

- Write tests for new features
- Ensure all tests pass: `cargo test`
- Test on your platform (macOS or Windows)

### Documentation

- Update README.md for user-facing changes
- Add inline comments for complex logic
- Update CHANGELOG.md in the [Unreleased] section

## Pull Request Guidelines

### Before Submitting

- [ ] Code compiles: `cargo build --release`
- [ ] All tests pass: `cargo test`
- [ ] Code is formatted: `cargo fmt`
- [ ] Clippy warnings resolved: `cargo clippy`
- [ ] CHANGELOG.md updated
- [ ] Documentation updated

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change)
- [ ] New feature (non-breaking change)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
Describe how you tested these changes:
- [ ] Tested on macOS
- [ ] Tested on Windows

## Related Issues
Closes #ISSUE_NUMBER (if applicable)
```

### Review Process

1. Maintainer reviews code and tests
2. Automated checks must pass (CI/CD)
3. Address review feedback
4. PR is merged by maintainer

## Reporting Issues

### Bug Reports

Include:
- **Platform:** macOS or Windows + version
- **Rust version:** `rustc --version`
- **Steps to reproduce:** Clear, numbered steps
- **Expected behavior:** What should happen
- **Actual behavior:** What actually happened
- **Logs:** Any error messages or stack traces

### Feature Requests

Include:
- **Use case:** Why you need this feature
- **Proposed solution:** How it should work
- **Alternatives considered:** Other approaches you've thought about

## Architecture Overview

```
src/
├── main.rs           # Entry point, Tauri commands, global state, logging
├── config.rs         # Configuration management (TOML)
├── recorder.rs       # Audio recording with cpal
├── transcriber.rs    # Whisper.cpp integration
└── voice_service.rs  # Background service orchestrator, clipboard, transcription loop
```

## Release Process

1. Update CHANGELOG.md with release date
2. Update version in Cargo.toml
3. Create PR with changes
4. After merge, maintainer creates git tag: `git tag v0.1.0`
5. GitHub Actions builds and publishes release

## Questions?

Open an issue for any questions or discussions about the project.

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
