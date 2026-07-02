use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub wallet: String,
    pub rpc_host: String,
    pub rpc_port: u16,
    pub rpc_user: String,
    pub rpc_password: String,
    pub threads: usize,
    pub quantization_depth: f64,
    pub probabilistic_threshold: f64,
    pub vulkan_device_index: i32,
    pub memory_offload_threshold_mb: usize,
    pub enable_tui: bool,
    pub cpu_mining: bool,
    pub gpu_mining: bool,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            wallet: String::from("bc1q5d7026rlav5t9whw55648a04n05apzxlxlq27p"),
            rpc_host: String::from("127.0.0.1"),
            rpc_port: 8332,
            rpc_user: String::from("qpzip_admin"),
            rpc_password: String::from("qpzip_secure_password_2024"),
            threads: num_cpus::get(),
            quantization_depth: 1024.0,
            probabilistic_threshold: 0.05,
            vulkan_device_index: -1,
            memory_offload_threshold_mb: 512,
            enable_tui: true,
            cpu_mining: true,
            gpu_mining: true,
        }
    }
}

impl MinerConfig {
    pub fn load(path: &str) -> Self {
        if Path::new(path).exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(cfg) = toml::from_str(&content) {
                    return cfg;
                }
            }
        }
        let cfg = MinerConfig::default();
        let _ = cfg.save(path);
        cfg
    }
    pub fn save(&self, path: &str) -> Result<(), String> {
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}