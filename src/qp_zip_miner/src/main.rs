use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;

mod miner_modules;
use miner_modules::config::MinerConfig;
use miner_modules::tui::{TuiState, run_tui};
use miner_modules::vulkan::VulkanEngine;
use miner_modules::stratum::*;
lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<MinerState>> = Arc::new(Mutex::new(MinerState::default()));
}

struct MinerState {
    is_mining: bool,
    hashrate: f64,
    shares_accepted: u64,
    shares_rejected: u64,
    shares_total: u64,
    current_job: String,
    pool_connected: bool,
}
impl Default for MinerState {
    fn default() -> Self {
        Self { is_mining: true, hashrate: 0.0, shares_accepted: 0, shares_rejected: 0, shares_total: 0, current_job: String::new(), pool_connected: false }
    }
}

fn load_config() -> MinerConfig { MinerConfig::load("miner_config.toml") }

fn main() {
    println!("============================================");
    println!("    HCSminer v2.0 - Pool Mining");
    println!("    PPLNS @ public-pool.io");
    println!("============================================");
    let config = load_config();
    println!("BTC: {}", config.btc_address);
    println!("Pool: {}:{}", config.pool_host, config.pool_port);
    let vk = VulkanEngine::new(config.vulkan_device_index);
    if vk.available { println!("GPU: {} (VRAM: {:.0}MB)", vk.device_name, vk.vram_mb); }
    let ts = Arc::new(Mutex::new(TuiState::new()));
    let tt = ts.clone(); thread::spawn(move || { let _ = run_tui(tt); });
    let ts2 = ts.clone();
    let btc = config.btc_address.clone(); let wrk = config.worker_name.clone();
    let host = config.pool_host.clone(); let port = config.pool_port;
    thread::spawn(move || { pool_miner_loop(ts2, &btc, &wrk, &host, port); });
    println!("Mining started. Press q in TUI to quit.");
    loop {
        thread::sleep(Duration::from_secs(1));
        if !ts.lock().unwrap().running { break; }
        let s = STATE.lock().unwrap();
        let mut t = ts.lock().unwrap();
        t.cpu_hashrate = s.hashrate;
        t.pool_connected = s.pool_connected;
        t.shares_accepted = s.shares_accepted;
        t.shares_rejected = s.shares_rejected;
        t.total_hashes = s.shares_total;
        t.vram_used_mb = if vk.available { vk.vram_mb * 0.3 } else { 0.0 };
        t.ram_used_mb = 512.0;
    }
    println!("Shutdown.");
}

fn pool_miner_loop(ts: Arc<Mutex<TuiState>>, btc: &str, wrk: &str, host: &str, port: u16) {
    let mut sc = StratumClient::new(btc, wrk);
    loop {
        ts.lock().unwrap().add_log(format!("Connecting to {}:{}...", host, port));
        if let Err(e) = sc.connect(host, port) {
            ts.lock().unwrap().add_log(format!("Failed: {} - retry 10s", e));
            thread::sleep(Duration::from_secs(10)); continue;
        }
        STATE.lock().unwrap().pool_connected = true;
        ts.lock().unwrap().add_log("Connected".to_string());
        if let Err(e) = sc.subscribe() { ts.lock().unwrap().add_log(format!("Sub err: {}", e)); thread::sleep(Duration::from_secs(5)); continue; }
        ts.lock().unwrap().add_log(format!("Subscribed. EN1: {}", sc.extranonce1));
        if let Err(e) = sc.authorize() { ts.lock().unwrap().add_log(format!("Auth err: {}", e)); thread::sleep(Duration::from_secs(5)); continue; }
        ts.lock().unwrap().add_log("Authorized - mining...".to_string());
        let mut nonce: u64 = rand::thread_rng().gen::<u64>();
        loop {
            if let Ok(has_job) = sc.wait_for_notify() {
                if has_job {
                    ts.lock().unwrap().add_log(format!("Job: {} clean:{}", sc.job_id, sc.clean_jobs));
                    STATE.lock().unwrap().current_job = sc.job_id.clone();
                    if sc.clean_jobs { nonce = rand::thread_rng().gen::<u64>(); }
                }
            }
            if sc.job_id.is_empty() { thread::sleep(Duration::from_millis(100)); continue; }
            let e2 = format!("{:08x}", (nonce & 0xFFFFFFFF) as u32);
            let cb = build_coinbase(&sc.coinb1, &sc.extranonce1, &e2, &sc.coinb2);
            let cb_hash = double_sha256(&cb);
            let mr = build_merkle_root(&cb_hash, &sc.merkle_branches);
            for i in 0..1000 {
                let n = nonce.wrapping_add(i) as u32;
                let header = build_header(&sc.version, &sc.prevhash, &mr, &sc.ntime, &sc.nbits, n);
                let hash = double_sha256(&header);
                if hash_meets_target(&hash, &sc.nbits) {
                    let nh = format!("{:08x}", n);
                    let jid = sc.job_id.clone(); let ntm = sc.ntime.clone(); if sc.submit(&jid, &e2, &ntm, &nh).is_ok() {
                        ts.lock().unwrap().add_log(format!("Share accepted! N:{}", nh));
                        STATE.lock().unwrap().shares_accepted += 1;
                    }
                }
            }
            nonce = nonce.wrapping_add(1000);
            STATE.lock().unwrap().shares_total += 1000;
            if !ts.lock().unwrap().running { break; }
        }
        STATE.lock().unwrap().pool_connected = false;
        ts.lock().unwrap().add_log("Disconnected. Reconnecting...".to_string());
        if !ts.lock().unwrap().running { break; }
        thread::sleep(Duration::from_secs(5));
    }
}

