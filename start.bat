@echo off
title HCSminer v2.0 - Post-Quantum Pool Miner
echo ====================================================
echo    HCSminer v2.0 - Pool Mining (PPLNS)
echo    public-pool.io
echo ====================================================
echo.
echo [INFO] Compiling miner binary for Windows x64...
echo [INFO] Using profile: release (optimized for speed)
cargo build --release --manifest-path src/qp_zip_miner/Cargo.toml
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo compilation failed. Please verify Rust installation.
    pause
    exit /b %ERRORLEVEL%
)

echo [INFO] Starting HCSminer...
echo [INFO] Press 'q' in the TUI to quit.
echo.
src\qp_zip_miner\target\release\hcsminer.exe
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Miner exited with error code %ERRORLEVEL%
    pause
)
