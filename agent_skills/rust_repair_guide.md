# Rust Repair Guide for HCSminer

## Common Compile Errors & Fixes

### E0061 - Wrong argument count
- ratatui `Layout::split()` takes 1 arg (use `.constraints([...]).split(area)`)
- crossterm `KeyCode::Char()` takes `char`, not `&str` (use single quotes)

### E0308 - Mismatched types
- `base64::encode` is deprecated in 0.21, use `Engine::encode`
- Vulkan `application_name()` needs `&CStr`, not `&[u8]`

### E0599 - No method found
- `ash::Entry::linked()` requires ash 0.38+; use `Entry::load()` for 0.37
- BufReader needs `use std::io::BufRead;` trait import

### E0425 - Cannot find function
- Feature-gated function behind `#[cfg(feature = "opencl")]` needs fallback
- Use `#[cfg(not(feature = "opencl"))]` for CPU fallback

## Safe Pattern: Non-blocking TCP
```rust
s.set_read_timeout(Some(Duration::from_millis(100))).ok();
match s.peek(&mut buf) {
    Ok(_) => { /* data available */ }
    Err(ref e) if e.kind() == WouldBlock => { /* no data */ }
    Err(e) => { /* error */ }
}
```
