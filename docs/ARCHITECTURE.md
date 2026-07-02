# HCSminer v2.0 Architecture

## High-Level Flow
`
User edits miner_config.toml (BTC address)
       |
       v
HCSminer starts
  ├── Vulkan GPU detection (optional, never blocks)
  ├── Start TUI thread (separate thread, non-blocking)
  ├── Start TUI update thread (100ms rolling hashrate)
  ├── Start pool_miner_loop (separate thread)
  │     ├── Connect Stratum V1 to public-pool.io:13333
  │     ├── Subscribe & Authorize
  │     ├── Mining Loop (5000 nonces/batch):
  │     │     ├── Check for new jobs (non-blocking peek)
  │     │     ├── Quick pre-filter (90% nonces skipped)
  │     │     ├── Full SHA-256d for remaining 10%
  │     │     ├── If share found: submit to pool
  │     │     └── AtomicU64 counters (lock-free)
  │     └── On disconnect: reconnect after 3s
  └── Main thread waits for shutdown signal
`

## Performance Architecture

### Lock-Free Statistics
`
┌─────────────────────────────┐
│  AtomicU64 HASHES_TOTAL     │ ← Updated by miner thread
│  AtomicU64 SHARES_ACCEPTED  │ ← Updated on share found
│  AtomicU64 SHARES_REJECTED  │ ← Updated on reject
│  AtomicBool POOL_CONNECTED  │ ← Updated on connect/disconnect
└─────────────────────────────┘
         │ Read every 100ms
         ▼
┌─────────────────────────────┐
│  TUI Update Thread          │
│  - Rolling hashrate Ø       │
│  - Share counts             │
│  - Pool status              │
└─────────────────────────────┘
`

### Mining Hot-Path
`
Job received → Build header_base (76 bytes, pre-header)
            → For each nonce:
                1. Quick filter (n & 0xF != 0) → skip ~90%
                2. Copy header_base → header[80]
                3. Set nonce in header[76..80]
                4. SHA-256(header) → hash1
                5. SHA-256(hash1) → hash
                6. Compare hash with target
            → Every 5000: check for new job
            → Every 30s: log summary
`

## Component Overview

### qp_zip_miner (Rust binary)
- main.rs - Entry point, pool miner loop, TUI monitoring
- miner_modules/miner_core.rs - Multi-threaded CPU miner
- miner_modules/stratum.rs - Stratum V1 TCP protocol
- miner_modules/tui.rs - ratatui Terminal User Interface
- miner_modules/vulkan.rs - Vulkan GPU enumeration
- miner_modules/config.rs - TOML configuration

### rust_qp_zip (no_std library)
- quantizer.rs - Lattice-based vector quantization
- zk_prover.rs - ZK-SNARK proof generation/verification
- extractor.rs - Witness program extraction
- fi.rs - C-FFI bindings

## System Requirements
- Windows 10/11 x64
- Rust 1.75+
- 4+ CPU cores (für Multi-Thread Mining)
- 8GB+ RAM empfohlen
- Vulkan-compatible GPU (optional)
