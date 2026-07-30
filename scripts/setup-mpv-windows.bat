@echo off
REM Double-clickable wrapper for setup-mpv-windows.ps1 (fetches libmpv dev
REM files needed for `cargo build --features mpv` on Windows).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-mpv-windows.ps1" %*
pause
