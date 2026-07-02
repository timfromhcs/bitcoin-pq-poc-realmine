# Vulkan GPU Debug Guide

## VRAM Detection
For integrated GPUs (Ryzen 7000, shared memory):
- DEVICE_LOCAL heaps may report only 256MB
- Sum ALL heaps as fallback when VRAM < 1024MB
- Code in `vulkan.rs` handles this with `total_heaps` summation

## Entry API (ash 0.37)
- Use `ash::Entry::load()` which returns `Result`
- NOT `Entry::linked()` (not available in 0.37)
- NOT `Entry::new()` (returns Result in 0.37+)

## CStr Strings
```rust
let cstr = std::ffi::CString::new("My App").unwrap();
let ai = ash::vk::ApplicationInfo::builder()
    .application_name(&cstr)
    .engine_name(&std::ffi::CString::new("Engine").unwrap());
```

## Memory Properties
- Use `instance.get_physical_device_memory_properties(pd)`
- NOT `props.memory_properties` (struct field different in ash 0.37)
- `feat.shader_float64`, NOT `feat.shader_f64`
