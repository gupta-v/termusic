# Setup

One command gets you from a fresh clone to a running `termusic` with the `mpv` playback
backend enabled, on Windows, Linux, or macOS. Everything is idempotent — re-running the
script only does what's still missing.

## Quick start

**Windows:**

```powershell
git clone https://github.com/gupta-v/termusic.git
cd termusic
.\scripts\setup-windows.ps1
```

**Linux / macOS:**

```bash
git clone https://github.com/gupta-v/termusic.git
cd termusic
./scripts/setup-unix.sh
```

You'll be prompted (y/n) before anything gets installed, and at the end for your music
folder:

```
Paste the path to your music folder in quotes ("D:\Music"), or press Enter to use your default Music folder
```

Press Enter to use your OS's default Music folder (`%USERPROFILE%\Music` on Windows,
`~/Music` on Linux/macOS), or paste a specific folder (with or without quotes — both are
accepted).

When it finishes, run:

```
./dist/termusic        # Linux/macOS
.\dist\termusic.exe     # Windows
```

`termusic-server` starts automatically alongside it — you never run it directly.

## What the script does (Windows)

1. **Scoop** — checks for [Scoop](https://scoop.sh) (a no-admin-rights package manager for
   Windows); offers to install it if missing.
2. **Rust** — checks for `cargo`; offers to install via `scoop install rustup` +
   `rustup default stable-msvc` (the MSVC toolchain, not GNU — required for the linker step
   below to work).
3. **protoc** — checks for the Protocol Buffers compiler (needed to build termusic's gRPC
   client/server layer); offers `scoop install protobuf`.
4. **MSVC C++ Build Tools** — checks (via `vswhere`) for the actual linker (`link.exe`)
   Rust needs on Windows; if missing, offers to install it via
   `winget install Microsoft.VisualStudio.2022.BuildTools` with the C++ workload. This step
   is a multi-GB download and can take a while — it's the only slow step.
5. **libmpv vendor files** — runs `scripts/setup-mpv-windows.ps1` if
   `vendor/mpv-windows/64/mpv.lib` isn't already present. See below for why this exists as
   its own step.
6. **Build** —
   ```powershell
   cargo build --release -p termusic-server --features mpv
   cargo build --release -p termusic --features cover-viuer-sixel
   ```
   (`cover-viuer-sixel` gives sharper cover art if your terminal supports the Sixel graphics
   protocol; it silently falls back to a lower-resolution ANSI renderer otherwise.)
