#![allow(dead_code)]
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;

mod miner_modules;
use miner_modules::config::MinerConfig;
use miner_modules::tui::{TuiState, run_tui};
use miner_modules::vulkan::VulkanEngine;
use miner_modules::stratum::*;
use miner_modules::miner_core::{MinerStats, spawn_miner_threads, nbits_to_target};

lazy_static::lazy_static! {
    static ref HASHES_TOTAL: AtomicU64 = AtomicU64::new(0);
    static ref SHARES_ACCEPTED: AtomicU64 = AtomicU64::new(0);
    static ref SHARES_REJECTED: AtomicU64 = AtomicU64::new(0);
    static ref POOL_CONNECTED: AtomicBool = AtomicBool::new(false);
}

fn load_config() -> MinerConfig { MinerConfig::load("miner_config.toml") }

fn main() {
    println!("============================================");
    println!("    HCSminer v3.0 - Post-Quantum Pool Miner");
    println!("============================================");
    let cfg = load_config();
    println!("BTC: {}", cfg.btc_address);
    println!("Threads: {}", cfg.threads);
    println!("Pool: {}:{}", cfg.pool_host, cfg.pool_port);

    let vk = VulkanEngine::new(cfg.vulkan_device_index);
    if vk.available {
        println!("GPU: {} (VRAM: {:.0}MB)", vk.device_name, vk.vram_mb);
    } else {
        println!("GPU: CPU-only mode (Vulkan unavailable)");
    }

    let ts = Arc::new(Mutex::new(TuiState::new()));
    let running = Arc::new(AtomicBool::new(true));

    if cfg.enable_tui {
        let tt = ts.clone();
        let r2 = running.clone();
        thread::spawn(move || { let _ = run_tui(tt, r2); });
    }

    let ts2 = ts.clone();
    let r3 = running.clone();
    let vk_avail = vk.available;
    let vk_vram = vk.vram_mb;
    thread::spawn(move || {
        let mut last_update = Instant::now();
        let mut last_hashes = 0u64;
        while r3.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
            let h = HASHES_TOTAL.load(Ordering::Relaxed);
            let dt = last_update.elapsed().as_secs_f64().max(0.001);
            let hashrate = (h - last_hashes) as f64 / dt;
            last_hashes = h;
            last_update = Instant::now();
            if let Ok(mut t) = ts2.lock() {
                t.cpu_hashrate = hashrate;
                t.pool_connected = POOL_CONNECTED.load(Ordering::Relaxed);
                t.shares_accepted = SHARES_ACCEPTED.load(Ordering::Relaxed);
                t.shares_rejected = SHARES_REJECTED.load(Ordering::Relaxed);
                t.total_hashes = h;
                t.vram_used_mb = if vk_avail { (vk_vram * 0.3).min(16000.0) } else { 0.0 };
                t.ram_used_mb = 512.0;
            }
        }
    });

    let btc = cfg.btc_address.clone();
    let wrk = cfg.worker_name.clone();
    let host = cfg.pool_host.clone();
    let port = cfg.pool_port;
    let threads = cfg.threads;
    let r4 = running.clone();

    thread::spawn(move || {
        pool_miner_loop(ts.clone(), &btc, &wrk, &host, port, threads, r4);
    });

    println!("Mining. Press 'q' in TUI to quit.");
    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(500));
    }
    println!("Shutdown.");
}
fn pool_miner_loop(
    ts: Arc<Mutex<TuiState>>,
    btc: &str,
    wrk: &str,
    host: &str,
    port: u16,
    num_threads: usize,
    running: Arc<AtomicBool>,
) {
    let num_threads = num_threads.max(1);
    let batch_size_per_thread: u64 = 1_000_000 / num_threads as u64;
    let total_in_batch = num_threads as u64 * batch_size_per_thread;

    loop {
        if !running.load(Ordering::Relaxed) { break; }
        ts.lock().ok().map(|mut t| t.add_log("Connecting...".to_string()));

        let mut sc = StratumClient::new(btc, wrk);
        match sc.connect(host, port) {
            Ok(()) => {
                POOL_CONNECTED.store(true, Ordering::Relaxed);
                ts.lock().ok().map(|mut t| t.add_log("Connected!".into()));
            }
            Err(e) => {
                ts.lock().ok().map(|mut t| t.add_log(format!("Failed: {}", e)));
                if !running.load(Ordering::Relaxed) { break; }
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        }

        if sc.subscribe().is_err() || sc.authorize().is_err() {
            ts.lock().ok().map(|mut t| t.add_log("Auth failed".into()));
            POOL_CONNECTED.store(false, Ordering::Relaxed);
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        ts.lock().ok().map(|mut t| t.add_log("Mining!".into()));

        let mut nonce = rand::thread_rng().gen::<u64>();
        let mut last_log = Instant::now();

        loop {
            if !running.load(Ordering::Relaxed) { break; }

            match sc.check_notify_nonblock() {
                Ok(true) => { if sc.clean_jobs { nonce = rand::thread_rng().gen::<u64>(); } }
                Ok(false) => {}
                Err(_) => { ts.lock().ok().map(|mut t| t.add_log("Lost conn".into())); break; }
            }

            if sc.job_id.is_empty() { thread::sleep(Duration::from_millis(10)); continue; }

            let e2 = format!("{:08x}", (nonce & 0xFFFFFFFF) as u32);
            let cb = build_coinbase(&sc.coinb1, &sc.extranonce1, &e2, &sc.coinb2);
            let cb_hash = double_sha256(&cb);
            let mr = build_merkle_root(&cb_hash, &sc.merkle_branches);

            let ver = u32::from_str_radix(&sc.version, 16).unwrap_or(0);
            let prev = swap_endian(&sc.prevhash);
            let pb = hex::decode(&prev).unwrap_or_default();
            let mr_rev: Vec<u8> = mr.iter().rev().cloned().collect();
            let tm = u32::from_str_radix(&sc.ntime, 16).unwrap_or(0);
            let bits = u32::from_str_radix(&sc.nbits, 16).unwrap_or(0);

            let mut header_base = [0u8; 76];
            header_base[..4].copy_from_slice(&ver.to_le_bytes());
            if pb.len() >= 32 { header_base[4..36].copy_from_slice(&pb[..32]); }
            header_base[36..68].copy_from_slice(&mr_rev);
            header_base[68..72].copy_from_slice(&tm.to_le_bytes());
            header_base[72..76].copy_from_slice(&bits.to_le_bytes());

            let target = nbits_to_target(&sc.nbits);
            let share_queue: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
            let stats = Arc::new(MinerStats::new());

            let handles = spawn_miner_threads(
                num_threads, header_base, target,
                stats.clone(), share_queue.clone(), batch_size_per_thread,
            );

            for handle in handles { let _ = handle.join(); }

            let mut found_nonces: Vec<u32> = Vec::new();
            if let Ok(mut q) = share_queue.lock() { std::mem::swap(&mut found_nonces, &mut q); }

            for nf in &found_nonces {
                let nh = format!("{:08x}", nf);
                let jid = sc.job_id.clone();
                let ntm = sc.ntime.clone();
                let e2s = format!("{:08x}", (nonce & 0xFFFFFFFF) as u32);
                if sc.submit(&jid, &e2s, &ntm, &nh).is_ok() {
                    ts.lock().ok().map(|mut t| t.add_log(format!("Share! {}", nh)));
                    SHARES_ACCEPTED.fetch_add(1, Ordering::Relaxed);
                } else {
                    SHARES_REJECTED.fetch_add(1, Ordering::Relaxed);
                }
            }

            let bh = stats.total_hashes.load(Ordering::Relaxed);
            HASHES_TOTAL.fetch_add(bh.max(total_in_batch), Ordering::Relaxed);

            if last_log.elapsed() >= Duration::from_secs(30) {
                let th = HASHES_TOTAL.load(Ordering::Relaxed);
                let sa = SHARES_ACCEPTED.load(Ordering::Relaxed);
                let sr = SHARES_REJECTED.load(Ordering::Relaxed);
                ts.lock().ok().map(|mut t| t.add_log(format!(
                    "Hashrate: ~{} H/s | Shares: {}/{}",
                    th as f64 / 30.0, sa, sr
                )));
                last_log = Instant::now();
            }

            nonce = nonce.wrapping_add(total_in_batch);
            if !running.load(Ordering::Relaxed) { break; }
        }

        POOL_CONNECTED.store(false, Ordering::Relaxed);
        if !running.load(Ordering::Relaxed) { break; }
        ts.lock().ok().map(|mut t| t.add_log("Reconnecting...".into()));
        thread::sleep(Duration::from_secs(3));
    }
}
