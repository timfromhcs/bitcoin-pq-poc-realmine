

pub struct VulkanEngine {
    pub available: bool,
    pub device_name: String,
    pub vram_mb: f64,
    pub system_ram_mb: f64,
    pub total_available_mb: f64,
    pub supports_fp64: bool,
}

impl VulkanEngine {
    pub fn new(device_index: i32) -> Self {
        match Self::init_vulkan(device_index) {
            Ok(engine) => engine,
            Err(e) => {
                eprintln!("Vulkan init failed: {}, fallback to CPU", e);
                Self { available: false, device_name: String::from("N/A"), vram_mb: 0.0, system_ram_mb: 0.0, total_available_mb: 0.0, supports_fp64: false }
            }
        }
    }

    fn init_vulkan(device_index: i32) -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load().map_err(|e| format!("Load: {}", e))?;
            let ac = std::ffi::CString::new("HCSminer v2.0").unwrap();
            let ec = std::ffi::CString::new("HCSminer v3.0 Engine").unwrap();
            let ai = ash::vk::ApplicationInfo::builder()
                .application_name(&ac).application_version(ash::vk::make_api_version(0,2,0,0))
                .engine_name(&ec).engine_version(ash::vk::make_api_version(0,2,0,0))
                .api_version(ash::vk::make_api_version(0,1,3,0));
            let ci = ash::vk::InstanceCreateInfo::builder().application_info(&ai);
            let instance = entry.create_instance(&ci, None).map_err(|e| format!("Create: {}", e))?;
            let devices = instance.enumerate_physical_devices().map_err(|e| format!("Enum: {}", e))?;
            if devices.is_empty() { return Err(String::from("No devices")); }
            let idx = if device_index >= 0 && (device_index as usize) < devices.len() { device_index as usize } else { 0 };
            let pd = devices[idx];
            let props = instance.get_physical_device_properties(pd);
            let mp = instance.get_physical_device_memory_properties(pd);
            let dn = { let s = &props.device_name; std::ffi::CStr::from_ptr(s.as_ptr()).to_string_lossy().to_string() };
            let mut vram = 0.0;
            let mut total_heaps = 0.0;
            for i in 0..(mp.memory_heap_count as usize).min(32) {
                let hs = (mp.memory_heaps[i].size as f64) / 1048576.0;
                total_heaps += hs;
                if mp.memory_heaps[i].flags.contains(ash::vk::MemoryHeapFlags::DEVICE_LOCAL) {
                    if hs > vram { vram = hs; }
                }
            }
            if vram < 1024.0 && total_heaps > vram { vram = total_heaps; }
            let feat = instance.get_physical_device_features(pd);
            instance.destroy_instance(None);
            let fp64 = feat.shader_float64 == ash::vk::TRUE;
            let sys_ram = (vram * 1.33).min(16000.0);
            Ok(VulkanEngine {
                available: true, device_name: dn, vram_mb: vram,
                system_ram_mb: sys_ram, total_available_mb: vram + sys_ram,
                supports_fp64: fp64
            })
        }
    }
}
