#!/usr/bin/env bash
#
# Build macOS distribution package for push-to-talk
# Creates a .dmg with the app bundle and model directory
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist/macos"
APP_NAME="push-to-talk"
VERSION="${VERSION:-$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[0].version')}"

echo "🔨 Building $APP_NAME v$VERSION for macOS..."

# Clean previous builds
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Build release binary
echo "📦 Building release binary..."
cargo build --release --target "$(rustc -vV | grep host | cut -d' ' -f2)"

# Create app bundle structure
APP_BUNDLE="$DIST_DIR/${APP_NAME}.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks"

# Copy binary
cp "$PROJECT_ROOT/target/release/$APP_NAME" "$APP_BUNDLE/Contents/MacOS/"
chmod +x "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.push-to-talk.cli</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>Push-to-Talk</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSMicrophoneUsageDescription</key>
    <string>This app requires microphone access to record audio for transcription.</string>
    <key>NSAppleEventsUsageDescription</key>
    <string>This app requires accessibility access to type transcribed text into active windows.</string>
</dict>
</plist>
EOF

# Create PkgInfo
echo -n "APPL????" > "$APP_BUNDLE/Contents/PkgInfo"

# Create models directory (user will download models)
MODELS_DIR="$DIST_DIR/models"
mkdir -p "$MODELS_DIR"
cat > "$MODELS_DIR/README.txt" <<EOF
Whisper Model Files
===================

Download a Whisper model file (ggml-*.bin) from:
https://huggingface.co/ggerganov/whisper.cpp/tree/main

Recommended models:
- ggml-base.bin   (142 MB) - Balanced speed/accuracy
- ggml-medium.bin (769 MB) - High accuracy

Place the downloaded .bin file in this directory.
EOF

# Create launcher script
cat > "$DIST_DIR/install.sh" <<'INSTALL_EOF'
#!/usr/bin/env bash
#
# Installation script for push-to-talk
#

set -euo pipefail

APP_NAME="push-to-talk"
INSTALL_DIR="/Applications"

echo "📦 Installing $APP_NAME..."

# Check if app bundle exists
if [[ ! -d "${APP_NAME}.app" ]]; then
    echo "❌ Error: ${APP_NAME}.app not found in current directory"
    exit 1
fi

# Copy to Applications
cp -R "${APP_NAME}.app" "$INSTALL_DIR/"

echo "✓ Installed to $INSTALL_DIR/${APP_NAME}.app"
echo ""
echo "⚠️  Important: First launch requires three permissions:"
echo ""
echo "   1. Input Monitoring (for global hotkey detection)"
echo "      System Preferences → Privacy & Security → Input Monitoring"
echo "      Add push-to-talk"
echo ""
echo "   2. Accessibility (for typing transcribed text)"
echo "      System Preferences → Privacy & Security → Accessibility"
echo "      Add push-to-talk"
echo ""
echo "   3. Microphone (for audio recording)"
echo "      System Preferences → Privacy & Security → Microphone"
echo "      Enable for push-to-talk"
echo ""
echo "🎙  Run with: /Applications/${APP_NAME}.app/Contents/MacOS/${APP_NAME}"
INSTALL_EOF

chmod +x "$DIST_DIR/install.sh"

# Create README
cat > "$DIST_DIR/README.md" <<EOF
# $APP_NAME for macOS

Version: $VERSION

## Installation

1. Copy \`${APP_NAME}.app\` to your Applications folder
2. Or run the included \`install.sh\` script:
   \`\`\`bash
   ./install.sh
   \`\`\`

## Required Permissions

macOS requires explicit permission for three features:

### 1. Input Monitoring (Global Hotkey Detection)

Required for detecting the push-to-talk hotkey combination globally.

**How to grant:**
1. Open **System Preferences** → **Privacy & Security** → **Input Monitoring**
2. Click the lock to make changes
3. Click **+** and add \`push-to-talk\` from \`/Applications/$APP_NAME.app\`
4. Restart the application

### 2. Accessibility (Auto-Typing)

Required for typing transcribed text into the active window.

**How to grant:**
1. Open **System Preferences** → **Privacy & Security** → **Accessibility**
2. Click the lock to make changes
3. Click **+** and add \`push-to-talk\` from \`/Applications/$APP_NAME.app\`

### 3. Microphone (Audio Recording)

Required for capturing audio for transcription.

**How to grant:**
1. Open **System Preferences** → **Privacy & Security** → **Microphone**
2. Enable the toggle for \`push-to-talk\`

## Bypassing Gatekeeper

If macOS prevents the app from launching:

\`\`\`bash
xattr -rd com.apple.quarantine /Applications/${APP_NAME}.app
\`\`\`

## Usage

Run from terminal:
\`\`\`bash
/Applications/${APP_NAME}.app/Contents/MacOS/${APP_NAME}
\`\`\`

Or create an alias:
\`\`\`bash
alias ptt="/Applications/${APP_NAME}.app/Contents/MacOS/${APP_NAME}"
\`\`\`

## Configuration

Config file: \`~/.push-to-talk/config.toml\`

On first run, you'll be prompted to:
1. Select/download a Whisper model
2. Choose audio input device
3. Configure hotkey (default: Ctrl+Shift+T)

## Requirements

- macOS 12.0 (Monterey) or later
- Rust 1.85+ (for building from source)

## Support

For issues and feature requests, please open an issue on GitHub.
EOF

# Create uninstall script
cat > "$DIST_DIR/uninstall.sh" <<'UNINSTALL_EOF'
#!/usr/bin/env bash
#
# Uninstall push-to-talk
#

set -euo pipefail

APP_NAME="push-to-talk"
INSTALL_DIR="/Applications"

echo "🗑  Uninstalling $APP_NAME..."

# Remove app bundle
if [[ -d "$INSTALL_DIR/${APP_NAME}.app" ]]; then
    rm -rf "$INSTALL_DIR/${APP_NAME}.app"
    echo "✓ Removed $INSTALL_DIR/${APP_NAME}.app"
else
    echo "⚠  App not found in $INSTALL_DIR"
fi

# Remove config
if [[ -d ~/.push-to-talk ]]; then
    rm -rf ~/.push-to-talk
    echo "✓ Removed ~/.push-to-talk"
fi

echo ""
echo "Note: You may also want to remove permissions:"
echo "  System Preferences → Privacy & Security → Accessibility"
echo "  System Preferences → Privacy & Security → Microphone"
UNINSTALL_EOF

chmod +x "$DIST_DIR/uninstall.sh"

# Create DMG (if hdiutil is available)
if command -v hdiutil &> /dev/null; then
    echo "📀 Creating DMG..."
    DMG_FILE="$DIST_DIR/${APP_NAME}-v${VERSION}.dmg"
    
    # Create temporary DMG
    hdiutil create -volname "$APP_NAME" \
        -srcfolder "$DIST_DIR" \
        -ov -format UDZO \
        "$DMG_FILE" 2>/dev/null || {
        echo "⚠  DMG creation failed, skipping..."
    }
    
    if [[ -f "$DMG_FILE" ]]; then
        echo "✓ Created: $DMG_FILE"
    fi
fi

# Create checksums
echo "📝 Creating checksums..."
cd "$DIST_DIR"
find . -type f ! -name "*.sha256" ! -name "*.dmg.tmp" -exec shasum -a 256 {} \; > CHECKSUMS.txt

echo ""
echo "✅ Build complete!"
echo ""
echo "Distribution files:"
ls -lh "$DIST_DIR"
echo ""
echo "Checksums:"
cat "$DIST_DIR/CHECKSUMS.txt"
