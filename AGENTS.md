# AGENTS.md - HCSminer AI Coding Assistant Guidelines

Made with ❤️ by timfromhcs and @hcmedia

## Project Overview
**HCSminer v2.0** - Post-quantum Bitcoin Pool-Mining System (PPLNS @ public-pool.io):
- **rust_qp_zip** (no_std Rust library): Lattice quantization, ZK proofs, extraction
- **qp_zip_miner** (Rust binary): Multi-threaded CPU miner with Vulkan GPU detection, TUI, Stratum V1

## Key Principles
1. **No panics** - All operations fallible, use Result<>, avoid unwrap() in production code
2. **Atomic counters** - Lock-free statistics with AtomicU64/AtomicBool (never Mutex for stats)
3. **no_std** - rust_qp_zip must remain #![no_std] (no libstd dependency)
4. **Graceful fallback** - GPU failure falls back to CPU (never panic!)
5. **Non-blocking I/O** - Stratum reads use 1ms timeout + peek() pattern
6. **Pool-first** - All mining goes through public-pool.io Stratum V1
7. **Buffer pooling** - Pre-allocate buffers in hot paths, avoid String allocations

## Architecture v2.0

### Performance-Engine
`
main.rs -> pool_miner_loop()
  +-- spawn_miner_threads() (miner_core.rs - ALLE Kerne parallel)
  |     +-- Thread 0: nonces 0, N, 2N, ...
  |     +-- Thread 1: nonces 1, N+1, 2N+1, ...
  |     +-- Thread N: work-stealing via AtomicU64
  +-- AtomicU64 HASHES_TOTAL (lock-free)
  +-- AtomicU64 SHARES_ACCEPTED (lock-free)
  +-- Arc<AtomicBool> running (graceful shutdown)
  +-- TUI update thread (100ms rolling hashrate)
  +-- Stratum non-blocking job polling (1ms peek)
`

### Bugfixes (v2.0 final)
- wait_for_notify() method added (was missing in v1.0)
- miner_core.rs now implemented and USED (was empty+unused)
- **CRITICAL: false "Quick Pre-Filter" removed** (skipped 93.75% of nonces)
- **CRITICAL: Multi-Thread mining via miner_core.rs** (was single-thread!)
- **Stratum double-newline in send() fixed** (was sending 2 newlines)
- **Duplicate AtomicBool import removed** (compiler error)
- Unused ureq dependency removed
- start.bat: cd /d "%~dp0" added (works from any directory)
- unwrap() replaced with .ok().map() (no crash on Mutex-Poison)
- TUI Poison-Handling: Graceful Break instead of panic
- serde_derive -> serde (deprecated removed)
- Rolling Hashrate calculation (accurate, not fixed divisor)
- nbits_to_target() exported from both miner_core and stratum

### Performance (v2.0)
- Multi-Thread SHA-256d (ALL CPU cores via work-stealing)
- Atomic Counters (zero lock contention)
- Pre-allocated header buffers (no heap in hot path)
- 100% nonce validation (no false pre-filter)
- 1,000,000 nonces per batch (was 5000)

## Code Conventions
- Use Result<T, QPZipError> for library code
- #[cfg(feature = "opencl")] for OpenCL features
- TUI operations must be non-blocking (separate thread)
- Configuration via TOML (miner_config.toml)
- Atomic types for cross-thread statistics (no Mutex in hot paths)

## File Map
- rust_qp_zip/ - Post-quantum crypto, no_std
- qp_zip_miner/src/main.rs - Entry point, pool miner loop
- miner_modules/miner_core.rs - Multi-threaded CPU miner engine
- miner_modules/stratum.rs - Stratum V1 protocol handler
- miner_modules/tui.rs - ratatui Terminal UI
- miner_modules/vulkan.rs - Vulkan GPU detection
- miner_modules/config.rs - TOML config parser

## Agent Skills
See agent_skills/ directory:
- rust_repair_guide.md - Common compile errors & fixes
- stratum_protocol.md - Stratum V1 message flow & mining math
- vulkan_debug.md - Vulkan ash API, VRAM detection, CStr strings
