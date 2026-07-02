# HCSminer - Post-Quantum Bitcoin Pool Miner

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Vulkan](https://img.shields.io/badge/Vulkan-1.3-red)](https://www.vulkan.org/)
[![Stratum](https://img.shields.io/badge/Stratum-V1-blue)](https://public-pool.io/)

**HCSminer** - Pool-Mining Bitcoin Miner with Post-Quantum signature compression, Vulkan GPU acceleration and real-time TUI.

> Made with love by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)

---

## Quick Start
```batch
scripts\start.bat
```

Or manually:
```batch
cd src\qp_zip_miner
cargo build --release
target\release\hcsminer.exe
```

## Configuration
Edit `miner_config.toml`:
```toml
btc_address = "<your BTC address>"  # Only field you need to edit
worker_name = "hcsminer"
pool_host = "public-pool.io"
pool_port = 13333  # V1=13333, TLS=14333, V2=23331
```

## Pool: public-pool.io (PPLNS)
| Mode | Address | Status |
|------|---------|--------|
| Stratum V1 | stratum+tcp://public-pool.io:13333 | ✅ Live |
| Stratum V1+TLS | stratum+tls://public-pool.io:14333 | 🔒
| Stratum V2 | stratum+tcp://public-pool.io:23331 | 🚀
| Datum | datum://public-pool.io:23336 | 🔑

## Releases
| Version | Download |
|---------|----------|
| v2.0.0 (Win x64) | releases/v2.0.0/hcsminer.exe |

## Architecture
```
HCSminer -> Stratum V1 -> public-pool.io (PPLNS)
  ├── CPU Miner (SHA-256d)
  ├── GPU Detector (Vulkan)
  ├── TUI (ratatui)
  └── Quant Engine (Post-Quantum)
```

## Developer Resources
- AGENTS.md - Guidelines for AI assistants
- agent_skills/ - Rust repair, Stratum protocol, Vulkan debug
- developer_resources/ - Agent guidelines, specs, benchmarks
- docs/ - Architecture and setup documentation

## License
MIT - Made with love by timfromhcs and @hcmedia
