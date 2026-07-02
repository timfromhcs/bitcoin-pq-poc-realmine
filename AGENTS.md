# AGENTS.md - AI Coding Assistant Guidelines

## Project Overview
HCSminer - a post-quantum Bitcoin mining system:
- rust_qp_zip (no_std Rust library): Lattice quantization, ZK proofs, extraction
- qp_zip_miner (Rust binary): GPU miner with Vulkan, TUI, probabilistic pre-filtering

Made with love by timfromhcs and @hcmedia.

## Key Principles
1. No panics in library code - all operations fallible with error propagation
2. FFI safety - validate pointers and lengths before dereferencing
3. no_std - rust_qp_zip must remain #![no_std]
4. Graceful fallback - GPU failure falls back to CPU
5. Probabilistic optimization - pre-filter nonces before SHA-256

## Code Conventions
- Use Result<T, QPZipError> for library
- OpenCL behind #[cfg(feature = "opencl")]
- Vulkan via optional ash dependency
- TUI operations must be non-blocking
- Configuration via TOML

## Architecture
- rust_qp_zip never depends on std
- Miner imports from miner_modules/
- Vulkan detection is graceful
- RPC failure is non-fatal
