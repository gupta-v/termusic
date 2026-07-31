@echo off
REM Double-clickable wrapper for setup-windows.ps1 (full one-shot setup: Scoop, Rust,
REM protoc, MSVC Build Tools, libmpv, build, and config - see setup-documentation.md).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-windows.ps1" %*
pause
