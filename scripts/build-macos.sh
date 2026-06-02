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

# Verify binary was built
if [[ ! -f "$PROJECT_ROOT/target/release/$APP_NAME" ]]; then
    echo "❌ Error: Binary not found at target/release/$APP_NAME"
    exit 1
fi

echo "✓ Binary built successfully"

# Create app bundle structure
echo "📦 Creating app bundle..."
APP_BUNDLE="$DIST_DIR/${APP_NAME}.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks"

# Copy binary
cp "$PROJECT_ROOT/target/release/$APP_NAME" "$APP_BUNDLE/Contents/MacOS/"
chmod +x "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

# Convert logo to app icon (if logo.png exists)
if [[ -f "$PROJECT_ROOT/logo.png" ]]; then
    echo "🎨 Converting logo to app icon..."
    ICONSET="$APP_BUNDLE/Contents/Resources/icon.iconset"
    mkdir -p "$ICONSET"
    
    # Generate all required icon sizes
    sips -z 16 16 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_16x16.png" >/dev/null 2>&1
    sips -z 32 32 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_16x16@2x.png" >/dev/null 2>&1
    sips -z 32 32 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_32x32.png" >/dev/null 2>&1
    sips -z 64 64 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_32x32@2x.png" >/dev/null 2>&1
    sips -z 128 128 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_128x128.png" >/dev/null 2>&1
    sips -z 256 256 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_128x128@2x.png" >/dev/null 2>&1
    sips -z 256 256 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_256x256.png" >/dev/null 2>&1
    sips -z 512 512 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_256x256@2x.png" >/dev/null 2>&1
    sips -z 512 512 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_512x512.png" >/dev/null 2>&1
    sips -z 1024 1024 "$PROJECT_ROOT/logo.png" --out "$ICONSET/icon_512x512@2x.png" >/dev/null 2>&1
    
    # Convert iconset to icns
    if command -v iconutil &> /dev/null; then
        iconutil -c icns "$ICONSET" -o "$APP_BUNDLE/Contents/Resources/app.icns" 2>/dev/null
        rm -rf "$ICONSET"
        echo "✓ App icon created: app.icns"
    else
        # Fallback: just copy the 512x512 version
        sips -z 512 512 "$PROJECT_ROOT/logo.png" --out "$APP_BUNDLE/Contents/Resources/app.icns" >/dev/null 2>&1 || true
        rm -rf "$ICONSET"
    fi
fi

# Create Info.plist with required permissions
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
    <key>CFBundleIconFile</key>
    <string>app</string>
    <key>NSMicrophoneUsageDescription</key>
    <string>This app requires microphone access to record audio for transcription.</string>
    <key>NSAppleEventsUsageDescription</key>
    <string>This app requires accessibility access to detect global hotkeys and type transcribed text.</string>
    <key>NSAccessibilityUsageDescription</key>
    <string>This app requires accessibility access to detect global hotkeys and type transcribed text into active windows.</string>
    <key>NSSystemPolicyAllFiles</key>
    <string>This app needs full disk access to read configuration and write logs.</string>
    <key>LSUIElement</key>
    <false/>
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
    echo ""
    echo "💡 Tip: If you downloaded from DMG, drag ${APP_NAME}.app to /Applications"
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

# Verify app bundle structure
echo "🔍 Verifying app bundle..."
if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "❌ Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi
if [[ ! -x "$APP_BUNDLE/Contents/MacOS/$APP_NAME" ]]; then
    echo "❌ Error: Binary not executable in app bundle"
    exit 1
fi
echo "✓ App bundle verified: $APP_BUNDLE"

# Create DMG (if hdiutil is available)
DMG_SOURCE="$DIST_DIR/.dmg-source"
rm -rf "$DMG_SOURCE"
mkdir -p "$DMG_SOURCE"

# Copy only app bundle to DMG source
cp -R "$APP_BUNDLE" "$DMG_SOURCE/"

# Create DMG (if hdiutil is available)
if command -v hdiutil &> /dev/null; then
    echo "📀 Creating DMG..."
    DMG_FILE="$DIST_DIR/${APP_NAME}-v${VERSION}.dmg"
    DMG_TEMP="$DIST_DIR/.temp.dmg"
    DMG_RW="$DIST_DIR/.rw.dmg"
    
    # Clean up any previous attempts
    rm -f "$DMG_FILE" "$DMG_TEMP" "$DMG_RW" 2>/dev/null
    
    # Step 1: Create initial DMG from source folder
    echo "  Creating initial DMG..."
    if ! hdiutil create -volname "$APP_NAME" \
        -srcfolder "$DMG_SOURCE" \
        -ov -format UDZO \
        "$DMG_TEMP" 2>&1; then
        echo "⚠  Failed to create initial DMG"
        rm -f "$DMG_TEMP"
    fi
    
    # Step 2: Create DMG with custom layout (Applications symlink)
    if [[ -f "$DMG_TEMP" ]]; then
        echo "  Adding Applications symlink and layout..."
        
        # Convert to read/write
        hdiutil convert "$DMG_TEMP" -format UDRW -o "$DMG_RW" >/dev/null 2>&1
        
        if [[ -f "$DMG_RW" ]]; then
            # Mount the DMG
            MOUNT_POINT="/Volumes/$APP_NAME"
            hdiutil detach "$MOUNT_POINT" 2>/dev/null || true
            
            if hdiutil attach "$DMG_RW" -mountpoint "$MOUNT_POINT" -nobrowse 2>/dev/null; then
                # Create Applications symlink
                ln -s /Applications "$MOUNT_POINT/Applications"
                
                # Hide unwanted files
                chflags hidden "$MOUNT_POINT/.fseventsd" 2>/dev/null || true
                
                # Use AppleScript to set custom view (no background image)
                osascript -e "
                tell application \"Finder\"
                    tell disk \"$APP_NAME\"
                        open
                        set current view of container window to icon view
                        set toolbar visible of container window to false
                        set statusbar visible of container window to false
                        set bounds of container window to {400, 100, 900, 450}
                        set theViewOptions to the icon view options of container window
                        set arrangement of theViewOptions to not arranged
                        set icon size of theViewOptions to 80
                        set position of item \"push-to-talk.app\" of container window to {150, 180}
                        set position of item \"Applications\" of container window to {350, 180}
                        close
                    end tell
                end tell
                " 2>/dev/null || echo "  ⚠  AppleScript customization skipped (CI environment)"
                
                sleep 2
                hdiutil detach "$MOUNT_POINT" 2>/dev/null || true
                
                # Convert back to read-only compressed
                hdiutil convert "$DMG_RW" -format UDZO -o "$DMG_FILE" >/dev/null 2>&1
                if [[ -f "$DMG_FILE" ]]; then
                    echo "✓ Created: $DMG_FILE (with custom layout)"
                else
                    cp "$DMG_RW" "$DMG_FILE"
                    echo "✓ Created: $DMG_FILE (read/write)"
                fi
                rm -f "$DMG_RW"
            else
                cp "$DMG_RW" "$DMG_FILE"
                echo "✓ Created: $DMG_FILE (without customization)"
                rm -f "$DMG_RW"
            fi
        else
            cp "$DMG_TEMP" "$DMG_FILE"
            echo "✓ Created: $DMG_FILE (standard)"
        fi
        
        # Cleanup
        rm -f "$DMG_TEMP" 2>/dev/null
    elif [[ -f "$DMG_TEMP" ]]; then
        mv "$DMG_TEMP" "$DMG_FILE"
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
