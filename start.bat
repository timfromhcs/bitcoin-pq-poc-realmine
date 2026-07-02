@echo off
title BIP-QP-ZIP GPU Miner
echo ====================================================
echo    BIP-QP-ZIP GPU-ACCELERATED AMD ROCm MINER RUNTIME
echo ====================================================
echo.
echo [INFO] Compiling miner binary for Windows x64...
cargo build --release --manifest-path src/qp_zip_miner/Cargo.toml
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo compilation failed. Please verify Rust installation.
    pause
    exit /b %ERRORLEVEL%
)

echo [INFO] Starting Web UI and launching miner backend...
echo [INFO] Opening Web UI in your default browser...
start http://localhost:3000

src\qp_zip_miner\target\release\qp_zip_miner.exe
pause