7. **Collect a portable folder** — copies `termusic.exe`, `termusic-server.exe`, and
   `libmpv-2.dll` into `dist\`, so the two binaries + their shared library live together
   (required — `termusic.exe` looks for `termusic-server.exe` next to itself).
8. **Music folder prompt** — see Quick start above.
9. **Config** — if `%APPDATA%\termusic\server.toml` doesn't exist yet, briefly launches
   `termusic-server.exe` once (headless, killed after ~2s) purely so it writes out its own
   full set of defaults — the script never hand-maintains a duplicate copy of every config
   field, only patches two lines afterward:
   ```toml
   [player]
   music_dirs = ['<your folder>']
   backend = "mpv"
   ```
   You can change the library folder later from inside termusic (`Shift+C` opens the config
   editor) without re-running any of this.

## What the script does (Linux/macOS)

Much simpler than Windows — apt/brew/pacman ship a proper `libmpv` (headers + import lib)
directly, so there's no vendoring, no Cloudflare bot-check, no MinGW/MSVC mismatch to work
around.

1. **Rust** — checks for `cargo`; offers to install via the official `rustup.rs` installer.
2. **protoc / build tools / libmpv-dev** — checks for `protoc` and `pkg-config`; if either
   is missing, installs both plus a C toolchain and `libmpv-dev`/`mpv` in one shot via
   whichever of apt/brew/pacman is available. If they're already present, still runs
   `scripts/setup-mpv-unix.sh` on its own (cheap no-op if already installed) to make sure
   the mpv dev package specifically landed.
3. **Build** — same two `cargo build` commands as Windows (see above), just without `-p
   termusic-server`/`-p termusic` needing any vendor path.
4. **Collect a portable folder** — copies `termusic` and `termusic-server` into `dist/`
   (no DLL to bundle — libmpv is a normal system library here).
5. **Music folder prompt** — see Quick start above.
6. **Config** — same approach as Windows: briefly runs `termusic-server` once if
   `~/.config/termusic/server.toml` (or `~/Library/Application Support/termusic/server.toml`
   on macOS) doesn't exist yet, so it writes its own defaults, then patches just
   `music_dirs` and `backend = "mpv"`.

## Why libmpv needs its own script (Windows only)

Upstream's official Windows mpv builds
([sourceforge.net/projects/mpv-player-windows](https://sourceforge.net/projects/mpv-player-windows))
have two problems for this project specifically:

- The download page is behind a Cloudflare bot-check that blocks non-browser requests
  (`curl`, `Invoke-WebRequest`, CI runners all get HTTP 403).
- The import library they ship (`libmpv.dll.a`) is in **MinGW** format, which MSVC's
  `link.exe` (what `rustup default stable-msvc` uses) cannot read at all.

Rather than requiring every user to manually fix this (extract the DLL's real exports with
`dumpbin`, hand-write a `.def` file, regenerate an MSVC-format `.lib` with `lib.exe`), that
pre-processed package is hosted once, on a GitHub Release
(`gupta-v/termusic` → release `mpv-windows-deps-v1`), and `setup-mpv-windows.ps1` just pulls
it — a plain HTTPS GitHub download, no bot-check, no manual steps. It's not committed
directly into the repo because the DLL alone is ~120MB and would bloat git history for
everyone who ever clones this project, whether they need the mpv backend or not.

## Manual fallback

If you'd rather not run the script (or it fails partway and you want to finish by hand),
each step above is just the underlying command — run them in order:

```powershell
# 1. Scoop (skip if already installed)
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
Invoke-RestMethod get.scoop.sh | Invoke-Expression

# 2. Rust (MSVC toolchain)
scoop install rustup
rustup default stable-msvc

# 3. protoc
scoop install protobuf

# 4. MSVC C++ Build Tools
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# 5. libmpv vendor files
.\scripts\setup-mpv-windows.ps1

# 6. Build
cargo build --release -p termusic-server --features mpv
cargo build --release -p termusic --features cover-viuer-sixel

# 7. Binaries end up in target\release\ - copy what you need next to each other:
#    termusic.exe, termusic-server.exe, libmpv-2.dll
```

Then edit `%APPDATA%\termusic\server.toml` by hand (create it by running
`termusic-server.exe` once first if it doesn't exist yet):

```toml
[player]
music_dirs = ['C:\Path\To\Your\Music']
backend = "mpv"
```

### Manual fallback (Linux/macOS)

```bash
# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. protoc, build tools, and libmpv-dev
sudo apt-get install -y protobuf-compiler pkg-config build-essential libmpv-dev   # Debian/Ubuntu
brew install protobuf pkg-config mpv                                              # macOS
sudo pacman -S --needed protobuf pkgconf base-devel mpv                           # Arch

# 3. Build
cargo build --release -p termusic-server --features mpv
cargo build --release -p termusic --features cover-viuer-sixel

# 4. Binaries end up in target/release/ - copy termusic and termusic-server
#    next to each other wherever you want to run them from.
```

Then edit `~/.config/termusic/server.toml` (`~/Library/Application Support/termusic/server.toml`
on macOS) by hand, creating it first by running `termusic-server` once if it doesn't exist:

```toml
[player]
music_dirs = ['/path/to/your/music']
backend = "mpv"
```

## Re-running / troubleshooting

- The script is safe to re-run any time — every step checks whether it's already satisfied
  before doing anything.
- If a build fails after a Rust/toolchain install, open a **new** terminal (PATH changes
  from `rustup`/`scoop` don't reach already-open shells) and re-run the script.
- If `termusic.exe` starts but audio doesn't play, check `backend = "mpv"` actually landed
  in `server.toml` under `[player]` (the script's automatic patch only fires if that line's
  exact format matches — if you'd hand-edited the file into an unusual shape first, fix it
  manually per the fallback section above).
