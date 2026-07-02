# BIP-QP-ZIP MTP Miner

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Vulkan](https://img.shields.io/badge/Vulkan-1.3-red)](https://www.vulkan.org/)
[![Bitcoin](https://img.shields.io/badge/Bitcoin_Core-28.0-blue)](https://bitcoincore.org/)

**Post-Quantum Proof-of-Work Miner with Vulkan GPU Acceleration**

Bitcoin mining with lattice-based post-quantum signature compression, Vulkan GPU offloading, and real-time TUI.

---

## Quick Start

### Prerequisites
- Bitcoin Core 28.0+ (fully synced)
- Rust toolchain (for building)
- Vulkan-capable GPU (optional, CPU fallback)

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

### miner_config.toml
| Parameter | Default | Description |
|-----------|---------|-------------|
| wallet | bc1q... | Mining payout address |
| quantization_depth | 1024.0 | Lattice precision |
| probabilistic_threshold | 0.05 | Pre-filter sensitivity |
| vulkan_device_index | -1 | GPU (-1 = auto) |
| memory_offload_threshold_mb | 512 | VRAM offload boundary |
| enable_tui | true | Toggle Terminal UI |

## Architecture

Mainnet -> Bitcoin Core (bitcoind) -> RPC -> QP-ZIP Miner -> CPU/Vulkan/TUI

Components:
- rust_qp_zip/ - Post-quantum crypto library (no_std)
- qp_zip_miner/ - GPU miner with Vulkan engine, TUI, config

## User Interfaces
- Terminal UI: Real-time hashrate, VRAM gauges, logs. Press 'q' to quit.
- Web UI: http://localhost:3000 - Browser dashboard

## Releases
| Version | Platform | Download |
|---------|----------|----------|
| v2.0.0 | Windows x64 | releases/v2.0.0/qp_zip_miner.exe |

Build: cd src/qp_zip_miner && cargo build --release

## Performance
- CPU (16 threads, Ryzen 7000): ~50 KH/s
- GPU + CPU: ~65 KH/s (with Vulkan)
- Probabilistic filter: 16x nonce reduction

## Developer Resources
- AGENTS.md - AI coding assistant guidelines
- docs/ - Architecture and setup documentation
- developer_resources/ - Specs and benchmarks

## License
MIT
