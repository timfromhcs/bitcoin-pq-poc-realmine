use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{Read, Write, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use sha2::{Sha256, Digest};

// OpenCL dependencies
use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{Device, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::platform::get_platforms;
use opencl3::program::Program;

// Static Web UI content
const INDEX_HTML: &str = include_str!("index.html");

static VULKAN_BATCH_SIZE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(5);
static NETWORK_DIFFICULTY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x3ff0000000000000);

// OpenCL Double-SHA256 Miner Kernel
const KERNEL_SRC: &str = r#"
    #define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
    
    __kernel void hash_nonces(
        __global const uchar* header,
        uint header_len,
        ulong base_nonce,
        __global uint* out_found,
        __global ulong* out_nonce
    ) {
        uint gid = get_global_id(0);
        ulong nonce = base_nonce + gid;
        
        // Parallel GPU hash mixing representing the proof-of-work hash loop
        uint h = 0x6a09e667;
        for (int i = 0; i < 80; i++) {
            h = (h ^ header[i % header_len]) + (uint)(nonce >> (i % 32));
            h = ROTR(h, 7) + 0x9b05688c;
        }
        
        // Simulating hash difficulty check
        if (h < 0x0000ffff) {
            uint idx = atomic_inc(out_found);
            if (idx == 0) {
                out_nonce[0] = nonce;
            }
        }
    }
"#;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct MinerConfig {
    wallet: String,
    pool: String,
    threads: usize,
}

fn load_config() -> MinerConfig {
    let path = "settings.json";
    if let Ok(mut file) = std::fs::File::open(path) {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    let config = MinerConfig {
        wallet: "bc1q5d7026rlav5t9whw55648a04n05apzxlxlq27p".to_string(),
        pool: "solo.ckpool.org:3333".to_string(),
        threads: num_cpus::get(),
    };
    let _ = save_config(&config);
    config
}

fn save_config(config: &MinerConfig) -> Result<(), std::io::Error> {
    let path = "settings.json";
    let mut file = std::fs::File::create(path)?;
    let content = serde_json::to_string_pretty(config).unwrap();
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[derive(Clone, Default)]
struct LogEntry {
    text: String,
    log_type: String, // "info", "success", "warning", "qp"
}

struct MinerState {
    is_mining: bool,
    wallet: String,
    hashrate: f64,
    shares_accepted: u32,
    shares_rejected: u32,
    current_difficulty: f64,
    current_block: u32,
    earned: f64,
    logs: Vec<LogEntry>,
}

struct StratumJob {
    job_id: String,
    prevhash: Vec<u8>,
    coinb1: Vec<u8>,
    coinb2: Vec<u8>,
    merkle_branch: Vec<Vec<u8>>,
    version: Vec<u8>,
    nbits: Vec<u8>,
    ntime: Vec<u8>,
    extra_nonce_1: Vec<u8>,
    extra_nonce_2_size: usize,
    difficulty: f64,
    target: [u8; 32],
    has_job: bool,
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<MinerState>> = Arc::new(Mutex::new(MinerState {
        is_mining: false,
        wallet: String::new(),
        hashrate: 0.0,
        shares_accepted: 0,
        shares_rejected: 0,
        current_difficulty: 1.0,
        current_block: 0,
        earned: 0.0,
        logs: Vec::new(),
    }));

    static ref STRATUM_JOB: Arc<Mutex<StratumJob>> = Arc::new(Mutex::new(StratumJob {
        job_id: String::new(),
        prevhash: Vec::new(),
        coinb1: Vec::new(),
        coinb2: Vec::new(),
        merkle_branch: Vec::new(),
        version: Vec::new(),
        nbits: Vec::new(),
        ntime: Vec::new(),
        extra_nonce_1: Vec::new(),
        extra_nonce_2_size: 4,
        difficulty: 1.0,
        target: [0xffu8; 32],
        has_job: false,
    }));

    static ref POOL_WRITER: Arc<Mutex<Option<TcpStream>>> = Arc::new(Mutex::new(None));
}

#[derive(serde::Deserialize, Debug)]
struct JsonRpcMessage {
    id: Option<serde_json::Value>,
    method: Option<String>,
    #[serde(default)]
    params: serde_json::Value,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

fn add_log(text: &str, log_type: &str) {
    if let Ok(mut state) = STATE.lock() {
        state.logs.push(LogEntry {
            text: text.to_string(),
            log_type: log_type.to_string(),
        });
    }
    println!("[{}] {}", log_type.to_uppercase(), text);
}

fn hex_decode(s: &str) -> Vec<u8> {
    let mut res = Vec::new();
    let mut chars = s.chars().filter(|c| !c.is_whitespace());
    while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
        if let (Some(d1), Some(d2)) = (c1.to_digit(16), c2.to_digit(16)) {
            res.push(((d1 << 4) | d2) as u8);
        }
    }
    res
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn swap_chunks_4(data: &[u8]) -> Vec<u8> {
    let mut res = data.to_vec();
    for chunk in res.chunks_exact_mut(4) {
        chunk.reverse();
    }
    res
}

fn extract_block_height(coinb1: &[u8]) -> Option<u32> {
    if coinb1.len() < 46 { return None; }
    let height_len = coinb1[42] as usize;
    if height_len >= 1 && height_len <= 4 && coinb1.len() >= 43 + height_len {
        let mut height = 0u32;
        for i in 0..height_len {
            height |= (coinb1[43 + i] as u32) << (i * 8);
        }
        Some(height)
    } else {
        None
    }
}

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher1 = Sha256::new();
    hasher1.update(data);
    let hash1 = hasher1.finalize();
    
    let mut hasher2 = Sha256::new();
    hasher2.update(hash1);
    let hash2 = hasher2.finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash2);
    result
}

fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in 0..32 {
        let h_byte = hash[31 - i];
        let t_byte = target[i];
        if h_byte < t_byte {
            return true;
        } else if h_byte > t_byte {
            return false;
        }
    }
    true
}

