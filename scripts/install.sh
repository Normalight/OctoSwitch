#!/usr/bin/env bash
set -euo pipefail

# OctoSwitch Universal Installer
# Detects platform, downloads latest release, installs, and bypasses macOS Gatekeeper.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Normalight/OctoSwitch/main/scripts/install.sh | bash
#   or
#   bash install.sh [VERSION]

REPO="Normalight/OctoSwitch"
APP_NAME="OctoSwitch"
VERSION="${1:-latest}"

# ── Platform detection ──
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)  PLATFORM="macos" ;;
    Linux)   PLATFORM="linux" ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# ── Architecture normalization ──
case "$ARCH" in
    x86_64|amd64) ARCH_NORM="x64" ;;
    arm64|aarch64) ARCH_NORM="arm64" ;;
    *)            ARCH_NORM="$ARCH" ;;
esac

echo "=== OctoSwitch Installer ==="
echo "Platform: $PLATFORM / $ARCH_NORM"
echo ""

# ── GitHub API: get latest release info ──
get_release_json() {
    local tag="$1"
    local url
    if [ "$tag" = "latest" ]; then
        url="https://api.github.com/repos/$REPO/releases/latest"
    else
        url="https://api.github.com/repos/$REPO/releases/tags/$tag"
    fi

    curl -fsSL "$url" \
        -H "Accept: application/vnd.github+json" \
        -H "X-GitHub-Api-Version: 2022-11-28" \
        2>/dev/null
}

RELEASE_JSON="$(get_release_json "$VERSION")" || {
    echo "Failed to fetch release info. GitHub API may be rate-limited."
    echo "Try installing gh CLI and logging in, or specify a version tag directly."
    exit 1
}

REAL_TAG=$(echo "$RELEASE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])" 2>/dev/null)
echo "Latest version: $REAL_TAG"

# ── Find the right asset ──
download_url() {
    # args: platform, arch_norm
    echo "$RELEASE_JSON" | python3 -c "
import sys, json
r = json.load(sys.stdin)
platform = sys.argv[1]
arch = sys.argv[2]

# macOS: prefer arm64 DMG, fallback to x64 DMG
# Linux: prefer AppImage, fallback to deb
# Windows: prefer msi, fallback to exe

patterns = {
    'macos': ['.app.tar.gz', '_' + arch + '.dmg', '_x64.dmg', '.dmg'],
    'linux': ['_' + arch + '.AppImage', '_x64.AppImage', '.AppImage', '.deb'],
    'windows': ['_' + arch + '.msi', '_x64.msi', '.msi', '.exe'],
}

for pat in patterns.get(platform, []):
    for a in r.get('assets', []):
        name = a['name']
        if pat in name:
            print(a['browser_download_url'])
            sys.exit(0)
print('')
" "$PLATFORM" "$ARCH_NORM" 2>/dev/null
}

DOWNLOAD_URL="$(download_url)"

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: No matching asset found for $PLATFORM/$ARCH_NORM"
    echo ""
    echo "Available assets:"
    echo "$RELEASE_JSON" | python3 -c "import sys,json; r=json.load(sys.stdin); [print(f'  {a[\"name\"]}') for a in r.get('assets',[])]"
    exit 1
fi

ASSET_NAME=$(basename "$DOWNLOAD_URL")
echo "Downloading: $ASSET_NAME"

# ── Download asset ──
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

curl -fsSL -o "$TEMP_DIR/$ASSET_NAME" "$DOWNLOAD_URL" || {
    echo "Download failed. Try again or check your network."
    exit 1
}

echo "Downloaded to: $TEMP_DIR/$ASSET_NAME"

# ── Platform-specific installation ──
case "$PLATFORM" in
    macos)
        install_macos() {
            local asset="$1"
            local app_path=""

            # Handle .dmg
            if [[ "$asset" == *.dmg ]]; then
                echo "[1/3] Mounting DMG..."
                VOLUME=$(hdiutil attach "$asset" -nobrowse -readonly | grep /Volumes/ | awk '{print $NF}')
                APP_NAME_DMG=$(ls "$VOLUME" | grep '\.app$' | head -1)
                if [ -n "$APP_NAME_DMG" ]; then
                    echo "[2/3] Copying $APP_NAME_DMG to /Applications..."
                    if [ -d "/Applications/$APP_NAME_DMG" ]; then
                        rm -rf "/Applications/$APP_NAME_DMG"
                    fi
                    cp -R "$VOLUME/$APP_NAME_DMG" /Applications/
                    hdiutil detach "$VOLUME" -quiet
                    app_path="/Applications/$APP_NAME_DMG"
                else
                    hdiutil detach "$VOLUME" -quiet
                    echo "Error: No .app found in DMG"
                    exit 1
                fi
            elif [[ "$asset" == *.tar.gz ]]; then
                echo "[1/2] Extracting tarball..."
                tar -xzf "$asset" -C /Applications/
                app_path=$(ls -d /Applications/OctoSwitch*.app 2>/dev/null | head -1)
            fi

            if [ -z "$app_path" ] || [ ! -d "$app_path" ]; then
                echo "Error: Failed to install app."
                exit 1
            fi

            # ── Bypass Gatekeeper ──
            echo "[3/3] Removing quarantine and self-signing..."
            xattr -cr "$app_path" 2>/dev/null || true
            # Deep quarantine removal for all binaries
            find "$app_path" -type f \( -perm +111 -o -name "*.dylib" -o -name "*.so" \) | while read -r f; do
                xattr -d com.apple.quarantine "$f" 2>/dev/null || true
            done
            codesign --force --deep --sign - "$app_path" 2>/dev/null || true

            echo ""
            echo "Installed: $app_path"
            echo "Launching OctoSwitch..."
            open "$app_path"
        }
        install_macos "$TEMP_DIR/$ASSET_NAME"
        ;;

    linux)
        install_linux() {
            local asset="$1"

            if [[ "$asset" == *.AppImage ]]; then
                echo "[1/2] Installing AppImage..."
                mkdir -p ~/.local/bin
                cp "$asset" ~/.local/bin/OctoSwitch.AppImage
                chmod +x ~/.local/bin/OctoSwitch.AppImage

                # Create desktop entry
                mkdir -p ~/.local/share/applications
                cat > ~/.local/share/applications/octoswitch.desktop << EOF
[Desktop Entry]
Name=OctoSwitch
Comment=Local model routing gateway
Exec=$HOME/.local/bin/OctoSwitch.AppImage
Type=Application
Categories=Utility;
EOF
                echo "Installed to: ~/.local/bin/OctoSwitch.AppImage"
                echo "Launching..."
                ~/.local/bin/OctoSwitch.AppImage &
            elif [[ "$asset" == *.deb ]]; then
                echo "[1/2] Installing .deb..."
                sudo dpkg -i "$asset" || sudo apt-get install -f -y
                echo "Launching OctoSwitch..."
                octoswitch &
            else
                echo "Unsupported Linux package format: $asset"
                echo "Please install manually."
                exit 1
            fi
        }
        install_linux "$TEMP_DIR/$ASSET_NAME"
        ;;

    windows)
        echo "Downloaded: $TEMP_DIR/$ASSET_NAME"
        echo ""
        echo "On Windows, run the installer manually:"
        echo "  start $TEMP_DIR\\\\$ASSET_NAME"
        cmd.exe /c "start $TEMP_DIR\\$ASSET_NAME" 2>/dev/null || true
        ;;
esac

echo ""
echo "Tip: If macOS still blocks the app, go to System Settings → Privacy & Security"
echo "     and click 'Open Anyway' for OctoSwitch."
