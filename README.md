# Terminal Music and Podcast Player written in Rust

[![MSRV](https://img.shields.io/badge/MSRV-1.90.0-blue)](https://releases.rs/docs/1.90.0/)

> Personal fork of [tramhao/termusic](https://github.com/tramhao/termusic) — not published to
> crates.io or any package manager. Build from source (see below). The upstream project's
> badges/install methods (cargo, Homebrew, AUR, etc.) are removed here since they'd point to
> the wrong package.

Listen to music and podcasts freely as both in freedom and free of charge!

<table>
    <tr>
        <td>
            <img src="https://github.com/tramhao/termusic/blob/master/screenshots/main.png?raw=true" alt="Main view" style="width: 500px;"/>
        </td>
        <td>
            <img src="https://github.com/tramhao/termusic/blob/master/screenshots/tageditor.png?raw=true" alt="Tag editor" style="width: 500px;"/>
        </td>
    </tr>
</table>

**Freedom**: As time goes by, online service providers control pretty much everything we listen to.
Complicated copyright issues make things worse.
If my favorite song cannot be found on a website, I'll probably just not listen to them for years.

**Free of charge**: You can download from YouTube, NetEase, Migu and KuGou for free.
No need to register for monthly paid memberships.

As a contributor of [GOMU](https://github.com/issadarkthing/gomu), I met serious problems during development. The main problem is data race condition.
So I rewrote the player in rust, and hope to solve the problem.

## About this fork

Maintained by [gupta-v](https://github.com/gupta-v). Notable changes from upstream:

- Windows `mpv` backend support, with a one-shot setup script
  (`scripts/setup-windows.ps1`, see [`documentation/setup-docs.md`](./documentation/setup-docs.md))
  that handles the Cloudflare-gated download and MinGW/MSVC import-lib mismatch upstream's
  Windows builds otherwise require fixing by hand.
- Redesigned TUI layout (Library / Playlist / Cover art / Track details / Lyrics panels,
  plus a persistent status bar with volume/shuffle/repeat).
- `yt-dlp`-backed YouTube search (no dependency on public Invidious mirrors, which are
  frequently rate-limited or blocked) with corrected Opus/Vorbis-comment tagging.
- Assorted correctness fixes: Windows path handling for the `mpv` backend and the local
  track database, library-scan filtering, and cover-art rendering.

## Supported Formats

Below are the audio formats supported by the various backends.

In the case that metadata is not supported, an attempt will still be made to play the file.

| Container  | Rusty |  MPV  | Gstreamer | Metadata |
| :--------: | :---: | :---: | :-------: | :------: |
| MP4 / M4A  |  Yes  |  Yes  |    Yes    |   Yes    |
|    MP3     |  Yes  |  Yes  |    Yes    |   Yes    |
|    OGG     |  Yes  |  Yes  |    Yes    |   Yes    |
|    FLAC    |  Yes  |  Yes  |    Yes    |   Yes    |
|    ADTS    |  Yes  |  Yes  |    Yes    |   Yes    |
| WAV / AIFF |  Yes  |  Yes  |    Yes    |   Yes    |
|    CAF     |  Yes  |  Yes  |    Yes    |    No    |
| MKV / WebM |  Yes  |  Yes  |    Yes    |    No    |

|      Codec      | Rusty |  MPV  | Gstreamer |
| :-------------: | :---: | :---: | :-------: |
|     AAC-LC      |  Yes  |  Yes  |    Yes    |
|     HE-AAC      |  No   |  Yes  |    Yes    |
| MP3 / MP2 / MP1 |  Yes  |  Yes  |    Yes    |
|      FLAC       |  Yes  |  Yes  |    Yes    |
|       WAV       |  Yes  |  Yes  |    Yes    |
|     VORBIS      |  Yes  |  Yes  |    Yes    |
|      OPUS       | No*1  |  Yes  |    Yes    |
|      ADPCM      |  Yes  |  Yes  |    Yes    |
|       PCM       |  Yes  |  Yes  |    Yes    |

*1: `Opus` codec is supported in rusty backend if feature `rusty-libopus` is enabled.

## Installation

### Requirements

#### MSRV

The minimal Rust version required to build this project is `1.90.0`.

Note that using non-default features might increase the MSRV.

#### Dependencies

##### Linux

**Quick setup:** `./scripts/setup-unix.sh` handles everything below in one go (Rust, protoc,
build tools, libmpv, build, and config; also works on macOS) — see
[`documentation/setup-docs.md`](./documentation/setup-docs.md) for details, or read on for
the manual steps.

| Package name (ubuntu) | Package name (arch) | Required | Build-time-only |      Feature       |                      Description                      |   MSRV   |
| :-------------------: | :-----------------: | :------: | :-------------: | :----------------: | :---------------------------------------------------: | :------: |
|         `git`         |        `git`        |    X     |        X        |                    |                    version control                    |          |
|        `clang`        |       `clang`       |    X     |        X        |                    |       General Build tools (and sqlite compile)        |          |
|  `protobuf-compiler`  |     `protobuf`      |    X     |        X        |                    | communication protocol between server and client(tui) |          |
|    `libdbus-1-dev`    |       `dbus`        |    X     |     unknown     |                    |                  MPRIS media control                  |          |
|   `libasound2-dev`    |     `alsa-lib`      |    X     |     unknown     |                    |                     ALSA headers                      |          |
|       `yt-dlp`        |      `yt-dlp`       |          |                 |                    |                 Download some tracks                  |          |
|       `ffmpeg`        |      `ffmpeg`       |          |                 |                    |             Post-Processing for `yt-dlp`              |          |
|         `mpv`         |        `mpv`        |          |                 |       `mpv`        |                      MPV Backend                      |          |
|      `gstreamer`      |     `gstreamer`     |          |                 |       `gst`        |                   Gstreamer Backend                   |          |
|       `libopus`       |      `libopus`      |    X     |                 |  `rusty-libopus`   |          Opus codec support in rusty backend          | `1.89.0` |
|     `ueberzugpp`      |    `ueberzugpp`     |          |                 |  `cover-ueberzug`  |               Ueberzug protocol support               |          |
|     `libstdc++6`      |     `gcc-libs`      |          |                 | `rusty-soundtouch` |       Soundtouch requires linking to libstdc++        |          |

#### Windows

**Quick setup:** `.\scripts\setup-windows.ps1` handles everything below in one go (Scoop,
Rust, protoc, MSVC Build Tools, libmpv, build, and config) — see
[`documentation/setup-docs.md`](./documentation/setup-docs.md) for details, or read on
for the manual steps.

All the packages here can be installed via various sources, for ease of install the `winget` package name is listed.

|        Package name (winget)        |            Alternative Source             | Required | Build-time-only |      Feature       |                      Description                      |   MSRV   |
| :---------------------------------: | :---------------------------------------: | :------: | :-------------: | :----------------: | :---------------------------------------------------: | :------: |
|              `Git.Git`              |                                           |    X     |        X        |                    |                    version control                    |          |
| `Microsoft.VisualStudio.BuildTools` |                                           |    X     |        X        |                    |           General Windows (C++) build tools           |          |
|          `Google.Protobuf`          |                                           |    X     |        X        |                    | communication protocol between server and client(tui) |          |
|              `yt-dlp`               |                                           |          |                 |                    |                 Download some tracks                  |          |
|              `ffmpeg`               |                                           |          |                 |                    |             Post-Processing for `yt-dlp`              |          |
|         *see below*                 |                                           |          |                 |       `mpv`        |                      MPV Backend                      |          |
|               unknown               |                                           |          |                 |       `gst`        |                   Gstreamer Backend                   |          |
|             unavailable             | [libopus official site][libopus-download] |    X     |                 |  `rusty-libopus`   |          Opus codec support in rusty backend          | `1.89.0` |
|          *see list below*           |                                           |          |        X        | `rusty-soundtouch` |       Soundtouch requires linking to libstdc++        |          |

- See [MSVC Prerequisites: only the required components](https://rust-lang.github.io/rustup/installation/windows-msvc.html#installing-only-the-required-components-optional) for a minimal install

[libopus-download]: <https://opus-codec.org/downloads/> "Needs to be manually compiled for windows"

##### Windows `rusty-soundtouch`

Compiling feature `rusty-soundtouch` on windows requires a bunch extra dependencies that are otherwise not required.

It is recommended to just use the pre-built binaries by Github Actions to avoid installing ~2GB of extra C++ Dependencies and potentially having to mess around with dependencies.

If you actually still wanted to compile this yourself, you will need:

1. Install `C++ CMake tools for Windows` via `Microsoft.VisualStudio.BuildTools` (or also known as `Visual Studio Installer`)
2. Install a Clang compiler
  At the time of writing, `Microsoft.VisualStudio.BuildTools` does not provide a `clang.dll` / `libclang.dll`, which the `cc` crate needs for building C++.
  Instead, simply install `llvm` via `winget`: `winget install llvm`

This should be everything and feature `rusty-soundtouch` should compile without problems.

##### Windows `mpv`

libmpv Windows dev builds (headers + import lib + dll) are published at
[sourceforge.net/projects/mpv-player-windows/files/libmpv][mpv-windows-dev], but:

- The download is gated behind a Cloudflare bot challenge that blocks non-browser
  clients (curl, aria2, `Invoke-WebRequest`) — grab it in an actual browser.
- The package ships a MinGW-format import lib (`libmpv.dll.a`), which the MSVC
  linker cannot read. An MSVC-format `mpv.lib` has to be generated from the
  dll's export table (`dumpbin /exports` + `lib.exe /def:...`).

To skip all of that, run:

```powershell
.\scripts\setup-mpv-windows.ps1
```

This downloads a pre-processed package (already containing a working `mpv.lib`)
from a GitHub Release and drops it in `vendor/mpv-windows/`, where
[`playback/build.rs`](./playback/build.rs) picks it up automatically — no env
vars needed. Then build normally:

```powershell
cargo build --release -p termusic-server --features mpv
```

Note the vendored `libmpv-2.dll` requires an AVX2-capable CPU (x86-64-v3).

[mpv-windows-dev]: <https://sourceforge.net/projects/mpv-player-windows/files/libmpv/>

#### Backends

Default backend: `rusty`

|     Backend      | Requirements                                                                                                      |
| :--------------: | :---------------------------------------------------------------------------------------------------------------- |
| Symphonia(rusty) | On Linux [`libasound2-dev`](https://launchpad.net/ubuntu/noble/+package/libasound2-dev) is required for building. |
|    GStreamer     | [GStreamer](https://gstreamer.freedesktop.org)                                                                    |
|       MPV        | [MPV](https://mpv.io/)                                                                                            |

There are extra features for some backends:
Note that they are not enabled by default and potentially increase non-rust dependencies.

|      Feature       | Backend |                            Description                            | Extra Dependencies |   MSRV   |
| :----------------: | :-----: | :---------------------------------------------------------------: | :----------------: | :------: |
| `rusty-soundtouch` | `rusty` | Enable `soundtouch` compilation and use as default speed-modifier |                    |          |
|  `rusty-libopus`   | `rusty` |         Enable `libopus` support to support `opus` files          |     `libopus`      | `1.89.0` |

#### Album cover support

To display covers in the terminal itself, feature `cover` can be enabled.
To only enable specific protocols for cover support, see [tui/Cargo.toml#features](./tui/Cargo.toml).

Feature `cover-ueberzug` will require some ueberzug implementation to be present at runtime.

### Files

#### Configuration

Configuration files can be found in:

| System  |                   Path                    |
| :-----: | :---------------------------------------: |
|  Linux  |           `~/.config/termusic/`           |
|   Mac   | `~/Library/Application Support/termusic/` |
| Windows |           `%APPDATA%\termusic\`           |

Files & Folders:

|     Paths      |                   Description                    |
| :------------: | :----------------------------------------------: |
| `server.toml`  |             For server configuration             |
|   `tui.toml`   |              For TUI configuration               |
|   `themes/`    | Extra Themes to be selected in the Config Editor |
| `playlist.log` | The Playlist storing the current playlist/queue  |
| `library2.db`  |            The Indexed Music library             |
|   `data.db`    |               The Podcast Database               |

#### Logs

By default logs can be found in:

| System  |    Path    |
| :-----: | :--------: |
|  Linux  |  `/tmp/`   |
|   Mac   | `/tmp/`(?) |
| Windows |  `%TMP%\`  |

Files:

|         Files         |   Description   |
| :-------------------: | :-------------: |
| `termusic-server.log` | The server logs |
|  `termusic-tui.log`   |  The TUI logs   |
| `termusic-cover.EXT`  |   MPRIS cover   |

The default log level is `WARNING` (can be changed via [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)).

Note that log files are only created on the first log line to be saved.

### From Source

```bash
git clone https://github.com/gupta-v/termusic.git
cd termusic
make
```

On Windows, see [Quick setup](#windows) above instead — `make` isn't used there.

Then install with:

```bash
make install
```

By default, termusic can display album covers in Kitty or iTerm2.
If you need album covers displayed on other terminals, you can enable the `sixel` protocol or use a ueberzug implementation(x11/xwayland only).

To build all backends and all cover protocols and install them in your home:

```bash
make full
```

Finally, you can run it with:

```bash
~/.local/share/cargo/bin/termusic
```

To build with all backends and all cover protocols without copying binaries elsewhere:

```bash
make all-backends
```

## Contributors

hasezoey

## Thanks

- [tui-realm](https://github.com/veeso/tui-realm)
- [termscp](https://github.com/veeso/termscp)
- [netease-cloud-music-gtk](https://github.com/gmg137/netease-cloud-music-gtk)
- [alacritty-themes](https://github.com/rajasegar/alacritty-themes)
- [shellcaster](https://github.com/jeff-hughes/shellcaster)
- [stream-download](https://github.com/aschey/stream-download-rs)

## License

MIT License for main part of code.
GPLv3 for Podcast code under `lib/src/podcast/mod.rs`. Comes from shellcaster.
