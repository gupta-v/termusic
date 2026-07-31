<#
.SYNOPSIS
    One-shot setup for building & running termusic (with the mpv backend) on Windows,
    starting from a fresh clone of this repo.

.DESCRIPTION
    Checks for and (with confirmation) installs everything needed - Scoop, Rust, protoc,
    the MSVC C++ build tools, and the vendored libmpv dev files - then builds termusic and
    termusic-server, asks where your music lives, writes that into server.toml, and drops a
    ready-to-run copy of both binaries (+ libmpv-2.dll) into dist\.

    Safe to re-run: every step checks whether it's already done before acting.

.PARAMETER MusicDir
    Skip the interactive prompt and use this path as the music library folder.
#>
param(
    [string]$MusicDir
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$distDir = Join-Path $repoRoot "dist"

function Confirm-Step {
    param([string]$Prompt)
    $answer = Read-Host "$Prompt (y/n)"
    return $answer -match '^(y|yes)$'
}

Write-Host "=== termusic Windows setup ===" -ForegroundColor Cyan

# --- 1. Scoop -----------------------------------------------------------
if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    Write-Host "You don't have Scoop (used to install Rust/protoc/git without admin rights)."
    if (Confirm-Step "Install Scoop now?") {
        Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
        Invoke-RestMethod get.scoop.sh | Invoke-Expression
    } else {
        Write-Error "Scoop is required (or install Rust/protoc/git yourself, then re-run this script). Aborting."
        exit 1
    }
} else {
    Write-Host "Scoop: OK"
}

# --- 2. Rust (MSVC toolchain) --------------------------------------------
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust isn't installed."
    if (Confirm-Step "Install Rust (via Scoop's rustup) now?") {
        scoop install rustup
        rustup default stable-msvc
        # refresh PATH in this session
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    } else {
        Write-Error "Rust is required. Aborting."
        exit 1
    }
} else {
    Write-Host "Rust: OK"
}

# --- 3. protoc ------------------------------------------------------------
if (-not (Get-Command protoc -ErrorAction SilentlyContinue)) {
    Write-Host "protoc (protobuf compiler) isn't installed."
    if (Confirm-Step "Install protoc (via Scoop) now?") {
        scoop install protobuf
    } else {
        Write-Error "protoc is required. Aborting."
        exit 1
    }
} else {
    Write-Host "protoc: OK"
}

# --- 4. MSVC C++ build tools (needed to link anything, incl. libmpv) -----
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasMsvc = (Test-Path $vswhere) -and ((& $vswhere -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) -ne $null)
if (-not $hasMsvc) {
    Write-Host "MSVC C++ Build Tools (needed to link Rust binaries on Windows) aren't installed."
    Write-Host "This is a multi-GB download and can take a while."
    if (Confirm-Step "Install via winget now?") {
        if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
            Write-Error "winget isn't available. Install 'Desktop development with C++' manually via the Visual Studio Installer, then re-run this script."
            exit 1
        }
        winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    } else {
        Write-Error "MSVC Build Tools are required. Aborting."
        exit 1
    }
} else {
    Write-Host "MSVC Build Tools: OK"
}

# --- 5. Vendored libmpv ---------------------------------------------------
$mpvLib = Join-Path $repoRoot "vendor\mpv-windows\64\mpv.lib"
if (-not (Test-Path $mpvLib)) {
    Write-Host "Fetching vendored libmpv dev files..."
    & (Join-Path $PSScriptRoot "setup-mpv-windows.ps1")
} else {
    Write-Host "libmpv vendor files: OK"
}

# --- 6. Build --------------------------------------------------------------
Write-Host "Building termusic-server (with mpv backend)..." -ForegroundColor Cyan
Push-Location $repoRoot
try {
    cargo build --release -p termusic-server --features mpv
    if ($LASTEXITCODE -ne 0) { throw "termusic-server build failed" }

    Write-Host "Building termusic (TUI client)..." -ForegroundColor Cyan
    cargo build --release -p termusic --features cover-viuer-sixel
    if ($LASTEXITCODE -ne 0) { throw "termusic build failed" }
} finally {
    Pop-Location
}

# --- 7. Collect a portable dist\ folder ------------------------------------
New-Item -ItemType Directory -Force -Path $distDir | Out-Null
$releaseDir = Join-Path $repoRoot "target\release"
foreach ($file in @("termusic.exe", "termusic-server.exe", "libmpv-2.dll")) {
    Copy-Item -Force (Join-Path $releaseDir $file) (Join-Path $distDir $file)
}
Write-Host "Binaries ready in $distDir"

# --- 8. Music library folder -----------------------------------------------
if (-not $MusicDir) {
    $MusicDir = Read-Host 'Paste the path to your music folder in quotes ("D:\Music"), or press Enter to use your default Music folder'
}
$MusicDir = $MusicDir.Trim().Trim('"').Trim("'")
if ([string]::IsNullOrWhiteSpace($MusicDir)) {
    $MusicDir = [Environment]::GetFolderPath("MyMusic")
}
if (-not (Test-Path $MusicDir)) {
    Write-Warning "'$MusicDir' doesn't exist yet - creating it."
    New-Item -ItemType Directory -Force -Path $MusicDir | Out-Null
}
Write-Host "Library folder: $MusicDir (you can change this later in termusic's config editor, key: Shift+C)"

# --- 9. Write server.toml (music_dirs + mpv backend) -----------------------
$configDir = Join-Path $env:APPDATA "termusic"
$configPath = Join-Path $configDir "server.toml"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null

if (-not (Test-Path $configPath)) {
    # Let the server generate its own full set of defaults first, so this script never has
    # to hand-maintain a duplicate of every config field.
    Write-Host "Generating default config..."
    $proc = Start-Process -FilePath (Join-Path $distDir "termusic-server.exe") -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 2
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}

$content = Get-Content $configPath -Raw
# TOML single-quoted ("literal") strings take backslashes as-is, no escaping needed -
# and match what termusic itself already writes for Windows paths in this file.
$content = $content -replace "music_dirs\s*=\s*\[[^\]]*\]", "music_dirs = ['$MusicDir']"
$content = $content -replace 'backend\s*=\s*"[^"]*"', 'backend = "mpv"'
Set-Content -Path $configPath -Value $content -NoNewline

Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "Run: $distDir\termusic.exe"
Write-Host "(termusic-server starts automatically alongside it - no need to run it separately.)"