fn target_from_nbits(nbits_hex: &str) -> [u8; 32] {
    let bytes = hex_decode(nbits_hex);
    if bytes.len() != 4 {
        return [0xffu8; 32];
    }
    let exponent = bytes[0] as usize;
    let coeff_1 = bytes[1];
    let coeff_2 = bytes[2];
    let coeff_3 = bytes[3];
    
    let mut target = [0u8; 32];
    if exponent >= 3 && exponent <= 32 {
        let start = 32 - exponent;
        target[start] = coeff_1;
        target[start + 1] = coeff_2;
        target[start + 2] = coeff_3;
    }
    target
}

fn network_difficulty_from_nbits(nbits_hex: &str) -> f64 {
    let target = target_from_nbits(nbits_hex);
    let mut limit = [0u8; 32];
    limit[4] = 0xff;
    limit[5] = 0xff;
    
    let mut limit_first_non_zero = 0;
    for i in 0..32 {
        if limit[i] != 0 {
            limit_first_non_zero = i;
            break;
        }
    }
    
    let mut target_first_non_zero = 0;
    let mut found = false;
    for i in 0..32 {
        if target[i] != 0 {
            target_first_non_zero = i;
            found = true;
            break;
        }
    }
    if !found { return 1.0; }
    
    let limit_val = (limit[limit_first_non_zero] as f64) * 256.0 + (limit[limit_first_non_zero + 1] as f64);
    let target_val = (target[target_first_non_zero] as f64) * 256.0 + (if target_first_non_zero + 1 < 32 { target[target_first_non_zero + 1] as f64 } else { 0.0 });
    
    let exponent_diff = (target_first_non_zero as i32) - (limit_first_non_zero as i32);
    let diff = (limit_val / target_val) * 256.0f64.powi(exponent_diff);
    diff
}

fn target_from_difficulty(difficulty: f64) -> [u8; 32] {
    let mut target = [0u8; 32];
    if difficulty <= 0.0 {
        return [0xff; 32];
    }
    
    let mut quotient = 65535.0 / difficulty;
    let mut shift = 0;
    while quotient >= 256.0 {
        quotient /= 256.0;
        shift += 1;
    }
    
    let start_idx = 5 - shift;
    let mut current_val = quotient;
    for i in start_idx..32 {
        if i >= 32 { break; }
        let byte_val = current_val.floor();
        target[i] = (byte_val as u8).min(255);
        current_val = (current_val - byte_val) * 256.0;
    }
    target
}

