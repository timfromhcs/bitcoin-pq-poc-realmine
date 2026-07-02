# HCSminer - Agent Skills & Quick Reference

Made with love by timfromhcs and @hcmedia

## Quick Command Reference

### Build & Test
- `cargo check` - Quick compile check (30s)
- `cargo build` - Debug build (2min)
- `cargo build --release` - Release build (5min)
- `cargo fix` - Auto-fix warnings

### Key Files
- `src/qp_zip_miner/src/main.rs` - Entry point, pool miner loop
- `src/qp_zip_miner/src/miner_modules/stratum.rs` - Stratum V1 protocol
- `src/qp_zip_miner/src/miner_modules/tui.rs` - Terminal UI (ratatui)
- `src/qp_zip_miner/src/miner_modules/vulkan.rs` - Vulkan GPU detection
- `src/qp_zip_miner/src/miner_modules/config.rs` - TOML config parser
- `src/rust_qp_zip/src/` - Post-quantum crypto library (no_std)
- `miner_config.toml` - User configuration (edit BTC address)

## Common Fix Patterns

### Fix E0502 (borrow conflicts)
- Clone fields before calling methods: `let jid = sc.job_id.clone(); sc.submit(&jid)`
- Use `let x = value.clone()` to break simultaneous immutable+mutable borrows

### Fix E0432 (unresolved imports)
- Add missing `pub mod xyz;` in `mod.rs`
- Check `Cargo.toml` for missing dependencies

### Fix network timeouts
- Use non-blocking `peek()` for Stratum job checking
- Set `read_timeout(Some(Duration::from_millis(100)))` for non-blocking mode

## Stratum V1 Protocol Flow
1. TCP connect → port 13333
2. Send: `{"id":1,"method":"mining.subscribe","params":["HCSminer/2.0"]}`
3. Recv: extranonce1 + extranonce2_size
4. Send: `{"id":2,"method":"mining.authorize","params":["btc.worker","x"]}`
5. Recv: `{"id":null,"method":"mining.set_difficulty","params":[diff]}`
6. Recv: `{"id":null,"method":"mining.notify","params":[job_id,prevhash,coinb1,coinb2,merkle_branches,version,nbits,ntime,clean]}`
7. Build coinbase → double SHA256 → merkle root → block header
8. Hash header, check target, submit share

## Architecture Rules
- `rust_qp_zip` = no_std library, NEVER depend on std
- GPU failure = graceful fallback to CPU, never crash
- OpenCL gated behind `#[cfg(feature = "opencl")]`
- TUI operations must be non-blocking
- Pool RPC failure = retry, never panic

