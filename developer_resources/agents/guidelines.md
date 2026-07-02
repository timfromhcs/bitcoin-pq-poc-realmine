# Developer Agent Guidelines

## For AI agents working on the HCSminer codebase

### Before Editing
1. Read `AGENTS.md` for project conventions
2. Check `agent_skills/` for Rust repair patterns, Stratum protocol, Vulkan tips
3. Run `cargo check` before and after changes
4. Always build both debug and release

### When Adding Features
1. Add new modules to `miner_modules/`
2. Register in `mod.rs`
3. Keep pool mining working (Stratum V1 @ public-pool.io:13333)
4. Non-blocking I/O for all network operations
5. TUI updates go through `TuiState` struct (Arc<Mutex<>>)

### Error Recovery
- Pool disconnect → auto-reconnect every 3s
- GPU failure → CPU fallback (inform user via TUI log)
- Config missing → create with defaults
