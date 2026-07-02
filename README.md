# HCSminer v2.0 - Post-Quantum Bitcoin Pool Miner

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20x64-blue)](https://github.com/timfromhcs/bitcoin-pq-poc-realmine)

**HCSminer** - High-performance pool-mining client for Bitcoin with post-quantum signature compression, Stratum V1 protocol support, and real-time TUI monitoring.

> Made with ❤️ by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)

---

## Features

- **Multi-threaded CPU Mining** - Uses ALL CPU cores for maximum hashrate via work-stealing
- **Optimized SHA-256d** - Pre-allocated buffers, Atomic Counters, zero lock contention
- **Vulkan GPU Detection** - Automatic GPU detection and VRAM monitoring
- **Real-time TUI** - ratatui Terminal UI with hashrate, shares, RAM/VRAM usage
- **Stratum V1 Pool-Mining** - Connection to public-pool.io (PPLNS)
- **Post-Quantum Ready** - Lattice-based quantization and ZK-Proofs
- **Live Statistics** - Rolling hashrate average, share tracking, logging

## Quick Start

`atch
start.bat
`

Or manually (from repo root):
`atch
cd src\qp_zip_miner
cargo build --release
copy target\release\hcsminer.exe ..\..\
hcsminer.exe
`

## Configuration

Edit miner_config.toml:
`	oml
btc_address = "your_BTC_address"   # REQUIRED - change this!
worker_name = "hcsminer"            # Worker name for the pool
pool_host = "public-pool.io"        # Pool host
pool_port = 13333                   # V1=13333, TLS=14333, V2=23331
threads = 16                        # CPU cores (auto: num_cpus)
enable_tui = true                   # Enable TUI
`

## Pool: public-pool.io (PPLNS)

| Mode | Address | Status |
|------|---------|--------|
| Stratum V1 | stratum+tcp://public-pool.io:13333 | Live |
| Stratum V1+TLS | stratum+tls://public-pool.io:14333 | Secure |
| Stratum V2 | stratum+tcp://public-pool.io:23331 | Beta |

## Performance Optimizations (v2.0)

| Optimization | Description | Impact |
|-------------|-------------|--------|
| **Multi-Thread Mining** | All CPU cores via work-stealing | 8-16x Hashrate |
| **Atomic Counters** | Lock-free statistics | 0% Lock-Contention |
| **Buffer Pooling** | Pre-allocated header buffers | ~30% fewer allocations |
| **100% Nonce Validation** | Every nonce checked (NO false pre-filter) | 16x more effective |
| **Rolling Hashrate** | Sliding average every 100ms | Accurate display |
| **Non-Blocking I/O** | 1ms timeout + peek() pattern | No mining stalls |

## Bugfixes (v2.0 final)

| Bug | Fixed |
|-----|-------|
| CRITICAL: False "Quick Pre-Filter" skipped 93.75% of nonces | REMOVED - 100% validation now |
| CRITICAL: Single-thread mining despite docs saying multi-thread | Now uses ALL cores via miner_core.rs |
| CRITICAL: Double newline in Stratum send() broke protocol | Fixed to single newline |
| Compiler error: duplicate AtomicBool import | Fixed |
| wait_for_notify() missing (crash on job polling) | Implemented |
| miner_core.rs was empty AND unused | Now fully implemented and active |
| Wrong binary name in start.bat (qp_zip_miner.exe -> hcsminer.exe) | Fixed |
| serde_derive deprecated -> compiler warning | Fixed |
| unwrap() in hot-path -> crash risk | Replaced with .ok().map() |
| TUI Mutex Poisoning -> miner crashes | Graceful Break on poison |
| Unused ureq dependency bloat | Removed |
| start.bat fails from wrong working directory | cd /d "%~dp0" added |

## Project Structure

`
HCSminer/
+-- start.bat                    # Entry point
+-- miner_config.toml            # Configuration
+-- src/
|   +-- qp_zip_miner/            # Rust Miner Binary
|   |   +-- src/
|   |       +-- main.rs          # Entry, mining loop
|   |       +-- miner_modules/
|   |           +-- miner_core.rs # Multi-Thread CPU miner
|   |           +-- stratum.rs   # Stratum V1 protocol
|   |           +-- tui.rs       # ratatui Terminal UI
|   |           +-- vulkan.rs    # Vulkan GPU detection
|   |           +-- config.rs    # TOML configuration
|   +-- rust_qp_zip/             # Post-Quantum Crypto Library
|       +-- src/
|           +-- quantizer.rs     # Lattice quantization
|           +-- zk_prover.rs     # ZK-SNARK proofs
|           +-- extractor.rs     # Witness extraction
|           +-- ffi.rs           # C-FFI bindings
+-- docs/                        # Documentation
+-- agent_skills/                # AI Assistant guides
`

## Further Documentation

- [AGENTS.md](AGENTS.md) - Guidelines for AI assistants
- [agent_skills/](agent_skills/) - Rust Repair, Stratum Protocol, Vulkan Debug
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) - Developer manual

## License

MIT - Made with ❤️ by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)
