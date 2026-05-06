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

# ── Proxy support ──
CURL_PROXY=""
if [ -n "${HTTP_PROXY:-}" ] || [ -n "${http_proxy:-}" ]; then
    P="${HTTP_PROXY:-${http_proxy:-}}"
    CURL_PROXY="--proxy $P"
fi

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

    curl -fsSL --connect-timeout 10 $CURL_PROXY "$url" \
        -H "Accept: application/vnd.github+json" \
        -H "X-GitHub-Api-Version: 2022-11-28" \
        2>/dev/null
}

RELEASE_JSON="$(get_release_json "$VERSION")" || true
REAL_TAG=""

if [ -n "$RELEASE_JSON" ]; then
    REAL_TAG=$(echo "$RELEASE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])" 2>/dev/null) || true
fi

# ── Fallback: no API → construct URL from known tag ──
if [ -z "$REAL_TAG" ]; then
    if [ "$VERSION" = "latest" ] || [ -z "$VERSION" ]; then
        # Try git ls-remote for latest tag (no API rate limit)
        REAL_TAG=$(git ls-remote --tags --refs "https://github.com/$REPO.git" 2>/dev/null \
            | grep -o 'v[0-9.]*$' | sort -V | tail -1) || true
    else
        REAL_TAG="$VERSION"
    fi

    if [ -z "$REAL_TAG" ]; then
        echo "Failed to determine latest version."
        echo ""
        echo "Options:"
        echo "  1. Specify a version:  bash install.sh v0.4.4"
        echo "  2. Install gh CLI:     brew install gh && gh auth login"
        echo "  3. Or wait and retry (GitHub API rate limit resets hourly)"
        exit 1
    fi

    echo "(fallback mode) Using tag: $REAL_TAG"
else
    echo "Latest version: $REAL_TAG"
fi

# ── Direct URL construction (no API needed) ──
# GitHub release download URL pattern:
#   https://github.com/{owner}/{repo}/releases/download/{tag}/{asset}
# Tauri asset naming:
#   macOS:   OctoSwitch_{arch}.dmg or OctoSwitch.app.tar.gz
#   Linux:   OctoSwitch_{arch}.AppImage or OctoSwitch_{arch}.deb
#   Windows: OctoSwitch_{arch}.msi or OctoSwitch_{arch}.exe
#   (version in filename may vary; try both with and without version prefix)

construct_asset_patterns() {
    local plat="$1" arch="$2" tag="$3"
    local ver="${tag#v}"  # v0.4.4 → 0.4.4

    case "$plat" in
        macos)
            # Tauri may use either aarch64 or arm64 in asset names
            local arch2="aarch64"
            echo "OctoSwitch_${ver}_${arch2}.dmg"
            echo "OctoSwitch_${ver}_${arch}.dmg"
            echo "OctoSwitch_${arch2}.app.tar.gz"
            echo "OctoSwitch_${arch}.dmg"
            echo "OctoSwitch.app.tar.gz"
            ;;
        linux)
            echo "OctoSwitch_${ver}_${arch}.AppImage"
            echo "OctoSwitch_${arch}.AppImage"
            echo "OctoSwitch_${ver}_amd64.deb"
            echo "OctoSwitch_amd64.deb"
            ;;
        windows)
            echo "OctoSwitch_${ver}_${arch}.msi"
            echo "OctoSwitch_${arch}.msi"
            echo "OctoSwitch_${ver}_${arch}.exe"
            echo "OctoSwitch_${arch}.exe"
            ;;
    esac
}

DOWNLOAD_URL=""
if [ -n "$RELEASE_JSON" ]; then
    # Use API to find exact asset
    DOWNLOAD_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
r = json.load(sys.stdin)
platform = sys.argv[1]
arch = sys.argv[2]

patterns = {
    'macos': ['.app.tar.gz', '_aarch64.dmg', '_arm64.dmg', '_x64.dmg', '.dmg'],
    'linux': ['_aarch64.AppImage', '_arm64.AppImage', '_x64.AppImage', '.AppImage', '_aarch64.deb', '_arm64.deb', '_amd64.deb', '.deb'],
    'windows': ['_x64-setup.exe', '_x64.msi', '_x64.exe', '.msi', '.exe'],
}

for pat in patterns.get(platform, []):
    for a in r.get('assets', []):
        name = a['name']
        if pat in name:
            print(a['browser_download_url'])
            sys.exit(0)
" "$PLATFORM" "$ARCH_NORM" 2>/dev/null)
fi

# Fallback: try direct download URL patterns
if [ -z "$DOWNLOAD_URL" ]; then
    for PAT in $(construct_asset_patterns "$PLATFORM" "$ARCH_NORM" "$REAL_TAG"); do
        URL="https://github.com/$REPO/releases/download/$REAL_TAG/$PAT"
        HTTP_CODE=$(curl -sL -o /dev/null -w "%{http_code}" --connect-timeout 5 $CURL_PROXY "$URL" 2>/dev/null || echo "000")
        if [ "$HTTP_CODE" = "302" ] || [ "$HTTP_CODE" = "200" ]; then
            DOWNLOAD_URL="$URL"
            break
        fi
    done
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: No matching asset found for $PLATFORM/$ARCH_NORM"
    echo ""
    if [ -n "$RELEASE_JSON" ]; then
        echo "Available assets:"
        echo "$RELEASE_JSON" | python3 -c "import sys,json; r=json.load(sys.stdin); [print(f'  {a[\"name\"]}') for a in r.get('assets',[])]"
    else
        echo "Tip: GitHub API unavailable (rate limit?). Retry later or use gh CLI."
    fi
    exit 1
fi

ASSET_NAME=$(basename "$DOWNLOAD_URL")
echo "Downloading: $ASSET_NAME"

# ── Download asset ──
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

curl -fsSL $CURL_PROXY -o "$TEMP_DIR/$ASSET_NAME" "$DOWNLOAD_URL" || {
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
                # hdiutil outputs volume info to stderr; capture both streams.
                # Parse the /Volumes path using the same logic as the Rust updater:
                # split each line by tab, take the last field (the mount path).
                VOLUME=$(hdiutil attach "$asset" -nobrowse -readonly 2>&1 \
                    | grep '/Volumes/' \
                    | head -1 \
                    | rev | cut -f1 | rev)
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
            # Deep quarantine removal for all files
            find "$app_path" -type f | while read -r f; do
                xattr -d com.apple.quarantine "$f" 2>/dev/null || true
            done
            codesign --force --deep --sign - "$app_path" 2>/dev/null || true

            echo ""
            echo "Installed: $app_path"

            # If already running, kill old instance so the new binary takes effect
            if pgrep -x "$APP_NAME" > /dev/null 2>&1; then
                echo "Closing old instance..."
                pkill -x "$APP_NAME" 2>/dev/null || true
                sleep 1
            fi

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
