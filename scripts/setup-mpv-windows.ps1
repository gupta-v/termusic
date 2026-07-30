<#
.SYNOPSIS
    Fetches vendored libmpv dev files (headers, MSVC import lib, runtime dll)
    needed to build termusic with `--features mpv` on Windows, and drops them
    into vendor/mpv-windows/ at the repo root.

.DESCRIPTION
    Upstream's libmpv Windows builds (sourceforge.net/projects/mpv-player-windows)
    are gated behind a Cloudflare bot challenge that blocks non-browser downloads,
    and ship a MinGW-format import lib (libmpv.dll.a) that MSVC's linker can't
    read directly. This script instead pulls a pre-processed package (containing
    an MSVC-format mpv.lib generated via `dumpbin /exports` + `lib.exe /def:`)
    from a GitHub Release, which has no such restriction.

    After running this, `cargo build --release --features mpv` picks the files
    up automatically via playback/build.rs — no env vars needed.

.PARAMETER AssetUrl
    Override the release asset URL (e.g. to point at a different fork/release).
#>
param(
    [string]$AssetUrl = "https://github.com/gupta-v/termusic/releases/download/mpv-windows-deps-v1/mpv-windows-dev-x86_64-v3.zip"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$vendorDir = Join-Path $repoRoot "vendor\mpv-windows"
$zipPath = Join-Path $env:TEMP "mpv-windows-dev.zip"

Write-Host "Downloading libmpv dev files from $AssetUrl"
Invoke-WebRequest -Uri $AssetUrl -OutFile $zipPath

if (Test-Path $vendorDir) {
    Remove-Item -Recurse -Force $vendorDir
}
New-Item -ItemType Directory -Force -Path $vendorDir | Out-Null

Write-Host "Extracting to $vendorDir"
Expand-Archive -Path $zipPath -DestinationPath $env:TEMP\mpv-windows-dev-extract -Force
Copy-Item -Recurse -Force "$env:TEMP\mpv-windows-dev-extract\mpv-windows-dev\*" $vendorDir
Remove-Item -Recurse -Force "$env:TEMP\mpv-windows-dev-extract"
Remove-Item -Force $zipPath

Write-Host "Done. vendor\mpv-windows\64\mpv.lib and libmpv-2.dll are in place."
Write-Host "Build with: cargo build --release -p termusic-server --features mpv"
Write-Host "Note: libmpv-2.dll requires an AVX2-capable CPU (x86-64-v3)."
