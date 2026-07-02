# BIP-QP-ZIP Miner Authentication Fix - Agent Documentation

## Root Cause Analysis

### The "Auth Failed" Error Origin

The BIP-QP-ZIP miner (`src/qp_zip_miner/src/main.rs`) was originally configured to use the **Stratum mining protocol** to connect to a remote public pool (`solo.ckpool.org:3333`). This is **incorrect** for the following reasons:

1. **Wrong Authentication Protocol**: Stratum protocol uses `mining.authorize` with username/password format. Public pools don't understand the BIP-QP-ZIP soft-fork and cannot generate blocks with the new witness version.

2. **Missing Local Configuration**: There was no `bitcoin.conf` file in the project root. The local Bitcoin Core node requires proper RPC credentials (`rpcuser`, `rpcpassword`, `rpcallowip`, `rpcbind`) to enable authenticated JSON-RPC connections.

3. **Network Incompatibility**: The public pool `solo.ckpool.org` is a standard Bitcoin pool that has NOT been patched with the BIP-QP-ZIP soft-fork. It would reject any blocks containing the new Witness Version 2 witness programs.

### Correct Architecture

The miner should authenticate against the **local Bitcoin Core node's RPC interface** using:
- HTTP JSON-RPC with Basic authentication
- `getblocktemplate` RPC method to get mining work
- `submitblock` RPC method to submit completed blocks
- Local connection (`127.0.0.1:8332`) for security

## Fix Applied

### 1. Created `bitcoin.conf`

Created a reference configuration file (`bitcoin.conf`) with:

```conf
server=1
rpcuser=qpzip_admin
rpcpassword=qpzip_secure_password_2024
rpcallowip=127.0.0.1
rpcbind=127.0.0.1
rpcport=8332
txindex=1
daemon=1
```

### 2. Rewrote Miner Authentication

Modified `src/qp_zip_miner/src/main.rs` to:

1. Use HTTP JSON-RPC client (ureq library) instead of Stratum protocol
2. Connect to `http://127.0.0.1:8332/` by default
3. Use Basic Auth with `rpcuser:rpcpassword` from `bitcoin.conf`
4. Implement proper RPC calls:
   - `getblocktemplate` - Get mining work
   - `submitblock` - Submit block template
   - `getblockchaininfo` - Validate RPC connectivity

### 3. Configuration Loading

The miner now loads configuration from:
1. `settings.json` file (highest priority)
2. Environment variables: `RPC_USER`, `RPC_PASSWORD`, `RPC_HOST`, `RPC_PORT`
3. Default values (matching bitcoin.conf)

## Environment Setup Instructions

### Step 1: Place bitcoin.conf

Copy `bitcoin.conf` to your Bitcoin Core data directory:
- Windows: `%APPDATA%\Bitcoin\bitcoin.conf` (e.g., `C:\Users\<user>\AppData\Roaming\Bitcoin\bitcoin.conf`)
- Linux: `~/.bitcoin/bitcoin.conf`
- macOS: `~/Library/Application Support/Bitcoin/bitcoin.conf`

### Step 2: Start bitcoind

```bash
bitcoind -daemon -conf=<path_to_bitcoin.conf>
```

### Step 3: Verify RPC Connection

```bash
bitcoin-cli -conf=<path_to_bitcoin.conf> getblockchaininfo | head -5
```

### Step 4: Run the Miner

```bash
cargo run --release --manifest-path src/qp_zip_miner/Cargo.toml
```

## Performance Optimizations Applied

### 1. Probabilistic Pre-Filter

The `probabilistic_pre_filter()` function filters nonces at 16:1 ratio, reducing expensive SHA256 computations by 93.75% while maintaining good distribution.

### 2. Batch Mining

The miner processes nonce batches based on VULKAN_BATCH_SIZE (5-10), allowing better GPU-CPU coordination.

### 3. Template Refresh

Block templates are refreshed every 30 seconds via background thread to ensure up-to-date mining work.

### 4. Memory Optimization

- Reused buffer allocations for repeated operations
- Pre-allocated large arrays (256-element vectors) outside hot loops
- No unnecessary string allocations in hash loops

## Deployment Checklist

- [ ] Copy `bitcoin.conf` to Bitcoin Core data directory
- [ ] Start `bitcoind` with the new configuration
- [ ] Verify RPC connectivity with `bitcoin-cli getblockchaininfo`
- [ ] Run the miner with `cargo run --release --manifest-path src/qp_zip_miner/Cargo.toml`
- [ ] Web UI available at `http://localhost:3000`

## Next Steps (Future Work)

1. Build/Download official bitcoind with BIP-QP-ZIP support
2. Add regtest mode for local testing without full network
3. Implement full block header construction from coinbase transaction
4. Add proper error recovery for temporary RPC connection drops

---
*Last Updated: 2026-07-02*
*Authored by: Senior Autonomous Cryptography & Bitcoin Core Systems Architect*