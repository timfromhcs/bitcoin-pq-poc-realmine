use std::sync::{Arc, Mutex};
use std::thread;
use std::io::BufRead;
use std::time::{Duration, Instant};
use rand::Rng;

mod miner_modules;
use miner_modules::config::MinerConfig;
use miner_modules::tui::{TuiState, run_tui};
use miner_modules::vulkan::VulkanEngine;
use miner_modules::stratum::*;
lazy_static::lazy_static! { static ref STATE: Arc<Mutex<MinerState>> = Arc::new(Mutex::new(MinerState::default())); }
struct MinerState { is_mining: bool, hashrate: f64, shares_accepted: u64, shares_rejected: u64, shares_total: u64, current_job: String, pool_connected: bool }
impl Default for MinerState { fn default() -> Self { Self { is_mining: true, hashrate: 0.0, shares_accepted: 0, shares_rejected: 0, shares_total: 0, current_job: String::new(), pool_connected: false } } }
fn load_config() -> MinerConfig { MinerConfig::load("miner_config.toml") }
fn main() {
println!("============================================");
println!("    HCSminer v2.0 - Pool Mining (PPLNS)");
println!("============================================");
let cfg = load_config();
println!("BTC: {}  Pool: {}:{}", cfg.btc_address, cfg.pool_host, cfg.pool_port);
let vk = VulkanEngine::new(cfg.vulkan_device_index);
if vk.available { println!("GPU: {} (VRAM: {:.0}MB)", vk.device_name, vk.vram_mb); }
let ts = Arc::new(Mutex::new(TuiState::new()));
let tt = ts.clone(); thread::spawn(move || { let _ = run_tui(tt); });
let ts2 = ts.clone();
let btc = cfg.btc_address.clone(); let wrk = cfg.worker_name.clone();
let host = cfg.pool_host.clone(); let port = cfg.pool_port;
let vk_avail = vk.available; let vk_vram = vk.vram_mb;
thread::spawn(move || { pool_miner_loop(ts2, &btc, &wrk, &host, port); });
println!("Mining. Press q in TUI to quit.");
loop {
thread::sleep(Duration::from_secs(1));
if !ts.lock().unwrap().running { break; }
let s = STATE.lock().unwrap();
let mut t = ts.lock().unwrap();
t.cpu_hashrate = s.hashrate; t.pool_connected = s.pool_connected;
t.shares_accepted = s.shares_accepted; t.shares_rejected = s.shares_rejected;
t.total_hashes = s.shares_total;
t.vram_used_mb = if vk_avail { vk_vram * 0.3 } else { 0.0 }; t.ram_used_mb = 512.0;
} println!("Shutdown.");
}
fn pool_miner_loop(ts: Arc<Mutex<TuiState>>, btc: &str, wrk: &str, host: &str, port: u16) {
    loop {
        let mut sc = StratumClient::new(btc, wrk);
        ts.lock().unwrap().add_log("Connecting...".into());
        if sc.connect(host, port).is_err() { thread::sleep(Duration::from_secs(5)); continue; }
        STATE.lock().unwrap().pool_connected = true;
        ts.lock().unwrap().add_log("Connected. Subscribing...".into());
        if sc.subscribe().is_err() { STATE.lock().unwrap().pool_connected = false; thread::sleep(Duration::from_secs(3)); continue; }
        ts.lock().unwrap().add_log(format!("EN1: {}", sc.extranonce1));
        if sc.authorize().is_err() { STATE.lock().unwrap().pool_connected = false; thread::sleep(Duration::from_secs(3)); continue; }
        ts.lock().unwrap().add_log("Authorized!".into());
        // Read pool messages until we get a mining.notify
        if let Some(ref mut s) = sc.stream {
            s.set_read_timeout(Some(Duration::from_secs(1))).ok();
            let mut reader = std::io::BufReader::new(s.try_clone().unwrap());
            let mut got_job = false;
            for _ in 0..60 {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 { continue; }
                let v: serde_json::Value = match serde_json::from_str(line.trim()) { Ok(v) => v, Err(_) => continue };
                if let Some(m) = v.get("method").and_then(|m| m.as_str()) {
                    match m {
                        "mining.set_difficulty" => { if let Some(d) = v.get("params").and_then(|p| p[0].as_f64()) { sc.difficulty = d; } }
                        "mining.notify" => {
                            if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 9 {
                                sc.job_id = p[0].as_str().unwrap_or("").into(); sc.prevhash = p[1].as_str().unwrap_or("").into();
                                sc.coinb1 = p[2].as_str().unwrap_or("").into(); sc.coinb2 = p[3].as_str().unwrap_or("").into();
                                sc.merkle_branches = p[4].as_array().map(|a| a.iter().filter_map(|b| b.as_str().map(String::from)).collect()).unwrap_or_default();
                                sc.version = p[5].as_str().unwrap_or("").into(); sc.nbits = p[6].as_str().unwrap_or("").into();
                                sc.ntime = p[7].as_str().unwrap_or("").into(); sc.clean_jobs = p[8].as_bool().unwrap_or(false);
                                got_job = true; ts.lock().unwrap().add_log(format!("JOB: {} bits:{}", &sc.job_id[..8.min(sc.job_id.len())], sc.nbits));
                                break;
                            } }
                        }
                        _ => {}
                    }
                }
            }
            s.set_read_timeout(Some(Duration::from_millis(100))).ok(); // Non-blocking for mining
            if !got_job { ts.lock().unwrap().add_log("No job - reconnecting".into()); STATE.lock().unwrap().pool_connected = false; continue; }
        }
        ts.lock().unwrap().add_log("Mining...".into());
        
        let mut nonce: u64 = rand::thread_rng().gen::<u64>();
        loop {
            // Non-blocking check for new job (every batch)
            if let Some(ref mut s) = sc.stream {
                let mut buf = [0u8; 1];
                if s.peek(&mut buf).is_ok() {
                    let mut reader = std::io::BufReader::new(s.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) > 0 {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                            if let Some(m) = v.get("method").and_then(|m| m.as_str()) {
                                match m {
                                    "mining.notify" => {
                                        if let Some(p) = v.get("params").and_then(|p| p.as_array()) { if p.len() >= 9 {
                                            sc.job_id = p[0].as_str().unwrap_or("").into(); sc.prevhash = p[1].as_str().unwrap_or("").into();
                                            sc.coinb1 = p[2].as_str().unwrap_or("").into(); sc.coinb2 = p[3].as_str().unwrap_or("").into();
                                            sc.merkle_branches = p[4].as_array().map(|a| a.iter().filter_map(|b| b.as_str().map(String::from)).collect()).unwrap_or_default();
                                            sc.version = p[5].as_str().unwrap_or("").into(); sc.nbits = p[6].as_str().unwrap_or("").into();
                                            sc.ntime = p[7].as_str().unwrap_or("").into(); sc.clean_jobs = p[8].as_bool().unwrap_or(false);
                                            if sc.clean_jobs { nonce = rand::thread_rng().gen::<u64>(); }
                                            ts.lock().unwrap().add_log(format!("New job: {} clean:{}", &sc.job_id[..8.min(sc.job_id.len())], sc.clean_jobs));
                                        }}
                                    }
                                    "mining.set_difficulty" => { if let Some(d) = v.get("params").and_then(|p| p[0].as_f64()) { sc.difficulty = d; } }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            } else { break; } // Stream gone

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
                        ts.lock().unwrap().add_log(format!("SHARE! N:{}", nh));
                        STATE.lock().unwrap().shares_accepted += 1;
                    }
                }
            }
            nonce = nonce.wrapping_add(500);
            STATE.lock().unwrap().shares_total += 500;
            STATE.lock().unwrap().hashrate = 500.0;
            if !ts.lock().unwrap().running { break; }
        }
        STATE.lock().unwrap().pool_connected = false;
        if !ts.lock().unwrap().running { break; }
        ts.lock().unwrap().add_log("Reconnecting...".into());
        thread::sleep(Duration::from_secs(3));
    }
}
