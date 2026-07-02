# HCSminer - Post-Quantum Bitcoin Miner

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Vulkan](https://img.shields.io/badge/Vulkan-1.3-red)](https://www.vulkan.org/)
[![Bitcoin](https://img.shields.io/badge/Bitcoin_Core-28.0-blue)](https://bitcoincore.org/)

**HCSminer** - Post-quantum Bitcoin miner with Vulkan GPU acceleration.

> Made with ❤️ by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)

---

## Quick Start

### One-Click Mining
```batch
scripts\start.bat
```
Auto-downloads Bitcoin Core, configures it, starts bitcoind, opens Web UI.

### Manual Setup
```batch
:: 1. Start Bitcoin Core
bitcoind -server -rpcuser=qpzip_admin -rpcpassword=qpzip_secure_password_2024

:: 2. Build & run miner
cd src\qp_zip_miner
cargo build --release
target\release\qp_zip_miner.exe

:: 3. Open Web UI
start http://localhost:3000
```

## Configuration
See `miner_config.toml` for all settings.

## Architecture
Mainnet -> Bitcoin Core -> RPC -> HCSminer -> CPU/Vulkan/TUI

## User Interfaces
- **TUI**: Real-time hashrate, VRAM gauges. Press 'q' to quit.
- **Web UI**: http://localhost:3000

## Releases
- **v2.0.0** (Win x64): `releases/v2.0.0/qp_zip_miner.exe`
- Build: `cd src/qp_zip_miner && cargo build --release`

## License
MIT — Made with ❤️ by timfromhcs and @hcmedia
