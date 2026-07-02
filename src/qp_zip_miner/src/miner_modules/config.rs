use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub btc_address: String,
    pub worker_name: String,
    pub pool_host: String,
    pub pool_port: u16,
    pub use_tls: bool,
    pub threads: usize,
    pub quantization_depth: f64,
    pub vulkan_device_index: i32,
    pub enable_tui: bool,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            btc_address: String::from("bc1qzscjusprz5wwhxspgeyhlufqp3g8uxappsjm2w"),
            worker_name: String::from("hcsminer"),
            pool_host: String::from("public-pool.io"),
            pool_port: 13333,
            use_tls: false,
            threads: num_cpus::get(),
            quantization_depth: 1024.0,
            vulkan_device_index: -1,
            enable_tui: true,
        }
    }
}

impl MinerConfig {
    pub fn load(path: &str) -> Self {
        if Path::new(path).exists() {
            if let Ok(c) = fs::read_to_string(path) {
                if let Ok(cfg) = toml::from_str(&c) { return cfg; }
            }
        }
        let cfg = MinerConfig::default();
        let _ = cfg.save(path);
        cfg
    }
    pub fn save(&self, path: &str) -> Result<(), String> {
        fs::write(path, toml::to_string_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
}
