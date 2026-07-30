#!/usr/bin/env bash
# Installs the mpv dev package needed for `cargo build --features mpv` on
# Linux/macOS. Unlike Windows, no vendoring is needed here: apt/brew ship a
# proper libmpv (headers + import lib) directly, and pkg-config finds it.
set -euo pipefail

if command -v apt-get >/dev/null 2>&1; then
    echo "Installing libmpv-dev via apt..."
    sudo apt-get update
    sudo apt-get install -y libmpv-dev
elif command -v brew >/dev/null 2>&1; then
    echo "Installing mpv via Homebrew..."
    brew install mpv
elif command -v pacman >/dev/null 2>&1; then
    echo "Installing mpv via pacman..."
    sudo pacman -S --needed mpv
else
    echo "No supported package manager found (apt, brew, pacman)." >&2
    echo "Install libmpv development files manually, see README.md." >&2
    exit 1
fi

echo "Done. Build with: cargo build --release -p termusic-server --features mpv"
