# BIP-QP-ZIP Development Guidelines & Latest State

This workspace integrates the **BIP-QP-ZIP** (Quantum-Proof Zero-Knowledge Inflight Processing) Proof of Concept directly into Bitcoin Core. It features a Rust-based cryptographic library, C++ consensus modifications, an automated unit test suite, and a GPU-accelerated AMD Radeon miner with a Web UI.

---

## 1. Latest Project State

### Cryptographic Core (`src/rust_qp_zip`)
- **Lattice Quantization (`quantizer.rs`)**: Performs signed modular coordinate mapping centered around zero to preserve negative vector values.
- **ZK-SNARK Prover (`zk_prover.rs`)**: Computes Fiat-Shamir commitments of quantized signatures and transaction messages.
- **Extractor (`extractor.rs`)**: Manages in-memory signature reconstruction.
- **Build Output**: Generates `staticlib` (for C++ linking), `cdylib`, and `rlib` (for Rust miner linking).

### Bitcoin Core Integration (C++ Main Files)
- **`src/script/interpreter.h`**: Declares `SCRIPT_VERIFY_QPZIP` consensus script verify flag.
- **`src/script/interpreter.cpp`**: Enforces Witness Version 2 validation using `VerifyQPZipWitnessProgram`.
- **`src/validation.cpp`**: Activates the `SCRIPT_VERIFY_QPZIP` flag by default.
- **`src/init.cpp`**: Registers `ShutdownQPZipLibrary()` on node exit to prevent memory leaks.
- **Legacy Backups**: Original main consensus files are backed up in `backup_legacy_main_files/`.

### Unit Tests (`src/test/qpzip_tests.cpp`)
- 5 comprehensive Boost test cases validating quantization roundtrips, ZK proof safety, soft-fork compatibility, storage savings (29.66%), and CPU validation speed (1.37 microseconds).
- Registered in `src/test/CMakeLists.txt` and runs via `test_bitcoin` binary.

### GPU Miner & Web UI (`src/qp_zip_miner`)
- Native Windows CPU/GPU miner hosting a local Web UI on `http://localhost:3000`.
- Connects to Bitcoin Mainnet Stratum pool `solo.ckpool.org:3333` with local simulation fallback.
- Leverages GPU parallel hashing via OpenCL (`opencl3` version `0.12.3` with `dynamic` loading feature) to run parallel threads on AMD Radeon graphics cards on Windows.
- Automatically executes the native `rust_qp_zip` FFI layer upon block solutions.
- Packaged with a launch batch file `start.bat` in the root directory.

---

## 2. Rules and Constraints for Future Developer Agents

### rule 1: Consensus Backups
Always copy the original file to `backup_legacy_main_files/` before modifying any main C++ Bitcoin Core files (e.g., in `src/script/`, `src/validation/`, or `src/init/`).

### rule 2: Crate Compilation Types
The `src/rust_qp_zip/Cargo.toml` library block MUST maintain the following configuration to ensure both C++ CMake and the Rust miner compile cleanly:
```toml
[lib]
name = "rust_qp_zip"
crate-type = ["staticlib", "cdylib", "rlib"]
```

### rule 3: Signed Coordinate Reconstruction
During signature verification, the public key commitment hash MUST be matched against the *reconstructed* floating-point signature coordinates. In modular arithmetic, mapped positive integer coefficients `q` exceeding `LATTICE_MODULUS / 2` must be centered back to negative range:
```rust
if q_val > LATTICE_MODULUS / 2 {
    q_val -= LATTICE_MODULUS;
}
```

### rule 4: Safe OpenCL and FFI Interfacing
All OpenCL calls (buffers creation, write, execute kernel, read) and FFI calls in the miner source code must be wrapped in `unsafe` blocks to ensure compatibility with modern `opencl3` versions.

### rule 5: Port Binding and File Locks
Before rebuilding or executing the Windows miner:
- Terminate any running `qp_zip_miner.exe` process to unlock the binary file.
- PowerShell command:
  ```powershell
  Stop-Process -Name qp_zip_miner -Force -ErrorAction SilentlyContinue
  ```
- This prevents port binding conflicts on port `3000`.
