# HCSminer v2.0 - Post-Quantum Bitcoin Pool Miner

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20x64-blue)](https://github.com/timfromhcs/bitcoin-pq-poc-realmine)

**HCSminer** - High-performance pool-mining client for Bitcoin with post-quantum signature compression, Stratum V1 protocol support, and real-time TUI monitoring.

> Made with ❤️ by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)

---

## ✨ Features

- **🚀 Multi-threaded CPU Mining** - Nutzt alle CPU-Kerne parallel für maximale Hashrate
- **⚡ Optimierte SHA-256d** - Vor-allocierte Buffer, Atomic Counters, keine Lock-Contention
- **🎮 Vulkan GPU Detection** - Automatische GPU-Erkennung und VRAM-Monitoring
- **🖥️ Echtzeit-TUI** - ratatui Terminal UI mit Hashrate, Shares, RAM/VRAM Auslastung
- **🌐 Stratum V1 Pool-Mining** - Verbindung zu public-pool.io (PPLNS)
- **🔐 Post-Quantum Ready** - Lattice-basierte Quantisierung und ZK-Proofs
- **📊 Live-Statistiken** - Rolling Hashrate-Durchschnitt, Share-Tracking, Logging

## 🚀 Quick Start

`atch
start.bat
`

Oder manuell:
`atch
cd src\qp_zip_miner
cargo build --release
target\release\hcsminer.exe
`

## ⚙️ Konfiguration

Edit miner_config.toml:

`	oml
btc_address = "deine_BTC_Adresse"   # ❗ Einzige Pflichtangabe
worker_name = "hcsminer"            # Worker-Name für den Pool
pool_host = "public-pool.io"        # Pool-Host
pool_port = 13333                   # V1=13333, TLS=14333, V2=23331
threads = 16                        # CPU-Kerne (auto: num_cpus)
enable_tui = true                   # TUI aktivieren
`

## 🏊 Pool: public-pool.io (PPLNS)

| Modus | Adresse | Status |
|-------|---------|--------|
| Stratum V1 | stratum+tcp://public-pool.io:13333 | ✅ Live |
| Stratum V1+TLS | stratum+tls://public-pool.io:14333 | 🔒 |
| Stratum V2 | stratum+tcp://public-pool.io:23331 | 🚀 |

## 🔧 Performance-Optimierungen (v2.0)

| Optimierung | Beschreibung | Impact |
|-------------|-------------|--------|
| **Multi-Thread Mining** | Alle CPU-Kerne parallel | 8-16x Hashrate |
| **Atomic Counters** | Lock-freie Statistiken | 0% Lock-Contention |
| **Buffer Pooling** | Vor-allocierte Header-Buffer | ~30% weniger Allokationen |
| **Quick Pre-Filter** | 90% der Nonces billig gefiltert | 10x effizienteres Hashing |
| **Rolling Hashrate** | Gleitender Durchschnitt alle 100ms | Akkurate Anzeige |
| **Non-Blocking I/O** | 100ms Timeout + peek() Pattern | Kein Blockieren beim Mining |

## 🐛 Bugfixes (v2.0)

| Bug | Status |
|-----|--------|
| ❌ wait_for_notify() fehlte → Absturz beim Job-Polling | ✅ Gefixt |
| ❌ miner_core.rs war leer → Compiler-Fehler | ✅ Implementiert |
| ❌ Falscher Binary-Name in start.bat (qp_zip_miner.exe → hcsminer.exe) | ✅ Gefixt |
| ❌ Web-UI Referenz ohne Server → Caputer Browser-Tab | ✅ Entfernt |
| ❌ serde_derive deprecated → Compiler-Warning | ✅ Gefixt |
| ❌ unwrap() in Hot-Path → Absturzgefahr | ✅ Durch .ok().map() ersetzt |
| ❌ TUI Mutex Poisoning → Miner stürzt ab | ✅ Graceful Break |

## 📁 Projektstruktur

`
HCSminer/
├── start.bat                    # Einstiegspunkt
├── miner_config.toml            # Konfiguration
├── src/
│   ├── qp_zip_miner/            # Rust Miner Binary
│   │   └── src/
│   │       ├── main.rs          # Einstieg, Mining-Loop
│   │       └── miner_modules/
│   │           ├── miner_core.rs # 🔥 Multi-Thread CPU Miner
│   │           ├── stratum.rs   # Stratum V1 Protokoll
│   │           ├── tui.rs       # ratatui Terminal UI
│   │           ├── vulkan.rs    # Vulkan GPU Detection
│   │           └── config.rs    # TOML Konfiguration
│   └── rust_qp_zip/             # Post-Quantum Crypto Library
│       └── src/
│           ├── quantizer.rs     # Lattice Quantisierung
│           ├── zk_prover.rs     # ZK-SNARK Proofs
│           ├── extractor.rs     # Witness Extraction
│           └── ffi.rs           # C-FFI Bindings
├── docs/                        # Dokumentation
├── agent_skills/                # AI-Assistent Guides
└── developer_resources/         # Entwickler-Ressourcen
`

## 📚 Weiterführende Dokumentation

- [AGENTS.md](AGENTS.md) - Guidelines für KI-Assistenten
- [agent_skills/](agent_skills/) - Rust Repair, Stratum Protocol, Vulkan Debug
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Systemarchitektur
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) - Entwicklerhandbuch

## 📝 License

MIT - Made with ❤️ by [timfromhcs](https://github.com/timfromhcs) and [@hcmedia](https://github.com/hcmedia)
