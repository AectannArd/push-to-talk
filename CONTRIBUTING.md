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

  **Linux:**
  ```bash
  # Ubuntu/Debian
  sudo apt-get install -y \
    libasound2-dev \
    libudev-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    pkg-config

  # Fedora
  sudo dnf install -y \
    alsa-lib-devel \
    libudev-devel \
    libxcb-devel \
    pkg-config
  ```

  **Windows:**
  ```bash
  # No additional dependencies required
  ```

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
- Test on your platform (macOS/Linux/Windows)

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
- [ ] Tested on Linux
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
- **Platform:** macOS/Linux/Windows + version
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
├── main.rs        # Application entry point, event loop
├── config.rs      # Configuration management (TOML)
├── recorder.rs    # Audio recording with cpal
├── transcriber.rs # Whisper.cpp integration
├── hotkey.rs      # Global hotkey handling with rdev
├── indicator.rs   # UI indicator window
└── ui.rs          # Interactive CLI wizard
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
