# HCSminer v2.0 Architecture

## High-Level Flow
`
User edits miner_config.toml (BTC address)
       |
       v
HCSminer starts
  +-- Vulkan GPU detection (optional, never blocks)
  +-- Start TUI thread (separate thread, non-blocking)
  +-- Start TUI update thread (100ms rolling hashrate)
  +-- Start pool_miner_loop (separate thread)
  |     +-- Connect Stratum V1 to public-pool.io:13333
  |     +-- Subscribe & Authorize
  |     +-- Mining Loop (1M nonces/batch):
  |     |     +-- Check for new jobs (non-blocking peek, 1ms)
  |     |     +-- Spawn N miner threads via miner_core::spawn_miner_threads()
  |     |     |     +-- Thread 0: nonce 0, N, 2N...
  |     |     |     +-- Thread 1: nonce 1, N+1, 2N+1...
  |     |     |     +-- Thread N-1: nonce N-1, 2N-1, 3N-1...
  |     |     +-- Full SHA-256d for EVERY nonce (no pre-filter!)
  |     |     +-- If share found: submit to pool
  |     |     +-- AtomicU64 counters (lock-free)
  |     +-- On disconnect: reconnect after 3s
  +-- Main thread waits for shutdown signal
`

## Performance Architecture

### Lock-Free Statistics
`
+-----------------------------+
|  AtomicU64 HASHES_TOTAL     | <- Updated by miner threads
|  AtomicU64 SHARES_ACCEPTED  | <- Updated on share found
|  AtomicU64 SHARES_REJECTED  | <- Updated on reject
|  AtomicBool POOL_CONNECTED  | <- Updated on connect/disconnect
+-----------------------------+
         | Read every 100ms
         v
+-----------------------------+
|  TUI Update Thread          |
|  - Rolling hashrate avg     |
|  - Share counts             |
|  - Pool status              |
+-----------------------------+
`

### Mining Hot-Path (v2.0 - fixed)
`
Job received -> Build header_base (76 bytes, pre-header)
             -> Spawn N miner threads (1000 nonces/batch each)
             -> Each thread:
                 1. Copy header_base -> header[80]
                 2. Set nonce in header[76..80]
                 3. SHA-256(header) -> hash1
                 4. SHA-256(hash1) -> hash
                 5. Compare hash with target
                 6. If found: increment AtomicU64 shares_found
                 7. Increment AtomicU64 total_hashes
             -> Poll for completion (200ms timeout)
             -> Update global counters
             -> Every 30s: log summary
`

## Component Overview

### qp_zip_miner (Rust binary)
- main.rs - Entry point, pool miner loop, TUI monitoring
- miner_modules/miner_core.rs - Multi-threaded CPU miner (spawn_miner_threads, mine_batch, nbits_to_target)
- miner_modules/stratum.rs - Stratum V1 TCP protocol
- miner_modules/tui.rs - ratatui Terminal User Interface
- miner_modules/vulkan.rs - Vulkan GPU enumeration
- miner_modules/config.rs - TOML configuration

### rust_qp_zip (no_std library)
- quantizer.rs - Lattice-based vector quantization
- zk_prover.rs - ZK-SNARK proof generation/verification
- extractor.rs - Witness program extraction
- ffi.rs - C-FFI bindings

## System Requirements
- Windows 10/11 x64
- Rust 1.75+
- 4+ CPU cores (for multi-thread mining)
- 8GB+ RAM recommended
- Vulkan-compatible GPU (optional)
