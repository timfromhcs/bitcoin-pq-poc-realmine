use sha2::{Sha256, Digest};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub struct StratumClient {
    pub stream: Option<TcpStream>,
    pub connected: bool,
    pub extranonce1: String,
    pub extranonce2_size: usize,
    pub difficulty: f64,
    pub job_id: String,
    pub prevhash: String,
    pub coinb1: String,
    pub coinb2: String,
    pub merkle_branches: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
    pub btc_address: String,
    pub worker_name: String,
}

impl StratumClient {
    pub fn check_notify_nonblock(&mut self) -> Result<bool, String> {
        if let Some(ref mut s) = self.stream {
            let mut buf = [0u8; 1];
            s.set_read_timeout(Some(std::time::Duration::from_millis(1))).ok();
            match s.peek(&mut buf) {
                Ok(_) => {
                    s.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
                    self.wait_for_notify()
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    s.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
                    Ok(false)
                }
                Err(e) => {
                    s.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
                    Err(format!("NB: {}", e))
                }
            }
        } else { Err("No stream".to_string()) }
    }

    /// Wait for and parse a mining.notify message (non-blocking peek + read)
    /// Returns Ok(true) if a new job was received, Ok(false) if no data available
    pub fn wait_for_notify(&mut self) -> Result<bool, String> {
        loop {
            let line = self.recv()?;
            if line.is_empty() { return Ok(false); }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                match v.get("method").and_then(|m| m.as_str()) {
                    Some("mining.notify") => {
                        if let Some(p) = v.get("params").and_then(|p| p.as_array()) {
                            if p.len() >= 9 {
                                self.job_id = p[0].as_str().unwrap_or("").into();
                                self.prevhash = p[1].as_str().unwrap_or("").into();
                                self.coinb1 = p[2].as_str().unwrap_or("").into();
                                self.coinb2 = p[3].as_str().unwrap_or("").into();
                                self.merkle_branches = p[4].as_array()
                                    .map(|a| a.iter().filter_map(|b| b.as_str().map(String::from)).collect())
                                    .unwrap_or_default();
                                self.version = p[5].as_str().unwrap_or("").into();
                                self.nbits = p[6].as_str().unwrap_or("").into();
                                self.ntime = p[7].as_str().unwrap_or("").into();
                                self.clean_jobs = p[8].as_bool().unwrap_or(false);
                                return Ok(true);
                            }
                        }
                    }
                    Some("mining.set_difficulty") => {
                        if let Some(d) = v.get("params").and_then(|p| p[0].as_f64()) {
                            self.difficulty = d;
                        }
                    }
                    Some("mining.set_extranonce") => {
                        if let Some(p) = v.get("params").and_then(|p| p.as_array()) {
                            if p.len() >= 1 {
                                self.extranonce1 = p[0].as_str().unwrap_or("").into();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn new(btc: &str, wrk: &str) -> Self {
        StratumClient {
            stream: None, connected: false, extranonce1: String::new(), extranonce2_size: 0,
            difficulty: 1.0, job_id: String::new(), prevhash: String::new(), coinb1: String::new(),
            coinb2: String::new(), merkle_branches: Vec::new(), version: String::new(),
            nbits: String::new(), ntime: String::new(), clean_jobs: false,
            btc_address: btc.to_string(), worker_name: wrk.to_string(),
        }
    }
    pub fn connect(&mut self, host: &str, port: u16) -> Result<(), String> {
        let stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| format!("TCP: {}", e))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
        self.stream = Some(stream);
        self.connected = true;
        Ok(())
    }
    pub fn subscribe(&mut self) -> Result<(), String> {
        let msg = serde_json::json!({"id":1,"method":"mining.subscribe","params":["HCSminer/2.0"]});
        self.send(&serde_json::to_string(&msg).map_err(|e| format!("JS: {}", e))?)?;
        let r = self.recv()?;
        let v: serde_json::Value = serde_json::from_str(&r).map_err(|e| format!("JP: {}", e))?;
        if let Some(arr) = v.get("result").and_then(|r| r.as_array()) {
            if arr.len() >= 3 {
                self.extranonce1 = arr[1].as_str().unwrap_or("").to_string();
                self.extranonce2_size = arr[2].as_i64().unwrap_or(4) as usize;
            }
        }
        Ok(())
    }
    pub fn authorize(&mut self) -> Result<(), String> {
        let user = format!("{}.{}", self.btc_address, self.worker_name);
        let msg = serde_json::json!({"id":2,"method":"mining.authorize","params":[user,"x"]});
        self.send(&serde_json::to_string(&msg).map_err(|e| format!("JS: {}", e))?)?;
        let r = self.recv()?;
        let v: serde_json::Value = serde_json::from_str(&r).map_err(|e| format!("JP: {}", e))?;
        if v.get("result").and_then(|r| r.as_bool()).unwrap_or(false) { return Ok(()); }
        Err("Auth rejected".to_string())
    }
    pub fn submit(&mut self, job: &str, e2: &str, tm: &str, nonce: &str) -> Result<(), String> {
        let user = format!("{}.{}", self.btc_address, self.worker_name);
        let msg = serde_json::json!({"id":3,"method":"mining.submit","params":[user,job,e2,tm,nonce]});
        self.send(&serde_json::to_string(&msg).map_err(|e| format!("JS: {}", e))?)
    }
    fn send(&mut self, data: &str) -> Result<(), String> {
        if let Some(ref mut s) = self.stream {
            s.write_all(format!("{}
", data).as_bytes()).map_err(|e| format!("W: {}", e))
        } else { Err("Not connected".to_string()) }
    }
    fn recv(&mut self) -> Result<String, String> {
        if let Some(ref mut s) = self.stream {
            let mut r = BufReader::new(s.try_clone().map_err(|e| format!("C: {}", e))?);
            let mut l = String::new();
            r.read_line(&mut l).map_err(|e| format!("R: {}", e))?;
            Ok(l.trim().to_string())
        } else { Err("Not connected".to_string()) }
    }
}

pub fn swap_endian(hex: &str) -> String {
    hex::decode(hex).unwrap_or_default().iter().rev().map(|b| format!("{:02x}", b)).collect()
}

pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    let h1 = Sha256::digest(data);
    Sha256::digest(h1).into()
}

pub fn build_coinbase(cb1: &str, ex1: &str, ex2: &str, cb2: &str) -> Vec<u8> {
    let mut c = hex::decode(cb1).unwrap_or_default();
    c.extend_from_slice(&hex::decode(ex1).unwrap_or_default());
    c.extend_from_slice(&hex::decode(ex2).unwrap_or_default());
    c.extend_from_slice(&hex::decode(cb2).unwrap_or_default());
    c
}

pub fn build_merkle_root(ch: &[u8; 32], branches: &[String]) -> [u8; 32] {
    let mut root = *ch;
    for b in branches {
        let bb = hex::decode(b).unwrap_or_default();
        if bb.len() != 32 { continue; }
        let mut combined = [0u8; 64];
        // Always put root first (little-endian for stratum) - actual order depends on hash comparison but this works for most pools
        combined[..32].copy_from_slice(&root);
        combined[32..].copy_from_slice(&bb);
        root = double_sha256(&combined);
    }
    root
}

pub fn build_header(version: &str, prevhash: &str, mr: &[u8; 32], ntime: &str, nbits: &str, nonce: u32) -> [u8; 80] {
    let mut h = [0u8; 80];
    let ver = u32::from_str_radix(version, 16).unwrap_or(0);
    h[..4].copy_from_slice(&ver.to_le_bytes());
    let prev = swap_endian(prevhash);
    let pb = hex::decode(&prev).unwrap_or_default();
    if pb.len() >= 32 { h[4..36].copy_from_slice(&pb[..32]); }
    let mr_rev: Vec<u8> = mr.iter().rev().cloned().collect();
    h[36..68].copy_from_slice(&mr_rev);
    let tm = u32::from_str_radix(ntime, 16).unwrap_or(0);
    h[68..72].copy_from_slice(&tm.to_le_bytes());
    let bits = u32::from_str_radix(nbits, 16).unwrap_or(0);
    h[72..76].copy_from_slice(&bits.to_le_bytes());
    h[76..80].copy_from_slice(&nonce.to_le_bytes());
    h
}

pub fn hash_meets_target(hash: &[u8; 32], nbits: &str) -> bool {
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
    for i in 0..32 {
        if hash[i] > target[i] { return false; }
        if hash[i] < target[i] { return true; }
    }
    true
}
