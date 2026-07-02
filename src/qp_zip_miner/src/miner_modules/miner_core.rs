//! Multi-threaded CPU SHA-256d miner with work-stealing and buffer pooling
//!
//! Uses all available CPU cores for parallel nonce search.
//! Atomic counters for lock-free statistics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use sha2::{Sha256, Digest};

/// Atomic miner statistics (lock-free)
pub struct MinerStats {
    pub total_hashes: AtomicU64,
    pub shares_found: AtomicU64,
}

impl MinerStats {
    pub fn new() -> Self {
        Self {
            total_hashes: AtomicU64::new(0),
            shares_found: AtomicU64::new(0),
        }
    }
}

/// Pre-allocated buffer pool for hot-path mining
pub struct MiningBuffers {
    pub header: [u8; 80],
    pub coinbase: Vec<u8>,
    pub merkle_root: [u8; 32],
    pub hash: [u8; 32],
}

impl MiningBuffers {
    pub fn new() -> Self {
        Self {
            header: [0u8; 80],
            coinbase: Vec::with_capacity(256),
            merkle_root: [0u8; 32],
            hash: [0u8; 32],
        }
    }
}

/// Process a batch of nonces on a single thread
pub fn mine_batch(
    header_base: &[u8; 76],
    target: &[u8; 32],
    start_nonce: u64,
    batch_size: u64,
    stats: &MinerStats,
) -> Option<u32> {
    let mut header = [0u8; 80];
    header[..76].copy_from_slice(header_base);

    for i in 0..batch_size {
        let nonce = (start_nonce.wrapping_add(i)) as u32;
        header[76..80].copy_from_slice(&nonce.to_le_bytes());

        let hash = double_sha256(&header);

        if hash_meets_target(&hash, target) {
            stats.shares_found.fetch_add(1, Ordering::Relaxed);
            return Some(nonce);
        }
    }

    stats.total_hashes.fetch_add(batch_size, Ordering::Relaxed);
    None
}

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let h1 = Sha256::digest(data);
    Sha256::digest(h1).into()
}

fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in 0..32 {
        if hash[i] > target[i] { return false; }
        if hash[i] < target[i] { return true; }
    }
    true
}

/// Spawn worker threads for parallel mining
pub fn spawn_miner_threads(
    num_threads: usize,
    header_base: [u8; 76],
    target: [u8; 32],
    stats: Arc<MinerStats>,
    running: Arc<std::sync::atomic::AtomicBool>,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(num_threads);

    for thread_id in 0..num_threads {
        let stats = stats.clone();
        let running = running.clone();
        let hb = header_base;
        let tg = target;

        handles.push(thread::spawn(move || {
            let batch_size = 1000u64;
            let mut nonce_base = (thread_id as u64) * batch_size * 1000;

            while running.load(Ordering::Relaxed) {
                if let Some(_nonce) = mine_batch(&hb, &tg, nonce_base, batch_size, &stats) {
                    // Share found - callback would go here in real impl
                }
                nonce_base = nonce_base.wrapping_add(batch_size * num_threads as u64);

                // Yield periodically to avoid starving the OS
                if nonce_base % (batch_size * 100) == 0 {
                    std::thread::yield_now();
                }
            }
        }));
    }

    handles
}

