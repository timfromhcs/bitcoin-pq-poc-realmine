@echo off
setlocal enabledelayedexpansion
title HCSminer v2.0 - Pool Mining @ public-pool.io

:: Farben
set "C=%~dp0..\"
set "MINER_DIR=%C%src\qp_zip_miner"
set "RELEASE=%C%releases\v2.0.0"

cls
echo.
echo ^|============================================================^|
echo ^|                   HCSminer v2.0                            ^|
echo ^|         Post-Quantum Pool Miner (PPLNS)                    ^|
echo ^|              public-pool.io:13333                         ^|
echo ^|============================================================^|
echo ^|     Made with love by timfromhcs and @hcmedia              ^|
echo ^|============================================================^|
echo.

:: Pruefe Miner Binary
set "MINER="
if exist "%RELEASE%\hcsminer.exe" set "MINER=%RELEASE%\hcsminer.exe"
if exist "%MINER_DIR%\target\release\hcsminer.exe" set "MINER=%MINER_DIR%\target\release\hcsminer.exe"
if exist "%MINER_DIR%\target\debug\hcsminer.exe" set "MINER=%MINER_DIR%\target\debug\hcsminer.exe"

if not defined MINER (
    echo [INFO] Compiling miner binary for Windows x64...
    cd /d "%MINER_DIR%"
    cargo build --release
    if exist "target\release\hcsminer.exe" set "MINER=%MINER_DIR%\target\release\hcsminer.exe"
    if not defined MINER (
        echo [ERROR] Compilation failed.
        pause
        exit /b 1
    )
) else (
    echo [OK] Miner binary found
)
echo.

:: Starte Miner
echo [START] Starting HCSminer...
start "HCSminer-Miner" /MIN "%MINER%"
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Failed to start miner
    pause
    exit /b 1
)
echo [OK] Miner gestartet
echo.

:: Monitoring-Loop
:monitor
cls
echo ^|============================================================^|
echo ^|                   HCSminer v2.0 - RUNNING                    ^|
echo ^|============================================================^|
echo ^|  Pool: public-pool.io (Stratum V1 ^> V2 Fallback)            ^|
echo ^|  Web UI: http://localhost:3000                             ^|
echo ^|  TUI:    Terminal interface (press q to quit)              ^|
echo ^|============================================================^|
echo ^|  [CTRL+C] to stop mining and close                        ^|
echo ^|============================================================^|
echo.
tasklist /FI "IMAGENAME eq hcsminer.exe" 2>NUL | find /I /N "hcsminer.exe" >NUL
if !ERRORLEVEL! NEQ 0 (
    echo [!] Miner nicht mehr aktiv. Starte neu...
    start "HCSminer-Miner" /MIN "%MINER%"
)
echo.
timeout /t 5 /nobreak > nul
goto monitor

