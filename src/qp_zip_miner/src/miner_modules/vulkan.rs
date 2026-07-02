use std::sync::Arc;

pub struct VulkanEngine {
    pub available: bool,
    pub device_name: String,
    pub vram_mb: f64,
    pub supports_fp64: bool,
}

impl VulkanEngine {
    pub fn new(device_index: i32) -> Self {
        match Self::init_vulkan(device_index) {
            Ok(engine) => engine,
            Err(e) => {
                eprintln!("Vulkan init failed: {}, falling back to CPU-only", e);
                Self { available: false, device_name: String::from("N/A"), vram_mb: 0.0, supports_fp64: false }
            }
        }
    }

    fn init_vulkan(device_index: i32) -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load()
                .map_err(|e| format!("Failed to load Vulkan: {}", e))?;
            let app_cstr = std::ffi::CString::new("QP-ZIP Miner v2").unwrap();
            let engine_cstr = std::ffi::CString::new("GLM-5.2 MTP Engine").unwrap();
            let app_info = ash::vk::ApplicationInfo::builder()
                .application_name(&app_cstr)
                .application_version(ash::vk::make_api_version(0, 2, 0, 0))
                .engine_name(&engine_cstr)
                .engine_version(ash::vk::make_api_version(0, 5, 2, 0))
                .api_version(ash::vk::make_api_version(0, 1, 3, 0));
            let create_info = ash::vk::InstanceCreateInfo::builder()
                .application_info(&app_info);
            let instance = entry.create_instance(&create_info, None)
                .map_err(|e| format!("Failed: {}", e))?;
            let devices = instance.enumerate_physical_devices()
                .map_err(|e| format!("Enum failed: {}", e))?;
            if devices.is_empty() { return Err(String::from("No devices")); }
            let idx = if device_index >= 0 && (device_index as usize) < devices.len() {
                device_index as usize
            } else { 0 };
            let pd = devices[idx];
            let props = instance.get_physical_device_properties(pd);
            let mem_props = instance.get_physical_device_memory_properties(pd);
            let dn = { let s = &props.device_name; std::ffi::CStr::from_ptr(s.as_ptr()).to_string_lossy().to_string() };
            let mut vram = 0.0;
            for i in 0..(mem_props.memory_heap_count as usize).min(32) {
                if mem_props.memory_heaps[i].flags.contains(ash::vk::MemoryHeapFlags::DEVICE_LOCAL) {
                    vram = (mem_props.memory_heaps[i].size as f64) / (1024.0 * 1024.0);
                }
            }
            let feat = instance.get_physical_device_features(pd);
            instance.destroy_instance(None);
            let fp64 = feat.shader_float64 == ash::vk::TRUE;
            Ok(VulkanEngine { available: true, device_name: dn, vram_mb: vram, supports_fp64: fp64 })
        }
    }
}
