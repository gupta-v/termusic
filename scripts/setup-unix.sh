#!/usr/bin/env bash
# One-shot setup for building & running termusic (with the mpv backend) on Linux/macOS,
# starting from a fresh clone of this repo. Mirrors scripts/setup-windows.ps1 - see
# documentation/setup-docs.md for the full writeup.
#
# Safe to re-run: every step checks whether it's already done before acting.
#
# Usage: ./scripts/setup-unix.sh [music_dir]
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$repo_root/dist"

confirm() {
    read -r -p "$1 (y/n) " answer
    [[ "$answer" =~ ^[Yy] ]]
}

echo "=== termusic Unix setup ==="

# --- 1. Rust ---------------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust isn't installed."
    if confirm "Install Rust (via rustup) now?"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    else
        echo "Rust is required. Aborting." >&2
        exit 1
    fi
else
    echo "Rust: OK"
fi

# --- 2. protoc + C build tools + mpv dev files ------------------------------
if ! command -v protoc >/dev/null 2>&1 || ! command -v pkg-config >/dev/null 2>&1; then
    echo "protoc / build tools / libmpv-dev aren't all installed."
    if confirm "Install missing packages now?"; then
        if command -v apt-get >/dev/null 2>&1; then
            sudo apt-get update
            sudo apt-get install -y protobuf-compiler pkg-config build-essential libmpv-dev
        elif command -v brew >/dev/null 2>&1; then
            xcode-select --install 2>/dev/null || true
            brew install protobuf pkg-config mpv
        elif command -v pacman >/dev/null 2>&1; then
            sudo pacman -S --needed protobuf pkgconf base-devel mpv
        else
            echo "No supported package manager found (apt, brew, pacman)." >&2
            echo "Install protoc, pkg-config, a C toolchain, and libmpv-dev manually." >&2
            exit 1
        fi
    else
        echo "protoc and libmpv-dev are required. Aborting." >&2
        exit 1
    fi
else
    echo "protoc / pkg-config: OK"
    # protoc/pkg-config being present doesn't guarantee libmpv-dev is - run the
    # dedicated script too, it's a no-op cost-wise if already installed.
    "$repo_root/scripts/setup-mpv-unix.sh"
fi

# --- 3. Build ----------------------------------------------------------------
echo "Building termusic-server (with mpv backend)..."
cargo build --release --manifest-path "$repo_root/Cargo.toml" -p termusic-server --features mpv

echo "Building termusic (TUI client)..."
cargo build --release --manifest-path "$repo_root/Cargo.toml" -p termusic --features cover-viuer-sixel

# --- 4. Collect a portable dist/ folder --------------------------------------
mkdir -p "$dist_dir"
cp -f "$repo_root/target/release/termusic" "$dist_dir/termusic"
cp -f "$repo_root/target/release/termusic-server" "$dist_dir/termusic-server"
echo "Binaries ready in $dist_dir"

# --- 5. Music library folder -------------------------------------------------
music_dir="${1:-}"
if [ -z "$music_dir" ]; then
    read -r -p 'Path to your music folder (in quotes if it has spaces), or press Enter to use ~/Music: ' music_dir
fi
music_dir="${music_dir%\"}"
music_dir="${music_dir#\"}"
if [ -z "$music_dir" ]; then
    music_dir="$HOME/Music"
fi
mkdir -p "$music_dir"
echo "Library folder: $music_dir (you can change this later in termusic's config editor, key: Shift+C)"

# --- 6. Write server.toml (music_dirs + mpv backend) -------------------------
if [ "$(uname)" = "Darwin" ]; then
    config_dir="$HOME/Library/Application Support/termusic"
else
    config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/termusic"
fi
config_path="$config_dir/server.toml"
mkdir -p "$config_dir"

if [ ! -f "$config_path" ]; then
    # Let the server generate its own full set of defaults first, so this script never has
    # to hand-maintain a duplicate of every config field.
    echo "Generating default config..."
    "$dist_dir/termusic-server" &
    server_pid=$!
    sleep 2
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
fi

sed -i.bak -E \
    -e "s#music_dirs[[:space:]]*=[[:space:]]*\[[^]]*\]#music_dirs = ['$music_dir']#" \
    -e 's#backend[[:space:]]*=[[:space:]]*"[^"]*"#backend = "mpv"#' \
    "$config_path"
rm -f "$config_path.bak"

echo ""
echo "=== Setup complete ==="
echo "Run: $dist_dir/termusic"
echo "(termusic-server starts automatically alongside it - no need to run it separately.)"