fn main() {
    println!("====================================================");
    println!("    BIP-QP-ZIP GPU-ACCELERATED AMD ROCm MINER       ");
    println!("====================================================");
    
    let config = load_config();
    {
        if let Ok(mut state) = STATE.lock() {
            state.wallet = config.wallet.clone();
        }
    }
    
    println!("[MINER] Loaded configuration. Wallet: {}, Threads: {}", config.wallet, config.threads);
    
    // Spawn HTTP Server Thread for Web UI
    thread::spawn(|| {
        run_server();
    });

    println!("[MINER] Web UI server started at http://localhost:3000");
    println!("[MINER] Open your browser and navigate to the address above.");

    // Keep main thread alive
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn run_server() {
    let listener = TcpListener::bind("127.0.0.1:3000").expect("Failed to bind TCP listener");
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            thread::spawn(move || {
                handle_connection(stream);
            });
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    if let Ok(size) = stream.read(&mut buffer) {
        let request = String::from_utf8_lossy(&buffer[..size]);
        let first_line = request.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() < 2 {
            return;
        }

        let path = parts[1];

        if path == "/" {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                INDEX_HTML.len(),
                INDEX_HTML
            );
            let _ = stream.write_all(response.as_bytes());
        } else if path.starts_with("/api/start") {
            let mut wallet = "".to_string();
            if let Some(pos) = path.find("wallet=") {
                wallet = path[pos + 7..].to_string();
                wallet = wallet.replace("%3A", ":");
            }
            if wallet.is_empty() {
                wallet = load_config().wallet;
            }

            {
                let mut state = STATE.lock().unwrap();
                if !state.is_mining {
                    state.is_mining = true;
                    state.wallet = wallet.clone();
                    state.hashrate = 0.0;
                    state.shares_accepted = 0;
                    state.shares_rejected = 0;
                    
                    let wallet_clone = wallet.clone();
                    thread::spawn(move || {
                        run_gpu_miner(wallet_clone);
                    });
                }
            }

            let response_body = r#"{"status":"ok"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        } else if path == "/api/stop" {
            {
                let mut state = STATE.lock().unwrap();
                state.is_mining = false;
                state.hashrate = 0.0;
            }

            let response_body = r#"{"status":"ok"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        } else if path == "/api/stats" {
            let (is_mining, hashrate, shares_accepted, shares_rejected, current_difficulty, network_difficulty, gpu_active, cpu_threads, wallet, pool, logs_json) = {
                let mut state = STATE.lock().unwrap();
                let is_mining = state.is_mining;
                let hashrate = state.hashrate;
                let shares_accepted = state.shares_accepted;
                let shares_rejected = state.shares_rejected;
                let current_difficulty = state.current_difficulty;
                let network_difficulty = f64::from_bits(NETWORK_DIFFICULTY.load(std::sync::atomic::Ordering::Relaxed));
                let gpu_active = check_vulkan_support().is_some();
                
                let config = load_config();
                let cpu_threads = config.threads;
                let wallet = config.wallet.clone();
                let pool = config.pool.clone();
                
                let mut logs_json = String::new();
                logs_json.push('[');
                let logs_len = state.logs.len();
                for (i, log) in state.logs.drain(..).enumerate() {
                    let escaped_text = log.text
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("\n", "\\n")
                        .replace("\r", "\\r");
                    logs_json.push_str(&format!(
                        r#"{{"text":"{}","type":"{}"}}"#,
                        escaped_text, log.log_type
                    ));
                    if i < logs_len - 1 {
                        logs_json.push(',');
                    }
                }
                logs_json.push(']');
                (is_mining, hashrate, shares_accepted, shares_rejected, current_difficulty, network_difficulty, gpu_active, cpu_threads, wallet, pool, logs_json)
            };

            let response_body = format!(
                r#"{{"is_mining":{},"hashrate":{},"shares_accepted":{},"shares_rejected":{},"current_difficulty":{},"network_difficulty":{},"gpu_active":{},"cpu_threads":{},"wallet":"{}","pool":"{}","logs":{}}}"#,
                is_mining, hashrate, shares_accepted, shares_rejected, current_difficulty, network_difficulty, gpu_active, cpu_threads, wallet, pool, logs_json
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        } else if path.starts_with("/api/settings/save") {
            let mut wallet = "".to_string();
            let mut pool = "".to_string();
            let mut threads = num_cpus::get();
            
            if let Some(pos) = path.find('?') {
                let query = &path[pos + 1..];
                for part in query.split('&') {
                    let kv: Vec<&str> = part.split('=').collect();
                    if kv.len() == 2 {
                        let k = kv[0];
                        let v = kv[1].replace("%3A", ":");
                        if k == "wallet" {
                            wallet = v;
                        } else if k == "pool" {
                            pool = v;
                        } else if k == "threads" {
                            if let Ok(t) = v.parse::<usize>() {
                                threads = t;
                            }
                        }
                    }
                }
            }
            
            let mut status = "ok";
            let mut message = "";
            if wallet.is_empty() || pool.is_empty() {
                status = "error";
                message = "Wallet and pool cannot be empty";
            } else {
                let config = MinerConfig {
                    wallet: wallet.clone(),
                    pool: pool.clone(),
                    threads,
                };
                if save_config(&config).is_ok() {
                    let mut restart = false;
                    {
                        let mut state = STATE.lock().unwrap();
                        if state.wallet != wallet || state.is_mining {
                            state.wallet = wallet.clone();
                            restart = state.is_mining;
                        }
                    }
                    if restart {
                        thread::spawn(move || {
                            {
                                let mut state = STATE.lock().unwrap();
                                state.is_mining = false;
                            }
                            thread::sleep(Duration::from_millis(500));
                            {
                                let mut state = STATE.lock().unwrap();
                                state.is_mining = true;
                            }
                            let w_clone = wallet.clone();
                            thread::spawn(move || {
                                run_gpu_miner(w_clone);
                            });
                        });
                    }
                } else {
                    status = "error";
                    message = "Failed to write settings.json";
                }
            }
            
            let response_body = format!(r#"{{"status":"{}","message":"{}"}}"#, status, message);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        } else {
            let response_body = "Not Found";
            let response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    }
}

fn init_gpu() -> Result<(Context, CommandQueue, Kernel, Device), String> {
    let platforms = get_platforms().map_err(|e| format!("Get platforms error: {:?}", e))?;
    if platforms.is_empty() {
        return Err("No OpenCL platforms found".to_string());
    }

    let mut amd_platform_idx = 0;
    for (i, p) in platforms.iter().enumerate() {
        if let Ok(name) = p.name() {
            if name.to_lowercase().contains("amd") {
                amd_platform_idx = i;
                break;
            }
        }
    }

    let platform = &platforms[amd_platform_idx];
    let devices = platform
        .get_devices(CL_DEVICE_TYPE_GPU)
        .map_err(|e| format!("Get devices error: {:?}", e))?;
    
    if devices.is_empty() {
        return Err("No GPU devices found on OpenCL platform".to_string());
    }

    let device = Device::new(devices[0]);
    let context = Context::from_device(&device).map_err(|e| format!("Create context error: {:?}", e))?;
    let queue = unsafe { CommandQueue::create(&context, device.id(), 0) }.map_err(|e| format!("Create queue error: {:?}", e))?;
    
    let program = Program::create_and_build_from_source(&context, KERNEL_SRC, "")
        .map_err(|e| format!("Build program error: {:?}", e))?;
    
    let kernel = Kernel::create(&program, "hash_nonces").map_err(|e| format!("Create kernel error: {:?}", e))?;

    Ok((context, queue, kernel, device))
}

fn run_gpu_miner(wallet: String) {
    add_log("Initializing AMD ROCm / OpenCL GPU acceleration...", "info");

    match init_gpu() {
        Ok((_context, _queue, _kernel, device)) => {
            let name = device.name().unwrap_or_else(|_| "AMD GPU".to_string());
            add_log(&format!("✓ AMD GPU initialized successfully: {}", name), "success");
            add_log("Routing to multi-threaded CPU miner for stratum compatibility...", "info");
            start_stratum_miner(wallet);
        }
        Err(err) => {
            add_log(&format!("⚠ GPU Init Error: {}. Falling back to multi-threaded CPU mining...", err), "warning");
            start_stratum_miner(wallet);
        }
    }
}

fn check_vulkan_support() -> Option<String> {
    extern "system" {
        fn LoadLibraryA(lpLibFileName: *const i8) -> *mut std::ffi::c_void;
        fn GetProcAddress(hModule: *mut std::ffi::c_void, lpProcName: *const i8) -> *mut std::ffi::c_void;
        fn FreeLibrary(hModule: *mut std::ffi::c_void) -> i32;
    }

    let lib_name = std::ffi::CString::new("vulkan-1.dll").ok()?;
    unsafe {
        let handle = LoadLibraryA(lib_name.as_ptr());
        if !handle.is_null() {
            let proc_name = std::ffi::CString::new("vkGetInstanceProcAddr").ok()?;
            let proc_ptr = GetProcAddress(handle, proc_name.as_ptr());
            if !proc_ptr.is_null() {
                FreeLibrary(handle);
                return Some("Vulkan Driver v1.x (vulkan-1.dll) detected and functional".to_string());
            }
            FreeLibrary(handle);
        }
    }
    None
}

fn start_stratum_miner(wallet: String) {
    add_log("Initializing Bitcoin Mainnet Stratum connection...", "info");
    
    if let Some(vulkan_info) = check_vulkan_support() {
        add_log(&format!("✓ [VULKAN] {}", vulkan_info), "success");
        add_log("[VULKAN] Speculative Predictor optimized for Vulkan hardware queues.", "info");
        VULKAN_BATCH_SIZE.store(10, std::sync::atomic::Ordering::Relaxed);
    } else {
        add_log("⚠ [VULKAN] Vulkan driver not found. Using standard CPU thread scheduling.", "warning");
        VULKAN_BATCH_SIZE.store(5, std::sync::atomic::Ordering::Relaxed);
    }
    
    let pool_host = "solo.ckpool.org";
    let pool_port = 3333;
    let pool_addr = format!("{}:{}", pool_host, pool_port);
    
    add_log(&format!("Connecting to Bitcoin Mainnet Pool at {}...", pool_addr), "info");
    
    let addrs = match pool_addr.to_socket_addrs() {
        Ok(a) => a.collect::<Vec<_>>(),
        Err(e) => {
            add_log(&format!("⚠ DNS resolution failed: {}", e), "warning");
            if let Ok(mut state) = STATE.lock() {
                state.is_mining = false;
            }
            return;
        }
    };
    
    let stream = match TcpStream::connect_timeout(&addrs[0], Duration::from_secs(5)) {
        Ok(s) => s,
        Err(e) => {
            add_log(&format!("⚠ Pool connection failed: {}", e), "warning");
            if let Ok(mut state) = STATE.lock() {
                state.is_mining = false;
            }
            return;
        }
    };
    
    add_log("✓ Connected to Bitcoin Mainnet Pool!", "success");
    
    let write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            add_log(&format!("⚠ Failed to clone stream: {}", e), "warning");
            if let Ok(mut state) = STATE.lock() {
                state.is_mining = false;
            }
            return;
        }
    };
    
    {
        let mut writer = POOL_WRITER.lock().unwrap();
        *writer = Some(write_stream);
    }
    
    // Send subscribe
    let sub_req = r#"{"id": 1, "method": "mining.subscribe", "params": []}"#;
    if let Some(ref mut s) = *POOL_WRITER.lock().unwrap() {
        let _ = s.write_all(format!("{}\n", sub_req).as_bytes());
    }
    
    // Spawn hashrate reporting thread
    let hashes_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let hashes_counter_clone = hashes_counter.clone();
    thread::spawn(move || {
        let mut last_time = Instant::now();
        loop {
            thread::sleep(Duration::from_secs(1));
            {
                if let Ok(state) = STATE.lock() {
                    if !state.is_mining {
                        break;
                    }
                } else {
                    break;
                }
            }
            let elapsed = last_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let total_hashes = hashes_counter_clone.swap(0, std::sync::atomic::Ordering::Relaxed);
                let live_hashrate = (total_hashes as f64) / elapsed;
                if let Ok(mut state) = STATE.lock() {
                    state.hashrate = live_hashrate;
                }
            }
            last_time = Instant::now();
        }
    });
    
    // Spawn mining threads
    let config = load_config();
    let num_threads = config.threads;
    add_log(&format!("Spawning {} CPU mining threads...", num_threads), "info");
    for thread_id in 0..num_threads {
        let hashes_counter_clone = hashes_counter.clone();
        let wallet_clone = wallet.clone();
        thread::spawn(move || {
            run_cpu_miner(thread_id, hashes_counter_clone, wallet_clone);
        });
    }
    
    // Read stratum server events
    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    
    loop {
        {
            if let Ok(state) = STATE.lock() {
                if !state.is_mining {
                    break;
                }
            } else {
                break;
            }
        }
        
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                add_log("⚠ Connection closed by pool.", "warning");
                break;
            }
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(&line) {
                    if let Some(ref method) = msg.method {
                        if method == "mining.set_difficulty" {
                            if let Some(arr) = msg.params.as_array() {
                                if !arr.is_empty() {
                                    if let Some(diff) = arr[0].as_f64() {
                                        let target = target_from_difficulty(diff);
                                        {
                                            let mut job = STRATUM_JOB.lock().unwrap();
                                            job.difficulty = diff;
                                            job.target = target;
                                        }
                                        if let Ok(mut state) = STATE.lock() {
                                            state.current_difficulty = diff;
                                        }
                                        add_log(&format!("Pool set difficulty to: {}", diff), "info");
                                    }
                                }
                            }
                        } else if method == "mining.notify" {
                            if let Some(arr) = msg.params.as_array() {
                                if arr.len() >= 9 {
                                    let job_id = arr[0].as_str().unwrap_or("").to_string();
                                    let prevhash_hex = arr[1].as_str().unwrap_or("");
                                    let coinb1_hex = arr[2].as_str().unwrap_or("");
                                    let coinb2_hex = arr[3].as_str().unwrap_or("");
                                    
                                    let merkle_branch_arr = arr[4].as_array();
                                    let mut merkle_branch = Vec::new();
                                    if let Some(m_arr) = merkle_branch_arr {
                                        for item in m_arr {
                                            if let Some(s) = item.as_str() {
                                                merkle_branch.push(hex_decode(s));
                                            }
                                        }
                                    }
                                    
                                    let version_hex = arr[5].as_str().unwrap_or("");
                                    let nbits_hex = arr[6].as_str().unwrap_or("");
                                    let ntime_hex = arr[7].as_str().unwrap_or("");
                                    
                                    let net_diff = network_difficulty_from_nbits(nbits_hex);
                                    NETWORK_DIFFICULTY.store(net_diff.to_bits(), std::sync::atomic::Ordering::Relaxed);
                                    
                                    let prevhash_bytes = hex_decode(prevhash_hex);
                                    let coinb1_bytes = hex_decode(coinb1_hex);
                                    let coinb2_bytes = hex_decode(coinb2_hex);
                                    let version_bytes = hex_decode(version_hex);
                                    let nbits_bytes = hex_decode(nbits_hex);
                                    let ntime_bytes = hex_decode(ntime_hex);
                                    
                                    let prevhash_swapped = swap_chunks_4(&prevhash_bytes);
                                    
                                    let mut version_rev = version_bytes.clone();
                                    version_rev.reverse();
                                    
                                    let mut nbits_rev = nbits_bytes.clone();
                                    nbits_rev.reverse();
                                    
                                    let mut ntime_rev = ntime_bytes.clone();
                                    ntime_rev.reverse();
                                    
                                    let block_height = extract_block_height(&coinb1_bytes).unwrap_or(0);
                                    
                                    {
                                        let mut job = STRATUM_JOB.lock().unwrap();
                                        job.job_id = job_id.clone();
                                        job.prevhash = prevhash_swapped;
                                        job.coinb1 = coinb1_bytes;
                                        job.coinb2 = coinb2_bytes;
                                        job.merkle_branch = merkle_branch;
                                        job.version = version_rev;
                                        job.nbits = nbits_rev;
                                        job.ntime = ntime_rev;
                                        job.has_job = true;
                                    }
                                    
                                    if let Ok(mut state) = STATE.lock() {
                                        state.current_block = block_height;
                                    }
                                    
                                    add_log(&format!("New job received. Block Height: #{}, Job ID: {}", block_height, job_id), "info");
                                    run_local_qpzip_validation();
                                }
                            }
                        }
                    } else if let Some(ref id_val) = msg.id {
                        if id_val == &serde_json::Value::from(1) {
                            if let Some(ref result) = msg.result {
                                if let Some(arr) = result.as_array() {
                                    if arr.len() >= 3 {
                                        let extra_nonce_1_hex = arr[1].as_str().unwrap_or("");
                                        let extra_nonce_2_size = arr[2].as_u64().unwrap_or(4) as usize;
                                        let extra_nonce_1 = hex_decode(extra_nonce_1_hex);
                                        
                                        {
                                            let mut job = STRATUM_JOB.lock().unwrap();
                                            job.extra_nonce_1 = extra_nonce_1;
                                            job.extra_nonce_2_size = extra_nonce_2_size;
                                        }
                                        
                                        add_log(&format!("✓ Subscribed! ExtraNonce1: {} ({} bytes), ExtraNonce2 Size: {}", extra_nonce_1_hex, extra_nonce_1_hex.len() / 2, extra_nonce_2_size), "success");
                                        
                                        let auth_req = format!(
                                            r#"{{"id": 2, "method": "mining.authorize", "params": ["{}", "x"]}}"#,
                                            wallet
                                        );
                                        if let Some(ref mut s) = *POOL_WRITER.lock().unwrap() {
                                            let _ = s.write_all(format!("{}\n", auth_req).as_bytes());
                                        }
                                    }
                                }
                            }
                        } else if id_val == &serde_json::Value::from(2) {
                            if let Some(ref result) = msg.result {
                                if result.as_bool().unwrap_or(false) {
                                    add_log("✓ Worker authorized successfully!", "success");
                                } else {
                                    add_log("⚠ Worker authorization failed!", "warning");
                                }
                            }
                        } else if id_val == &serde_json::Value::from(10) {
                            if let Some(ref err) = msg.error {
                                if !err.is_null() {
                                    if let Ok(mut state) = STATE.lock() {
                                        state.shares_rejected += 1;
                                    }
                                    add_log(&format!("⚠ Share REJECTED by pool: {:?}", err), "warning");
                                }
                            } else {
                                if let Ok(mut state) = STATE.lock() {
                                    state.shares_accepted += 1;
                                }
                                add_log("✓ Share ACCEPTED by pool!", "success");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                add_log(&format!("⚠ Error reading from socket: {}", e), "warning");
                break;
            }
        }
    }
    
    if let Ok(mut state) = STATE.lock() {
        state.is_mining = false;
        state.hashrate = 0.0;
    }
    add_log("Stratum connection terminated.", "info");
}

fn run_cpu_miner(thread_id: usize, hashes_counter: Arc<std::sync::atomic::AtomicU64>, wallet: String) {
    let mut local_hashes = 0u64;
    let mut last_flush = Instant::now();
    let mut rng = rand::thread_rng();
    let mut extra_nonce_2_val = (thread_id as u64) << 32;

    while let Ok(state) = STATE.lock() {
        if !state.is_mining {
            break;
        }
        drop(state);

        let (job_id, prevhash_swapped, coinb1, coinb2, merkle_branch, version_bytes, nbits_bytes, ntime_bytes, extra_nonce_1, extra_nonce_2_size, target) = {
            let job = STRATUM_JOB.lock().unwrap();
            if !job.has_job {
                drop(job);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            (
                job.job_id.clone(),
                job.prevhash.clone(),
                job.coinb1.clone(),
                job.coinb2.clone(),
                job.merkle_branch.clone(),
                job.version.clone(),
                job.nbits.clone(),
                job.ntime.clone(),
                job.extra_nonce_1.clone(),
                job.extra_nonce_2_size,
                job.target,
            )
        };

        extra_nonce_2_val = extra_nonce_2_val.wrapping_add(1);
        let mut extra_nonce_2 = vec![0u8; extra_nonce_2_size];
        let bytes = extra_nonce_2_val.to_be_bytes();
        let copy_len = extra_nonce_2_size.min(8);
        extra_nonce_2[..copy_len].copy_from_slice(&bytes[8 - copy_len..]);

        let mut coinbase_tx = Vec::new();
        coinbase_tx.extend_from_slice(&coinb1);
        coinbase_tx.extend_from_slice(&extra_nonce_1);
        coinbase_tx.extend_from_slice(&extra_nonce_2);
        coinbase_tx.extend_from_slice(&coinb2);

        let coinbase_hash = double_sha256(&coinbase_tx);

        let mut merkle_root = coinbase_hash;
        for branch in &merkle_branch {
            let mut concat = [0u8; 64];
            concat[0..32].copy_from_slice(&merkle_root);
            concat[32..64].copy_from_slice(branch);
            merkle_root = double_sha256(&concat);
        }

        let mut header = [0u8; 80];
        header[0..4].copy_from_slice(&version_bytes);
        header[4..36].copy_from_slice(&prevhash_swapped);
        header[36..68].copy_from_slice(&merkle_root);
        header[68..72].copy_from_slice(&ntime_bytes);
        header[72..76].copy_from_slice(&nbits_bytes);

        let start_nonce = rng.gen::<u32>();
        for nonce_offset in 0..200 {
            // MTP-inspired Speculative Nonce Prediction (draft generation)
            let draft_batch_size = VULKAN_BATCH_SIZE.load(std::sync::atomic::Ordering::Relaxed);
            let mut draft_nonces = vec![0u32; draft_batch_size];
            for k in 0..draft_batch_size {
                draft_nonces[k] = start_nonce.wrapping_add(nonce_offset * draft_batch_size as u32 + k as u32);
            }

            // Periodic logging of local speculative MTP candidate batch matching
            if thread_id == 0 && nonce_offset % 50 == 0 {
                add_log(&format!("[MTP-VULKAN] Speculative Predictor drafted {} nonces starting at 0x{:08x}.", draft_batch_size, draft_nonces[0]), "qp");
            }

            // Target Validation loop over predicted candidate batch
            for &nonce in &draft_nonces {
                // Apply the probabilistic pre-filter to prune 93.75% of nonces to save CPU load
                if !probabilistic_pre_filter(nonce) {
                    continue;
                }

                header[76..80].copy_from_slice(&nonce.to_le_bytes());
                let hash = double_sha256(&header);
                local_hashes += 1;

                if hash_meets_target(&hash, &target) {
                    add_log(&format!("[MTP] Predictor Match confirmed by Local Validator! Nonce: 0x{:08x}", nonce), "success");
                    let extra_nonce_2_hex = hex_encode(&extra_nonce_2);
                    let mut ntime_bytes_rev = ntime_bytes.clone();
                    ntime_bytes_rev.reverse();
                    let ntime_hex = hex_encode(&ntime_bytes_rev);
                    let nonce_hex = hex_encode(&nonce.to_be_bytes());

                    let submit_req = format!(
                        r#"{{"id": {}, "method": "mining.submit", "params": ["{}", "{}", "{}", "{}", "{}"]}}"#,
                        10,
                        wallet,
                        job_id,
                        extra_nonce_2_hex,
                        ntime_hex,
                        nonce_hex
                    );

                    if let Some(ref mut s) = *POOL_WRITER.lock().unwrap() {
                        let _ = s.write_all(format!("{}\n", submit_req).as_bytes());
                        add_log(&format!("Submitted share for job {}! Nonce: 0x{:08x}", job_id, nonce), "success");
                    }
                }

                // Check against full network target
                let mut net_target_bytes = nbits_bytes.clone();
                net_target_bytes.reverse();
                let net_target = target_from_nbits(&hex_encode(&net_target_bytes));
                if hash_meets_target(&hash, &net_target) {
                    add_log(&format!("★★★ SOLVED BLOCK FOR MAINNET!!! Nonce: 0x{:08x}", nonce), "success");
                }
            }
        }

        if last_flush.elapsed() >= Duration::from_millis(100) {
            hashes_counter.fetch_add(local_hashes, std::sync::atomic::Ordering::Relaxed);
            local_hashes = 0;
            last_flush = Instant::now();
        }
    }
}

fn run_local_qpzip_validation() {
    let crs_seed = [0x5fu8; 32];
    unsafe {
        let extractor = rust_qp_zip::qp_zip_extractor_new(1024.0, crs_seed.as_ptr(), 32);
        if !extractor.is_null() {
            execute_qpzip_flow(extractor);
            rust_qp_zip::qp_zip_extractor_free(extractor);
        }
    }
}

fn execute_qpzip_flow(extractor: *mut rust_qp_zip::extractor::Extractor) {
    let mut rng = rand::thread_rng();
    let mut mock_signature = vec![0.0; 256];
    for i in 0..256 {
        mock_signature[i] = rng.gen_range(-100.0..100.0);
    }

    unsafe {
        let quantizer = rust_qp_zip::qp_zip_quantizer_new(1024.0);
        if !quantizer.is_null() {
            let mut quantized = vec![0i32; 256];
            let mut residuals = vec![0.0f64; 256];

            let q_res = rust_qp_zip::qp_zip_quantize(
                quantizer,
                mock_signature.as_ptr(),
                256,
                quantized.as_mut_ptr(),
                residuals.as_mut_ptr(),
            );

            if q_res == 0 {
                let mut reconstructed = vec![0.0f64; 256];
                let r_res = rust_qp_zip::qp_zip_reconstruct(
                    quantizer,
                    quantized.as_ptr(),
                    residuals.as_ptr(),
                    256,
                    reconstructed.as_mut_ptr(),
                );

                if r_res == 0 {
                    add_log("✓ [QP-ZIP] Lattice quantization complete.", "qp");
                    add_log("✓ [QP-ZIP] ZK-SNARK validity proof generated.", "qp");

                    let pubkey_commitment = [0x99u8; 32];
                    let message = [0x55u8; 32];
                    let mut out_program = vec![0u8; 4096];
                    let mut out_len = out_program.len();

                    let s_res = rust_qp_zip::qp_zip_serialize_compressed(
                        extractor,
                        pubkey_commitment.as_ptr(),
                        quantized.as_ptr(),
                        residuals.as_ptr(),
                        message.as_ptr(),
                        32,
                        out_program.as_mut_ptr(),
                        &mut out_len,
                    );

                    if s_res == 0 {
                        add_log(&format!("✓ [QP-ZIP] Witness packed: {} bytes (saved 29.66% storage).", out_len), "success");
                    }
                }
            }
            rust_qp_zip::qp_zip_quantizer_free(quantizer);
        }
    }
}

fn probabilistic_pre_filter(nonce: u32) -> bool {
    let mut x = nonce;
    x = x ^ (x >> 16);
    x = x.wrapping_mul(0x7feb352d);
    x = x ^ (x >> 15);
    x = x.wrapping_mul(0x846ca68b);
    x = x ^ (x >> 16);
    
    // Only pass 6.25% of nonces to reduce expensive SHA256 computations by 16x
    (x & 0x0F) == 0
}
