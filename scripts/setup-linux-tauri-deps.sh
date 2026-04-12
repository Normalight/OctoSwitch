#!/usr/bin/env bash
set -euo pipefail

echo "Installing Linux dependencies for Tauri build..."

if command -v apt-get &>/dev/null; then
    echo "Detected Debian/Ubuntu-based system..."
    sudo apt-get update
    sudo apt-get install -y \
        pkg-config \
        build-essential \
        libglib2.0-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.1-dev \
        libsoup-3.0-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        patchelf
elif command -v dnf &>/dev/null; then
    echo "Detected Fedora/RHEL-based system..."
    sudo dnf install -y \
        pkgconfig \
        gcc \
        gcc-c++ \
        make \
        glib2-devel \
        gtk3-devel \
        webkit2gtk4.1-devel \
        libsoup3-devel \
        libappindicator-gtk3-devel \
        librsvg2-devel \
        patchelf
elif command -v pacman &>/dev/null; then
    echo "Detected Arch-based system..."
    sudo pacman -Syu --noconfirm \
        pkg-config \
        base-devel \
        glib2 \
        gtk3 \
        webkit2gtk-4.1 \
        libsoup3 \
        libappindicator-gtk3 \
        librsvg \
        patchelf
else
    echo "ERROR: Unsupported package manager."
    echo "Please install the equivalent of: pkg-config, build tools, GTK3,"
    echo "webkit2gtk-4.1, libsoup3, appindicator, librsvg, and patchelf."
    exit 1
fi

echo ""
echo "Done. You can now run:"
echo "  source \"\$HOME/.cargo/env\""
echo "  cd src-tauri && cargo check"
