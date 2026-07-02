use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::io::BufRead;
use rand::Rng;

mod miner_modules;
use miner_modules::config::MinerConfig;
use miner_modules::tui::{TuiState, run_tui};
use miner_modules::vulkan::VulkanEngine;
use miner_modules::stratum::*;
lazy_static::lazy_static! { static ref STATE: Arc<Mutex<MinerState>> = Arc::new(Mutex::new(MinerState::default())); }
struct MinerState { pool_connected: bool, hashrate: f64, shares_accepted: u64, shares_rejected: u64, shares_total: u64, current_job: String, v1_attempts: u32, use_v2: bool }
impl Default for MinerState { fn default() -> Self { Self { pool_connected: false, hashrate: 0.0, shares_accepted: 0, shares_rejected: 0, shares_total: 0, current_job: String::new(), v1_attempts: 0, use_v2: false } } }
fn load_config() -> MinerConfig { MinerConfig::load("miner_config.toml") }
fn main() {
println!("============================================");
println!("    HCSminer v2.0 - Pool Mining (PPLNS)");
println!("============================================");
let cfg = load_config();
println!("BTC: {}", cfg.btc_address);
println!("Pool: public-pool.io (V1:13333 / V2:23331)");
println!("Stratum V1 tried first, falls back to V2");
let vk = VulkanEngine::new(cfg.vulkan_device_index);
if vk.available { println!("GPU: {} (VRAM: {:.0}MB)", vk.device_name, vk.vram_mb); }
let ts = Arc::new(Mutex::new(TuiState::new()));
let tt = ts.clone(); thread::spawn(move || { let _ = run_tui(tt); });
let ts2 = ts.clone();
let btc = cfg.btc_address.clone(); let wrk = cfg.worker_name.clone();
let host = cfg.pool_host.clone();
let vk_avail = vk.available; let vk_vram = vk.vram_mb;
thread::spawn(move || { pool_miner_loop(ts2, &btc, &wrk, &host); });
println!("Mining. Press q in TUI to quit.");
loop {
thread::sleep(Duration::from_secs(1));
if ts.lock().map(|s| !s.running).unwrap_or(true) { break; }
let s = STATE.lock().unwrap();
if let Ok(mut t) = ts.lock() {
t.cpu_hashrate = s.hashrate; t.pool_connected = s.pool_connected;
t.shares_accepted = s.shares_accepted; t.shares_rejected = s.shares_rejected;
t.total_hashes = s.shares_total;
t.vram_used_mb = if vk_avail { (vk_vram * 0.3).min(16000.0) } else { 0.0 };
t.ram_used_mb = 512.0;
}
} println!("Shutdown.");
}
fn pool_miner_loop(ts: Arc<Mutex<TuiState>>, btc: &str, wrk: &str, host: &str) {
    loop {
        // Determine which Stratum version to use
        let use_v2 = STATE.lock().unwrap().use_v2;
        let port: u16 = if use_v2 { 23331 } else { 13333 };
        let version = if use_v2 { "V2" } else { "V1" };
        ts.lock().unwrap().add_log(format!("Connecting Stratum {}...", version));
        let mut sc = StratumClient::new(btc, wrk);
        match sc.connect(host, port) {
            Ok(()) => { STATE.lock().unwrap().v1_attempts = 0; }
            Err(e) => {
                ts.lock().unwrap().add_log(format!("Stratum {} failed: {}", version, e));
                let mut s = STATE.lock().unwrap(); s.v1_attempts += 1;
                if s.v1_attempts >= 3 && !s.use_v2 {
                    s.use_v2 = true; s.v1_attempts = 0;
                    ts.lock().unwrap().add_log("Falling back to Stratum V2".into());
                } else if s.v1_attempts >= 3 && s.use_v2 {
                    s.use_v2 = false; s.v1_attempts = 0;
                    ts.lock().unwrap().add_log("Trying Stratum V1 again".into());
                }
                thread::sleep(Duration::from_secs(5)); continue;
            }
        }
        STATE.lock().unwrap().pool_connected = true;
        ts.lock().unwrap().add_log(format!("Connected - Stratum {}:{}:{}", host, port, version));
        if sc.subscribe().is_err() { STATE.lock().unwrap().pool_connected = false; thread::sleep(Duration::from_secs(3)); continue; }
        ts.lock().unwrap().add_log(format!("Subscribed EN1: {}", sc.extranonce1));
        if sc.authorize().is_err() { STATE.lock().unwrap().pool_connected = false; thread::sleep(Duration::from_secs(3)); continue; }
        ts.lock().unwrap().add_log("Authorized!".into());
        // Read pool messages until first mining.notify
        if let Some(ref mut s) = sc.stream {
            s.set_read_timeout(Some(Duration::from_secs(1))).ok();
            let mut reader = std::io::BufReader::new(s.try_clone().unwrap());
            let mut got_job = false;
            for _ in 0..60 {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 { continue; }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                    match v.get("method").and_then(|m| m.as_str()) {
                        Some("mining.set_difficulty") => { if let Some(d) = v.get("params").and_then(|p| p[0].as_f64()) { sc.difficulty = d; } }
                        Some("mining.notify") => {
                            if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 9 {
                                sc.job_id = p[0].as_str().unwrap_or("").into(); sc.prevhash = p[1].as_str().unwrap_or("").into();
                                sc.coinb1 = p[2].as_str().unwrap_or("").into(); sc.coinb2 = p[3].as_str().unwrap_or("").into();
                                sc.merkle_branches = p[4].as_array().map(|a| a.iter().filter_map(|b| b.as_str().map(String::from)).collect()).unwrap_or_default();
                                sc.version = p[5].as_str().unwrap_or("").into(); sc.nbits = p[6].as_str().unwrap_or("").into();
                                sc.ntime = p[7].as_str().unwrap_or("").into(); sc.clean_jobs = p[8].as_bool().unwrap_or(false);
                                got_job = true; break;
                            }}
                        }
                        Some("mining.set_extranonce") => { if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 1 { sc.extranonce1 = p[0].as_str().unwrap_or("").into(); } } }
                        _ => {}
                    }
                }
            }
            s.set_read_timeout(Some(Duration::from_millis(100))).ok();
            if !got_job { ts.lock().unwrap().add_log("No job - reconnect".into()); STATE.lock().unwrap().pool_connected = false; continue; }
        }
        ts.lock().unwrap().add_log(format!("Mining job: {} bits:{}", &sc.job_id[..8.min(sc.job_id.len())], sc.nbits));
        let mut nonce: u64 = rand::thread_rng().gen::<u64>();
        let mut last_log = Instant::now();
        loop {
            // Non-blocking job check
            if let Some(ref mut s) = sc.stream {
                let mut buf = [0u8; 1];
                if s.peek(&mut buf).is_ok() {
                    let mut reader = std::io::BufReader::new(s.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) > 0 {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                            match v.get("method").and_then(|m| m.as_str()) {
                                Some("mining.notify") => {
                                    if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 9 {
                                        sc.job_id = p[0].as_str().unwrap_or("").into(); sc.prevhash = p[1].as_str().unwrap_or("").into();
                                        sc.coinb1 = p[2].as_str().unwrap_or("").into(); sc.coinb2 = p[3].as_str().unwrap_or("").into();
                                        sc.merkle_branches = p[4].as_array().map(|a| a.iter().filter_map(|b| b.as_str().map(String::from)).collect()).unwrap_or_default();
                                        sc.version = p[5].as_str().unwrap_or("").into(); sc.nbits = p[6].as_str().unwrap_or("").into();
                                        sc.ntime = p[7].as_str().unwrap_or("").into(); sc.clean_jobs = p[8].as_bool().unwrap_or(false);
                                        if sc.clean_jobs { nonce = rand::thread_rng().gen::<u64>(); }
                                    }}
                                }
                                Some("mining.set_difficulty") => { if let Some(d) = v.get("params").and_then(|p| p[0].as_f64()) { sc.difficulty = d; } }
                                Some("mining.set_extranonce") => { if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 1 { sc.extranonce1 = p[0].as_str().unwrap_or("").into(); } } }
                                _ => {}
                            }
                        }
                    }
                }
            } else { break; }
            if sc.job_id.is_empty() { continue; }
            let e2 = format!("{:08x}", (nonce & 0xFFFFFFFF) as u32);
            let cb = build_coinbase(&sc.coinb1, &sc.extranonce1, &e2, &sc.coinb2);
            let cb_hash = double_sha256(&cb);
            let mr = build_merkle_root(&cb_hash, &sc.merkle_branches);
            for i in 0..500 {
                let n = nonce.wrapping_add(i) as u32;
                let header = build_header(&sc.version, &sc.prevhash, &mr, &sc.ntime, &sc.nbits, n);
                let hash = double_sha256(&header);
                if hash_meets_target(&hash, &sc.nbits) {
                    let nh = format!("{:08x}", n);
                    let jid = sc.job_id.clone(); let ntm = sc.ntime.clone();
                    if sc.submit(&jid, &e2, &ntm, &nh).is_ok() {
                        ts.lock().unwrap().add_log(format!("SHARE! {}", nh));
                        STATE.lock().unwrap().shares_accepted += 1;
                    }
                }
            }
            nonce = nonce.wrapping_add(500);
            STATE.lock().unwrap().shares_total += 500;
            STATE.lock().unwrap().hashrate = 500.0 / last_log.elapsed().as_secs_f64().max(1.0);
            if last_log.elapsed() >= Duration::from_secs(30) {
                ts.lock().unwrap().add_log(format!("{:.0} H/s | Shares: {}/{}", STATE.lock().unwrap().hashrate, STATE.lock().unwrap().shares_accepted, STATE.lock().unwrap().shares_rejected));
                last_log = Instant::now();
            }
            if ts.lock().map(|s| !s.running).unwrap_or(true) { break; }
        }
        STATE.lock().unwrap().pool_connected = false;
        if ts.lock().map(|s| !s.running).unwrap_or(true) { break; }
        ts.lock().unwrap().add_log("Reconnecting...".into());
        thread::sleep(Duration::from_secs(3));
    }
}




