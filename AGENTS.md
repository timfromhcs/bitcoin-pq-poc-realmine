# AGENTS.md - HCSminer AI Coding Assistant Guidelines

Made with love by timfromhcs and @hcmedia

## Project Overview
HCSminer - a post-quantum Bitcoin mining system for Pool-Mining (PPLNS @ public-pool.io):
- **rust_qp_zip** (no_std Rust library): Lattice quantization, ZK proofs, extraction
- **qp_zip_miner** (Rust binary): GPU miner with Vulkan, TUI, Stratum V1 pool mining

## Key Principles
1. **No panics in library code** - all operations fallible with error propagation
2. **FFI safety** - validate pointers and lengths before dereferencing
3. **no_std** - rust_qp_zip must remain `#![no_std]`
4. **Graceful fallback** - GPU failure falls back to CPU (never panic!)
5. **Non-blocking I/O** - Stratum reads use 100ms timeout + peek() pattern
6. **Pool-first** - all mining goes through public-pool.io Stratum V1

## Code Conventions
- Use `Result<T, QPZipError>` for library
- OpenCL behind `#[cfg(feature = "opencl")]`
- Vulkan via optional `ash` dependency (0.37)
- TUI operations must be non-blocking (separate thread)
- Configuration via TOML (`miner_config.toml`)

## Architecture
- `rust_qp_zip/` - Post-quantum crypto, never depends on std
- `qp_zip_miner/src/main.rs` - Entry point, pool miner loop
- `miner_modules/stratum.rs` - Stratum V1 protocol handler
- `miner_modules/tui.rs` - ratatui Terminal UI
- `miner_modules/vulkan.rs` - Vulkan GPU detection engine
- `miner_modules/config.rs` - TOML config parser

## Agent Skills & Shortcuts
See `agent_skills/` directory for detailed guides:
- `rust_repair_guide.md` - Common compile errors & fixes
- `stratum_protocol.md` - Stratum V1 message flow & mining math
- `vulkan_debug.md` - Vulkan ash API, VRAM detection, CStr strings
