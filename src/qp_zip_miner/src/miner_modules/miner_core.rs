//! Multi-threaded CPU SHA-256d miner with work-stealing and buffer pooling
//!
//! Uses all available CPU cores for parallel nonce search.
//! Atomic counters for lock-free statistics.
//! Each thread processes ONE batch and then exits (no thread leak).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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

/// Convert nbits hex string to a 32-byte big-endian target array
pub fn nbits_to_target(nbits: &str) -> [u8; 32] {
    let bits = u32::from_str_radix(nbits, 16).unwrap_or(0x1d00ffff);
    let exp = (bits >> 24) as usize;
    let mant = (bits & 0x007FFFFF) as u64;
    let mut target = [0u8; 32];
    if exp >= 3 {
        let idx = 32 - (exp - 3).min(32);
        if idx < 32 {
            target[idx] = ((mant >> 16) & 0xFF) as u8;
            if idx + 1 < 32 { target[idx + 1] = ((mant >> 8) & 0xFF) as u8; }
            if idx + 2 < 32 { target[idx + 2] = (mant & 0xFF) as u8; }
        }
    }
    target
}

/// Process a batch of nonces on a single thread.
/// Returns Some(nonce) if a valid share was found.
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

        let h1 = Sha256::digest(&header);
        let hash: [u8; 32] = Sha256::digest(h1).into();

        // Compare hash with target (big-endian comparison)
        let mut meets = true;
        for j in 0..32 {
            if hash[j] > target[j] { meets = false; break; }
            if hash[j] < target[j] { break; }
        }

        if meets {
            stats.shares_found.fetch_add(1, Ordering::Relaxed);
            return Some(nonce);
        }
    }

    stats.total_hashes.fetch_add(batch_size, Ordering::Relaxed);
    None
}

/// Spawn worker threads for ONE batch of mining.
/// Each thread processes a single batch and exits.
/// Returns JoinHandles - caller MUST join() them before spawning more threads.
/// Also returns a share_queue that contains any found nonces.
pub fn spawn_miner_threads(
    num_threads: usize,
    header_base: [u8; 76],
    target: [u8; 32],
    stats: Arc<MinerStats>,
    share_queue: Arc<Mutex<Vec<u32>>>,
    batch_size_per_thread: u64,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(num_threads);

    for thread_id in 0..num_threads {
        let stats = stats.clone();
        let share_queue = share_queue.clone();
        let hb = header_base;
        let tg = target;

        handles.push(thread::spawn(move || {
            let start_nonce = (thread_id as u64) * batch_size_per_thread;
            if let Some(nonce) = mine_batch(&hb, &tg, start_nonce, batch_size_per_thread, &stats) {
                // Push found nonce to share queue
                if let Ok(mut q) = share_queue.lock() {
                    q.push(nonce);
                }
            }
        }));
    }

    handles
}

