# HCSminer Architecture

## High-Level Flow
```
User edits miner_config.toml (BTC address)
       |
       v
HCSminer starts
  ├── Vulkan GPU detection (optional)
  ├── Stratum V1 connect to public-pool.io:13333
  ├── Subscribe & Authorize
  ├── Start mining loop (non-blocking job polling)
  │     ├── Every 500 hashes: check for new job
  │     ├── If share found: submit to pool
  │     └── If pool disconnected: reconnect after 3s
  └── TUI updates every 1s (hashrate, shares, pool status)
```

## Component Overview

### qp_zip_miner (Rust binary)
- `main.rs` - Entry point, pool miner loop, TUI monitoring
- `miner_modules/stratum.rs` - Stratum V1 TCP protocol
- `miner_modules/tui.rs` - ratatui Terminal User Interface
- `miner_modules/vulkan.rs` - Vulkan GPU enumeration
- `miner_modules/config.rs` - TOML configuration

### rust_qp_zip (no_std library)
- `quantizer.rs` - Lattice-based vector quantization
- `zk_prover.rs` - ZK-SNARK proof generation/verification
- `extractor.rs` - Witness program extraction
- `ffi.rs` - C-FFI bindings
