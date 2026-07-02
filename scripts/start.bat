@echo off
title BIP-QP-ZIP MTP Miner v2.0 - Mainnet Mining
setlocal enabledelayedexpansion

:: Colors
set "GREEN=[92m"
set "YELLOW=[93m"
set "RED=[91m"
set "CYAN=[96m"
set "RESET=[0m"

echo %CYAN%====================================================%RESET%
echo %CYAN%    BIP-QP-ZIP MTP MINER v2.0                       %RESET%
echo %CYAN%    Mainnet Mining Launcher                          %RESET%
echo %CYAN%    GLM-5.2 Vulkan Accelerated                       %RESET%
echo %CYAN%====================================================%RESET%
echo.

:: Configuration
set "BITCOIN_DIR=%USERPROFILE%\AppData\Roaming\Bitcoin"
set "MINER_DIR=%~dp0..\src\qp_zip_miner"
set "RELEASE_DIR=%~dp0..\releases\v2.0.0"

:: Step 1: Check Bitcoin Core
echo %CYAN%[1/4] Checking Bitcoin Core...%RESET%
set "BITCOIND_PATH="
if exist "%BITCOIN_DIR%\bitcoind.exe" set "BITCOIND_PATH=%BITCOIN_DIR%\bitcoind.exe"
if exist "%ProgramFiles%\Bitcoin\daemon\bitcoind.exe" set "BITCOIND_PATH=%ProgramFiles%\Bitcoin\daemon\bitcoind.exe"
if exist "%ProgramFiles(x86)%\Bitcoin\daemon\bitcoind.exe" set "BITCOIND_PATH=%ProgramFiles(x86)%\Bitcoin\daemon\bitcoind.exe"

if not defined BITCOIND_PATH (
    echo %YELLOW%Bitcoin Core not found. Downloading...%RESET%
    echo %YELLOW%Downloading Bitcoin Core 28.0 from bitcoincore.org...%RESET%
    powershell -Command "Invoke-WebRequest -Uri 'https://bitcoincore.org/bin/bitcoin-core-28.0/bitcoin-28.0-win64.zip' -OutFile '%TEMP%\bitcoin.zip' -UseBasicParsing"
    if exist "%TEMP%\bitcoin.zip" (
        powershell -Command "Expand-Archive -Path '%TEMP%\bitcoin.zip' -DestinationPath '%TEMP%\bitcoin-extracted' -Force"
        set "BITCOIND_PATH=%TEMP%\bitcoin-extracted\bitcoin-28.0\bin\bitcoind.exe"
        echo %GREEN%Bitcoin Core extracted%RESET%
    ) else (
        echo %RED%Failed to download Bitcoin Core from bitcoincore.org%RESET%
        echo %RED%Please install manually from https://bitcoincore.org/%RESET%
        pause
        exit /b 1
    )
) else (
    echo %GREEN%Bitcoin Core found%RESET%
)

:: Step 2: Create bitcoin.conf for mining
echo %CYAN%[2/4] Configuring Bitcoin Core...%RESET%
if not exist "%BITCOIN_DIR%" mkdir "%BITCOIN_DIR%"
if not exist "%BITCOIN_DIR%\bitcoin.conf" (
    echo server=1 > "%BITCOIN_DIR%\bitcoin.conf"
    echo rpcuser=qpzip_admin >> "%BITCOIN_DIR%\bitcoin.conf"
    echo rpcpassword=qpzip_secure_password_2024 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo rpcallowip=127.0.0.1 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo rpcport=8332 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo listen=1 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo txindex=1 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo daemon=1 >> "%BITCOIN_DIR%\bitcoin.conf"
    echo %GREEN%bitcoin.conf created for mining%RESET%
) else (
    echo %GREEN%bitcoin.conf exists%RESET%
)

:: Step 3: Start Bitcoin Core
start ""BitcoinCore"" ""%BITCOIND_PATH%"" -datadir=""%BITCOIN_DIR%""
echo Please wait for Bitcoin Core to sync...
timeout /t 5 /nobreak > nul

:: Step 4: Start Miner
set MINER_PATH=
if exist ""%RELEASE_DIR%\qp_zip_miner.exe"" set MINER_PATH=%RELEASE_DIR%\qp_zip_miner.exe
if exist ""%MINER_DIR%\target\release\qp_zip_miner.exe"" set MINER_PATH=%MINER_DIR%\target\release\qp_zip_miner.exe
if exist ""%MINER_DIR%\target\debug\qp_zip_miner.exe"" set MINER_PATH=%MINER_DIR%\target\debug\qp_zip_miner.exe
if not defined MINER_PATH (
    cd /d ""%MINER_DIR%"" && cargo build --release
)
if defined MINER_PATH (
    start ""QPZIP-Miner"" ""%MINER_PATH%""
    echo Miner started! Web UI at http://localhost:3000
    start http://localhost:3000
) else ( echo FAILED )
:monitor
timeout /t 30 /nobreak > nul
goto monitor
