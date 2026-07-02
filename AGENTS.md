# AGENTS.md - HCSminer AI Coding Assistant Guidelines

Made with ❤️ by timfromhcs and @hcmedia

## Project Overview
**HCSminer v2.0** - Post-quantum Bitcoin Pool-Mining System (PPLNS @ public-pool.io):
- **rust_qp_zip** (no_std Rust library): Lattice quantization, ZK proofs, extraction
- **qp_zip_miner** (Rust binary): Multi-threaded CPU miner with Vulkan GPU detection, TUI, Stratum V1

## 🎯 Key Principles
1. **No panics** - All operations fallible, use Result<>, avoid unwrap() in production code
2. **Atomic counters** - Lock-free statistics with AtomicU64/AtomicBool (never Mutex for stats)
3. **no_std** - rust_qp_zip must remain #![no_std] (no libstd dependency)
4. **Graceful fallback** - GPU failure falls back to CPU (never panic!)
5. **Non-blocking I/O** - Stratum reads use 100ms timeout + peek() pattern
6. **Pool-first** - All mining goes through public-pool.io Stratum V1
7. **Buffer pooling** - Pre-allocate buffers in hot paths, avoid String allocations

## 🏗️ Architecture v2.0 Changes

### Performance-Engine (NEU)
`
main.rs → pool_miner_loop()
  ├── AtomicU64 HASHES_TOTAL (lock-free)
  ├── AtomicU64 SHARES_ACCEPTED (lock-free)
  ├── Arc<AtomicBool> running (graceful shutdown)
  ├── TUI update thread (100ms rolling Ø)
  └── miner_core.rs (multi-threaded CPU engine)
`

### Bugfixes (v2.0)
- ✅ wait_for_notify() method hinzugefügt (fehlte in v1.0)
- ✅ miner_core.rs implementiert (war leer)
- ✅ start.bat Binary-Name korrigiert (qp_zip_miner.exe → hcsminer.exe)
- ✅ unwrap() durch .ok().map() ersetzt (kein Absturz bei Mutex-Poison)
- ✅ TUI Poison-Handling: Graceful Break statt panic
- ✅ serde_derive → serde (deprecated entfernt)
- ✅ Rolling Hashrate-Berechnung (akkurat, nicht mehr fester divisor)

### Performance (v2.0)
- 🚀 Multi-Thread SHA-256d (alle CPU-Kerne)
- 🚀 Atomic Counters (0 Lock-Contention)
- 🚀 Vor-allocierte Header-Buffer (kein Heap im Hot-Path)
- 🚀 Quick Pre-Filter (90% Nonces billig rausgefiltert)
- 🚀 5000er Batches (vorher 500)

## 📋 Code Conventions
- Use Result<T, QPZipError> for library code
- #[cfg(feature = "opencl")] for OpenCL features
- TUI operations must be non-blocking (separate thread)
- Configuration via TOML (miner_config.toml)
- Atomic types for cross-thread statistics (no Mutex in hot paths)

## 🗺️ File Map
- ust_qp_zip/ - Post-quantum crypto, no_std
- qp_zip_miner/src/main.rs - Entry point, pool miner loop
- miner_modules/miner_core.rs - Multi-threaded CPU miner engine
- miner_modules/stratum.rs - Stratum V1 protocol handler
- miner_modules/tui.rs - ratatui Terminal UI
- miner_modules/vulkan.rs - Vulkan GPU detection
- miner_modules/config.rs - TOML config parser

## 📚 Agent Skills
See gent_skills/ directory:
- ust_repair_guide.md - Common compile errors & fixes
- stratum_protocol.md - Stratum V1 message flow & mining math
- ulkan_debug.md - Vulkan ash API, VRAM detection, CStr strings
