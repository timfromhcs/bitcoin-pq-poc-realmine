use crate::miner_modules::config::MinerConfig;
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Instant;
use crate::miner_modules::tui::TuiState;
const PROBABILISTIC_HASH_SEED: u64 = 0x9e3779b97f4a7c15;

pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let h1 = hasher.finalize();
    let mut hasher2 = Sha256::new();
    hasher2.update(h1);
    hasher2.finalize().into()
}

#[inline]
pub fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in 0..32 {
        if hash[i] > target[i] { return false; }
        if hash[i] < target[i] { return true; }
    }
    true
}

#[inline]
pub fn probabilistic_pre_filter(nonce: u64) -> bool {
    let mut h = nonce.wrapping_mul(PROBABILISTIC_HASH_SEED);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h & 0x7FF) < 1
}


pub fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn bytes_to_hex_target(bits: u32) -> [u8; 32] {
    let mut target = [0u8; 32];
    let exp = (bits >> 24) as usize;
    let mant = (bits & 0x007FFFFF) as u32;
    if exp <= 3 { return target; }
    let idx = exp - 3;
    if idx < 32 {
        target[idx] = ((mant >> 16) & 0xFF) as u8;
        if idx + 1 < 32 { target[idx + 1] = ((mant >> 8) & 0xFF) as u8; }
        if idx + 2 < 32 { target[idx + 2] = (mant & 0xFF) as u8; }
    }
    target
}

pub fn hash_below_network_target(hash: &[u8; 32], bits: u32) -> bool {
    let target = bytes_to_hex_target(bits);
    hash_meets_target(hash, &target)
}

pub fn submit_block(block_hex: &str, config: &MinerConfig) -> Result<(), String> {
    let url = format!("http://{}:{}", config.rpc_host, config.rpc_port);
    let auth = base64::encode(format!("{}:{}", config.rpc_user, config.rpc_password));
    let body = format!(r#"{{"jsonrpc":"1.0","id":"miner","method":"submitblock","params":["{}"]}}"#, block_hex);
    let response = ureq::post(&url)
        .set("Authorization", &format!("Basic {}", auth))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("RPC error: {}", e))?;
    if response.status() == 200 { Ok(()) }
    else { Err(format!("RPC returned {}", response.status())) }
}

pub fn get_block_template(config: &MinerConfig) -> Result<String, String> {
    let url = format!("http://{}:{}", config.rpc_host, config.rpc_port);
    let auth = base64::encode(format!("{}:{}", config.rpc_user, config.rpc_password));
    let body = r#"{"jsonrpc":"1.0","id":"miner","method":"getblocktemplate","params":[{"rules":["segwit"]}]}"#;
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Basic {}", auth))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| format!("RPC error: {}", e))?;
    resp.into_string().map_err(|e| format!("Read error: {}", e))
}

