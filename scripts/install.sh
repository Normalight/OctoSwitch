#!/usr/bin/env bash
set -euo pipefail

APP_NAME="OctoSwitch.app"
DMG_NAME="OctoSwitch_*_x64.dmg"

echo "=== OctoSwitch macOS Installer ==="
echo ""

# 1. Find the .app
APP_PATH=""
if [ -d "/Applications/$APP_NAME" ]; then
    APP_PATH="/Applications/$APP_NAME"
elif [ -d "$HOME/Applications/$APP_NAME" ]; then
    APP_PATH="$HOME/Applications/$APP_NAME"
elif [ -d "./$APP_NAME" ]; then
    APP_PATH="./$APP_NAME"
fi

# 2. Try to find and mount DMG
if [ -z "$APP_PATH" ]; then
    # Search for DMG in current dir or Downloads
    DMG_PATH=$(find . ~/Downloads -maxdepth 2 -name "$DMG_NAME" -print -quit 2>/dev/null || true)
    if [ -n "$DMG_PATH" ]; then
        echo "[1/3] Mounting DMG..."
        VOLUME=$(hdiutil attach "$DMG_PATH" -nobrowse -readonly | grep /Volumes/ | awk '{print $3}')
        if [ -d "$VOLUME/$APP_NAME" ]; then
            echo "[2/3] Copying to /Applications..."
            cp -R "$VOLUME/$APP_NAME" /Applications/
            hdiutil detach "$VOLUME" -quiet
            APP_PATH="/Applications/$APP_NAME"
        fi
    fi
fi

if [ -z "$APP_PATH" ]; then
    echo "Error: $APP_NAME not found."
    echo ""
    echo "Usage:"
    echo "  1. Download the OctoSwitch DMG from GitHub Releases"
    echo "  2. Run this script in the same directory as the DMG"
    echo "  OR"
    echo "  3. Manually copy OctoSwitch.app to /Applications/ first"
    exit 1
fi

echo "Found app at: $APP_PATH"

# 3. Remove quarantine (Gatekeeper bypass)
echo "[3/3] Removing quarantine attribute..."
xattr -cr "$APP_PATH" 2>/dev/null || true
# Also remove com.apple.quarantine for all nested files
find "$APP_PATH" -type f -name "*.dylib" -o -name "OctoSwitch" | while read -r f; do
    xattr -d com.apple.quarantine "$f" 2>/dev/null || true
done

# Self-sign with ad-hoc signature (allows local execution without notarization)
codesign --force --deep --sign - "$APP_PATH" 2>/dev/null || true

echo ""
echo "Done! Launching OctoSwitch..."
open "$APP_PATH"

echo ""
echo "Tip: If macOS still blocks it, go to System Settings > Privacy & Security"
echo "     and click 'Open Anyway' at the bottom."
