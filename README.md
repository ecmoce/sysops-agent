# 🛡️ SysOps Agent

> **Lightweight System Monitoring Agent** — Security-focused Linux server monitoring daemon written in Rust

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Architecture](#-architecture)
- [Features](#-features)
- [Build](#-build)
- [Installation & Deployment](#-installation--deployment)
- [Configuration](#-configuration)
- [Alert Channel Configuration](#-alert-channel-configuration)
- [NATS Telemetry](#-nats-telemetry)
- [Usage](#-usage)
- [Documentation](#-documentation)
- [License](#-license)

---

## Related Projects

| Project | Description |
|---------|-------------|
| **sysops-agent** | Monitoring agent installed on servers (current repo) |
| [sysops-server](https://github.com/ecmoce/sysops-server) | Central data collection/API server |
| [sysops-console](https://github.com/ecmoce/sysops-console) | Web dashboard UI |

---

## 🔍 Overview

SysOps Agent runs as a daemon on Linux servers, performing **real-time anomaly detection**, **trend-based prediction**, **resource leak detection**, and **kernel/system log analysis** on system resources. When anomalies are detected, it immediately sends alerts through various channels including Discord, Slack, Telegram, Email, Webhook, NATS, and more.

It natively supports **enterprise server hardware** including multi-CPU socket servers, NVIDIA GPUs, NUMA topology, and automatically collects system inventory (OS, CPU, Memory, GPU specs) to periodically transmit to central management systems via NATS.

### Key Features

| Feature | Description |
|---------|-------------|
| 🦀 **Single Static Binary** | No runtime dependencies, deploy with single `scp` |
| ⚡ **Ultra-lightweight** | RSS < 50MB, CPU < 1% when idle |
| 🔒 **No root required** | Linux capabilities-based minimal privileges |
| 🚫 **No listening ports** | Default push-only, minimal attack surface |
| 🖥️ **Enterprise HW** | Multi-socket CPU, NVIDIA GPU, NUMA, ECC memory |
| 📡 **NATS Telemetry** | Periodic transmission of metrics/inventory to central aggregation system |
| 📊 **Prometheus Compatible** | Opt-in metrics endpoint |
| 📝 **TOML Configuration** | Intuitive and documented configuration file |

### Supported Distributions

| Distribution | Version | Build Verified |
|-------------|---------|---------------|
| Ubuntu | 20.04 / 22.04 / 24.04 | ✅ |
| Rocky Linux | 8 / 9 | ✅ |
| CentOS | 7 / 8 / 9 | ✅ |

---

## 🏗️ Architecture

### Overall System Architecture

```
┌──────────────────────────── Linux Server ─────────────────────────────┐
│                                                                       │
│  ┌──────────────────────── SysOps Agent ───────────────────────────┐  │
│  │                                                                 │  │
│  │  ╔═══════════════════╗                                         │  │
│  │  ║    Collectors     ║  /proc, /sys, nvidia-smi, dmidecode     │  │
│  │  ║                   ║                                         │  │
│  │  ║  CPU (per-socket) ║  /proc/stat, /sys/devices/system/node/  │  │
│  │  ║  Memory (DIMM)    ║  /proc/meminfo, /sys/devices/system/    │  │
│  │  ║  Disk  │ Network  ║  /proc/diskstats, /proc/net/dev         │  │
│  │  ║  GPU (NVIDIA)     ║  nvidia-smi --query-gpu, NVML           │  │
│  │  ║  FD    │ Process  ║  /proc/[pid]/, /proc/sys/fs/            │  │
│  │  ║  NUMA Topology    ║  /sys/devices/system/node/              │  │
│  │  ╚═════════╤═════════╝                                         │  │
│  │            │ MetricSample (mpsc channel)                       │  │
│  │            ▼                                                    │  │
│  │  ╔════════════════════════════╗                                │  │
│  │  ║    Ring Buffer Storage     ║◄─── Optional: SQLite           │  │
│  │  ║   (per-metric, 24h window) ║     (30-day retention)         │  │
│  │  ╚════════════╤═══════════════╝                                │  │
│  │               │ query (pull)                                    │  │
│  │               ▼                                                 │  │
│  │  ╔════════════════════════════╗                                │  │
│  │  ║        Analyzers           ║                                │  │
│  │  ║  Threshold │ Z-Score       ║  Anomaly Detection             │  │
│  │  ║  EMA       │ Trend(LinReg) ║  Predictive Analysis           │  │
│  │  ║  Leak Detect               ║  FD/Memory Leak                │  │
│  │  ╚════════════╤═══════════════╝                                │  │
│  │               │ Alert (mpsc channel)                            │  │
│  │               ▼                                                 │  │
│  │  ╔════════════════════════════╗       ┌───────────────────┐    │  │
│  │  ║      Alert Manager         ║──────▶│  📱 Discord       │    │  │
│  │  ║  • Rate Limiter            ║──────▶│  💬 Slack         │    │  │
│  │  ║  • Deduplication           ║──────▶│  ✈️ Telegram      │    │  │
│  │  ║  • Severity Routing        ║──────▶│  📧 Email/SMTP    │    │  │
│  │  ║  • Alert Grouping          ║──────▶│  🔗 Webhook       │    │  │
│  │  ╚════════════════════════════╝──────▶│  📋 Syslog        │    │  │
│  │                                       │  📡 NATS          │    │  │
│  │  ╔════════════════════════════╗       └───────────────────┘    │  │
│  │  ║      Log Analyzer          ║  dmesg, journal, syslog       │  │
│  │  ║  OOM │ HW Error │ FS Error ║──────▶ Alert Manager           │  │
│  │  ║  Hung Task │ Network │ GPU ║                                │  │
│  │  ╚════════════════════════════╝                                │  │
│  │                                                                 │  │
│  │  ╔════════════════════════════╗       ┌───────────────────┐    │  │
│  │  ║   System Inventory         ║──────▶│  📡 NATS Publish  │    │  │
│  │  ║  • OS release/kernel       ║       │  (periodic send)  │    │  │
│  │  ║  • CPU model/sockets/cores ║       └───────────────────┘    │  │
│  │  ║  • Memory DIMM/ECC spec   ║                                │  │
│  │  ║  • GPU model/VRAM/driver   ║                                │  │
│  │  ║  • Network interfaces      ║                                │  │
│  │  ║  • Disk model/serial       ║                                │  │
│  │  ╚════════════════════════════╝                                │  │
│  │                                                                 │  │
│  │  ┌────────────────┐  ┌───────────────────┐                     │  │
│  │  │ Config (TOML)  │  │ Prometheus (opt)  │ :9100/metrics       │  │
│  │  └────────────────┘  └───────────────────┘                     │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
                       10s/30s/60s
[/proc, /sys, NVML] ────collect────▶ [MetricSample]
                                          │
                                    ──store──▶ [RingBuffer] ──persist──▶ [SQLite?]
                                                   │
                                             ──analyze──▶ [Alert]
                                                            │
                                  ┌──────────┬──────────────┼────────────┬──────────┐
                                  ▼          ▼              ▼            ▼          ▼
                             [Discord]  [Slack]  [Telegram/Email]  [Webhook]   [NATS]

[dmidecode, lscpu, NVML] ────inventory────▶ [SystemInfo] ──publish──▶ [NATS]
                                                                    (period: 5min)
```

### NATS-based Central Aggregation Topology

```
 ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
 │ Web Server  │  │ DB Server   │  │ GPU Server  │
 │ SysOps Agent│  │ SysOps Agent│  │ SysOps Agent│
 └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
        │                │                │
        │   NATS Publish (metrics/alerts/inventory)
        │                │                │
        ▼                ▼                ▼
 ┌─────────────────────────────────────────────────┐
 │              NATS Server / Cluster               │
 │                                                  │
 │  Subject hierarchy:                              │
 │  sysops.{hostname}.metrics    ← periodic metrics │
 │  sysops.{hostname}.alerts     ← anomaly alerts   │
 │  sysops.{hostname}.inventory  ← system inventory │
 │  sysops.{hostname}.heartbeat  ← liveness signals │
 └──────────────────────┬──────────────────────────┘
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
   ┌────────────┐ ┌──────────┐ ┌───────────┐
   │ Dashboard  │ │ Alerting │ │ Inventory │
   │ (Grafana)  │ │ Gateway  │ │ CMDB      │
   └────────────┘ └──────────┘ └───────────┘
```

---

## ✨ Features

### Metric Collection

| Category | Metrics | Source | Period |
|----------|---------|--------|---------|
| **CPU** | usage%, per-core, per-socket, iowait, steal, load avg | `/proc/stat`, `/proc/loadavg` | 10s |
| **CPU Topology** | per-socket usage, per-NUMA node statistics | `/sys/devices/system/node/` | 10s |
| **Memory** | used%, available, buffers/cached, swap, NUMA per-node | `/proc/meminfo`, `/sys/devices/system/node/*/meminfo` | 10s |
| **Memory HW** | ECC error count (correctable/uncorrectable) | `/sys/devices/system/edac/mc*/` | 60s |
| **Disk** | usage%, inode%, I/O rate, latency, SMART health | `/proc/diskstats`, `statvfs()` | 10~60s |
| **Network** | rx/tx bytes, packets, errors, drops, per-interface | `/proc/net/dev` | 10s |
| **GPU (NVIDIA)** | utilization%, memory used/total, temperature, power, ECC | NVML / `nvidia-smi` | 10s |
| **Process** | top-N by CPU/RSS, count, zombie count, GPU process | `/proc/[pid]/stat`, NVML | 30s |
| **File Descriptors** | system-wide used/max, per-process fd count | `/proc/sys/fs/file-nr` | 30s |
| **Kernel** | OOM kills, hardware errors, hung tasks, GPU Xid errors | dmesg, journal, syslog | real-time |

### System Inventory (Auto-collection)

The agent collects system hardware/software information at startup and periodically (default 5 minutes).

| Category | Collected Items | Source |
|----------|----------------|--------|
| **OS** | distro, version, kernel version, architecture, hostname | `/etc/os-release`, `uname` |
| **CPU** | model name, vendor, sockets, cores/socket, threads/core, MHz, cache sizes, flags (avx, sse), microcode | `/proc/cpuinfo`, `lscpu`, `/sys/devices/system/cpu/` |
| **NUMA** | node count, CPU-to-node mapping, memory per node | `/sys/devices/system/node/` |
| **Memory** | total, DIMM count, DIMM size/type/speed/manufacturer, ECC support | `/proc/meminfo`, `/sys/devices/system/memory/`, `dmidecode` |
| **GPU** | model, VRAM total, driver version, CUDA version, GPU count, PCIe gen/width, power limit | NVML / `nvidia-smi -q` |
| **Disk** | model, serial, capacity, interface (NVMe/SAS/SATA), firmware, SMART status | `/sys/block/*/device/`, `smartctl` |
| **Network** | interface name, MAC, speed, MTU, driver, firmware | `/sys/class/net/*/`, `ethtool` |
| **BIOS/Board** | vendor, version, serial, product name | `/sys/devices/virtual/dmi/id/`, `dmidecode` |

**Inventory JSON Example:**

```json
{
  "hostname": "gpu-server-01",
  "collected_at": "2026-02-22T16:30:00Z",
  "os": {
    "distro": "Ubuntu",
    "version": "22.04.4 LTS",
    "kernel": "5.15.0-91-generic",
    "arch": "x86_64"
  },
  "cpu": {
    "model": "Intel Xeon Gold 6348 @ 2.60GHz",
    "vendor": "GenuineIntel",
    "sockets": 2,
    "cores_per_socket": 28,
    "threads_per_core": 2,
    "total_threads": 112,
    "cache_l3_mb": 42,
    "flags": ["avx512f", "avx512bw", "avx512vl"]
  },
  "numa": {
    "nodes": 2,
    "topology": [
      { "node": 0, "cpus": "0-27,56-83", "memory_mb": 262144 },
      { "node": 1, "cpus": "28-55,84-111", "memory_mb": 262144 }
    ]
  },
  "memory": {
    "total_gb": 512,
    "dimm_count": 16,
    "dimms": [
      { "slot": "DIMM_A1", "size_gb": 32, "type": "DDR4", "speed_mhz": 3200, "manufacturer": "Samsung", "ecc": true }
    ]
  },
  "gpu": [
    {
      "index": 0,
      "model": "NVIDIA A100-SXM4-80GB",
      "vram_gb": 80,
      "driver": "535.129.03",
      "cuda": "12.2",
      "pcie_gen": 4,
      "pcie_width": 16,
      "power_limit_w": 400,
      "ecc": true
    }
  ],
  "disks": [
    { "name": "nvme0n1", "model": "Samsung PM9A3 3.84TB", "capacity_gb": 3840, "interface": "NVMe", "smart_healthy": true }
  ],
  "network": [
    { "name": "eno1", "mac": "aa:bb:cc:dd:ee:ff", "speed_mbps": 25000, "mtu": 9000, "driver": "mlx5_core" }
  ]
}
```

### Anomaly Detection Algorithms

| Algorithm | Purpose | Operation |
|-----------|---------|-----------|
| **Threshold** | Immediate danger detection | Instant alert when configured threshold exceeded |
| **Z-Score** | Statistical anomaly detection | Detect 3σ deviations based on recent 1-hour data |
| **EMA** | Sudden change detection | Deviation from Exponential Moving Average |
| **Trend (Linear Regression)** | Resource depletion prediction | Predict disk full in 24h, OOM in 6h |
| **Leak Detection** | FD/memory leak | RSS monotonic increase + R² > 0.8 pattern detection |

### Log Analysis

| Pattern | Severity | Example |
|---------|----------|---------|
| OOM Kill | 🔴 Critical | `Out of memory: Killed process 1234 (java)` |
| Hardware Error | 🔴 Critical | `Machine check`, `ECC error`, `EDAC` |
| GPU Xid Error | 🔴 Critical | `NVRM: Xid ...: 79, pid=1234, GPU has fallen off the bus` |
| GPU ECC | 🟡 Warn | `NVRM: ...ECC... DBE (double bit error)` |
| Filesystem Error | 🔴 Critical | `EXT4-fs error`, `Remounting read-only` |
| NVMe Error | 🔴 Critical | `nvme nvme0: I/O error`, `critical warning` |
| Hung Task | 🟡 Warn | `task java blocked for more than 120 seconds` |
| Network Down | 🟡 Warn | `NIC Link is Down`, `carrier lost` |
| PCIe Error | 🟡 Warn | `PCIe Bus Error`, `AER: Corrected error` |

---

## 🔨 Build

### Requirements

- Rust 1.75+ (stable)
- Linux or cross-compilation environment

### Basic Build

```bash
cargo build --release
```

### Feature Flags

The default build includes only Core functionality. Additional features are enabled via feature flags.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Feature Flags                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─ Core (included by default) ─────────────────────────────────┐    │
│  │  • CPU, Memory, Disk, Network, Process, FD collection       │    │
│  │  • Threshold, Z-Score, EMA, Trend, Leak analysis            │    │
│  │  • Discord, Slack, Telegram, Email, Webhook, Syslog alerts  │    │
│  │  • Log Analyzer (dmesg, syslog, journal)                    │    │
│  │  • System Inventory (OS, CPU, Memory, Disk, Network)        │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ gpu ──────────────────────────────────────────────────────┐    │
│  │  NVIDIA GPU monitoring (NVML bindings)                      │    │
│  │  • GPU utilization, memory, temperature, power, ECC         │    │
│  │  • Per-process GPU usage, Xid error detection               │    │
│  │  • GPU inventory (model, VRAM, driver, CUDA version)        │    │
│  │  ⚠️  Runtime requirement: NVIDIA driver + libnvidia-ml.so   │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ nats ─────────────────────────────────────────────────────┐    │
│  │  NATS messaging support                                     │    │
│  │  • Periodic publish of metrics/alerts/inventory             │    │
│  │  • Heartbeat (liveness signals)                             │    │
│  │  • Subject: sysops.{hostname}.{metrics|alerts|inventory}    │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ prometheus ───────────────────────────────────────────────┐    │
│  │  Prometheus metrics endpoint (:9100/metrics)                │    │
│  │  • Expose all collected metrics in Prometheus format        │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ sqlite ───────────────────────────────────────────────────┐    │
│  │  Long-term metric storage (SQLite)                          │    │
│  │  • 1-minute average downsampling, 30-day retention          │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ TLS (choose one) ─────────────────────────────────────────┐    │
│  │  tls-rustls   Pure Rust TLS (no external deps, recommended) │    │
│  │  tls-native   OpenSSL-based TLS (use system CA certs)      │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Build Examples:**

```bash
# Minimal build (Core only, alerts only)
cargo build --release

# For GPU servers
cargo build --release --features "gpu,nats,tls-rustls"

# Full features
cargo build --release --features "gpu,nats,prometheus,sqlite,tls-rustls"

# Monitoring server integration (NATS + Prometheus)
cargo build --release --features "nats,prometheus,sqlite"
```

### Static Binary (musl)

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# → glibc version independent, runs anywhere
```

### Docker Multi-OS Build

```bash
# Individual OS
docker build --platform linux/amd64 -f docker/Dockerfile.ubuntu2204 -t sysops-agent:ubuntu2204 .

# All OS build & test
./scripts/build-test-all.sh
```

### Testing

```bash
cargo test
cargo test --features "gpu,nats,sqlite" -- --test-threads=1
```

---

## 📦 Installation & Deployment

### Method 1: Direct Binary Copy

```bash
# Build
cargo build --release --target x86_64-unknown-linux-musl --features "gpu,nats,tls-rustls"

# Deploy
scp target/x86_64-unknown-linux-musl/release/sysops-agent user@server:/usr/local/bin/
scp config.toml user@server:/etc/sysops-agent/config.toml
```

### Method 2: systemd Service

```bash
sudo cp sysops-agent /usr/local/bin/
sudo chmod 755 /usr/local/bin/sysops-agent
sudo mkdir -p /etc/sysops-agent
sudo cp config.toml /etc/sysops-agent/
sudo chmod 600 /etc/sysops-agent/config.toml
sudo cp deploy/sysops-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now sysops-agent
```

**systemd unit file** (`deploy/sysops-agent.service`):

```ini
[Unit]
Description=SysOps Agent - System Monitoring Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/sysops-agent --config /etc/sysops-agent/config.toml
Restart=always
RestartSec=10

# Security Hardening
User=sysops-agent
Group=sysops-agent
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
NoNewPrivileges=yes
CapabilityBoundingSet=CAP_DAC_READ_SEARCH CAP_SYSLOG
AmbientCapabilities=CAP_DAC_READ_SEARCH CAP_SYSLOG
ReadOnlyPaths=/proc /sys /var/log
# For GPU access if needed
SupplementaryGroups=video

[Install]
WantedBy=multi-user.target
```

### Method 3: Ansible

```bash
ansible-playbook -i inventory deploy/ansible/playbook.yml
```

---

## ⚙️ Configuration

Configuration file: `/etc/sysops-agent/config.toml`

### Minimal Configuration

```toml
[agent]
hostname = "web-server-01"

[thresholds]
cpu_percent = 90.0
memory_percent = 85.0
disk_percent = 90.0

[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/YOUR/WEBHOOK"
```

### Full Configuration Example

```toml
[agent]
hostname = "gpu-server-01"
collect_interval_secs = 10
log_level = "info"                    # trace, debug, info, warn, error
data_dir = "/var/lib/sysops-agent"
pid_file = "/var/run/sysops-agent.pid"

# ─── Collection Intervals ───
[collector]
cpu_interval_secs = 10
memory_interval_secs = 10
disk_interval_secs = 60
network_interval_secs = 10
process_interval_secs = 30
fd_interval_secs = 30
gpu_interval_secs = 10                # requires feature "gpu"

# ─── System Inventory ───
[inventory]
enabled = true
collect_interval_secs = 300           # collect/send every 5 minutes
include_dimm_details = true           # DIMM details (dmidecode, requires root)
include_smart = false                 # SMART info (smartctl, requires root)
include_bios = true                   # BIOS/board info

# ─── Thresholds ───
[thresholds]
cpu_percent = 90.0
cpu_per_socket_percent = 95.0         # per-socket threshold
memory_percent = 85.0
disk_percent = 90.0
disk_inode_percent = 85.0
fd_percent = 80.0
load_avg_multiplier = 2.0             # load > (CPU cores × multiplier)
network_error_rate = 0.01

# GPU thresholds (feature "gpu")
gpu_utilization_percent = 95.0
gpu_memory_percent = 90.0
gpu_temperature_celsius = 85.0        # alert before thermal throttling
gpu_power_percent = 95.0              # vs power limit

# ─── Multi-socket / NUMA ───
[cpu]
per_socket_monitoring = true          # separate monitoring per socket
numa_monitoring = true                # per-NUMA node memory stats
ecc_monitoring = true                 # EDAC ECC error count

# ─── Analyzers ───
[analyzer]
zscore_window = 360
zscore_threshold = 3.0
ema_alpha = 0.1
trend_window_hours = 6
leak_min_observation_hours = 1
leak_r_squared_threshold = 0.8

# ─── Storage ───
[storage]
ring_buffer_capacity = 8640
sqlite_enabled = false                # requires feature "sqlite"
sqlite_path = "/var/lib/sysops-agent/metrics.db"
sqlite_retention_days = 30

# ─── Log Analysis ───
[log_analyzer]
enabled = true
sources = ["dmesg", "syslog"]
syslog_path = "/var/log/syslog"
gpu_xid_monitoring = true             # NVIDIA Xid error detection
custom_patterns = [
    { pattern = "FATAL.*database", severity = "critical", name = "db_fatal" },
    { pattern = "connection refused", severity = "warn", name = "conn_refused" },
]

# ─── NATS Telemetry (feature "nats") ───
[nats]
enabled = true
url = "nats://nats-server:4222"       # NATS server address
# urls = ["nats://n1:4222", "nats://n2:4222"]  # cluster
credential_file = "/etc/sysops-agent/nats.creds"  # auth (optional)
# token = "${NATS_TOKEN}"             # token auth
subject_prefix = "sysops"             # → sysops.{hostname}.*
metrics_interval_secs = 30            # metrics transmission period
inventory_interval_secs = 300         # inventory transmission period
heartbeat_interval_secs = 60          # heartbeat period
include_alerts = true                 # also send alerts to NATS
batch_size = 100                      # metric batch size
compression = true                    # payload compression (zstd)

# ─── Prometheus (feature "prometheus") ───
[prometheus]
enabled = false
bind = "127.0.0.1:9100"
path = "/metrics"

# ─── Common Alert Settings ───
[alerting]
min_interval_secs = 300
max_alerts_per_hour = 60
dedup_window_secs = 600
emergency_bypass_rate_limit = true
```

---

## 📡 Alert Channel Configuration

### Channel-specific Configuration

#### 1. 📱 Discord (Webhook)

```
┌─────────────┐      HTTPS POST       ┌──────────────┐
│ SysOps Agent │ ───────────────────▶  │ Discord API  │
│              │  JSON (embeds)        │ /webhooks/   │
└─────────────┘                        └──────┬───────┘
                                              ▼
                                       ┌──────────────┐
                                       │  #alerts channel │
                                       └──────────────┘
```

**Setup:** Discord Server → Channel Settings → Integrations → Webhooks → Copy URL

```toml
[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/1234567890/abcdefgh"
username = "SysOps Agent"
mention_roles = ["@devops"]           # mention on Critical+
```

#### 2. 💬 Slack (Webhook)

```
┌─────────────┐      HTTPS POST       ┌──────────────┐
│ SysOps Agent │ ───────────────────▶  │ Slack API    │
│              │  JSON (blocks)        │ /incoming-   │
└─────────────┘                        │  webhooks/   │
                                       └──────────────┘
```

**Setup:** Slack App → Incoming Webhooks enabled → Select channel

```toml
[alerting.slack]
enabled = true
webhook_url = "https://hooks.slack.com/services/T00/B00/xxxx"
channel = "#server-alerts"
mention_users = ["U12345"]
```

#### 3. ✈️ Telegram (Bot API)

```
┌─────────────┐      HTTPS POST       ┌──────────────┐
│ SysOps Agent │ ───────────────────▶  │ Telegram     │
│              │  /sendMessage         │ Bot API      │
└─────────────┘                        └──────────────┘
```

**Setup:** @BotFather → `/newbot` → Token + Chat ID

```toml
[alerting.telegram]
enabled = true
bot_token = "${TELEGRAM_BOT_TOKEN}"
chat_id = "-1001234567890"
parse_mode = "HTML"
```

#### 4. 📧 Email (SMTP)

```toml
[alerting.email]
enabled = true
smtp_host = "smtp.gmail.com"
smtp_port = 587
smtp_tls = true
username = "alerts@company.com"
password = "${SMTP_PASSWORD}"
from = "SysOps Agent <alerts@company.com>"
to = ["devops@company.com", "oncall@company.com"]
subject_prefix = "[SysOps]"
min_severity = "critical"             # Critical+ only for email
```

#### 5. 🔗 Custom Webhook

```toml
[alerting.webhook]
enabled = true
url = "https://api.company.com/alerts"
method = "POST"
headers = { "Authorization" = "Bearer ${WEBHOOK_TOKEN}" }
timeout_secs = 10
retry_count = 3
```

**Payload:**
```json
{
  "hostname": "gpu-server-01",
  "timestamp": "2026-02-22T16:30:00+09:00",
  "severity": "critical",
  "metric": "gpu_temperature",
  "value": 87.0,
  "threshold": 85.0,
  "message": "GPU 0 temperature 87°C exceeds threshold 85°C",
  "labels": { "gpu_index": "0", "gpu_model": "A100" }
}
```

#### 6. 📋 Local Syslog

```toml
[alerting.syslog]
enabled = true
facility = "daemon"
tag = "sysops-agent"
```

### Severity Routing

```
┌───────────┬─────────┬───────┬──────────┬───────┬─────────┬────────┬──────┐
│ Severity  │ Discord │ Slack │ Telegram │ Email │ Webhook │ Syslog │ NATS │
├───────────┼─────────┼───────┼──────────┼───────┼─────────┼────────┼──────┤
│ Info      │    -    │   -   │    -     │   -   │    ✓    │   ✓    │  ✓   │
│ Warn      │    ✓    │   ✓   │    ✓     │   -   │    ✓    │   ✓    │  ✓   │
│ Critical  │   ✓ *   │  ✓ * │    ✓     │   ✓   │    ✓    │   ✓    │  ✓   │
│ Emergency │   ✓ *   │  ✓ * │    ✓     │   ✓   │    ✓    │   ✓    │  ✓   │
└───────────┴─────────┴───────┴──────────┴───────┴─────────┴────────┴──────┘
  * = with @mention / rate limit bypass for Emergency
```

---

## 📡 NATS Telemetry

The NATS channel is not for one-way alerts, but for **periodic telemetry transmission**. It publishes metrics, inventory, alerts, and heartbeats to NATS subjects for subscription by central management systems.

### Subject Structure

```
sysops.{hostname}.metrics      metric batches (every 30s)
sysops.{hostname}.alerts       anomaly detection alerts (on occurrence)
sysops.{hostname}.inventory    system inventory (every 5 min)
sysops.{hostname}.heartbeat    liveness signals (every 1 min)
```

### Transmitted Data Format

**Heartbeat** (`sysops.gpu-server-01.heartbeat`):
```json
{
  "hostname": "gpu-server-01",
  "timestamp": "2026-02-22T16:30:00Z",
  "uptime_secs": 864000,
  "agent_version": "0.1.0",
  "status": "healthy"
}
```

**Metrics** (`sysops.gpu-server-01.metrics`):
```json
{
  "hostname": "gpu-server-01",
  "timestamp": "2026-02-22T16:30:00Z",
  "metrics": [
    { "name": "cpu_usage_percent", "value": 45.2, "labels": {} },
    { "name": "cpu_socket_usage_percent", "value": 78.1, "labels": { "socket": "0" } },
    { "name": "cpu_socket_usage_percent", "value": 12.3, "labels": { "socket": "1" } },
    { "name": "memory_used_percent", "value": 62.8, "labels": {} },
    { "name": "memory_numa_used_percent", "value": 71.2, "labels": { "node": "0" } },
    { "name": "gpu_utilization_percent", "value": 92.0, "labels": { "gpu": "0", "model": "A100" } },
    { "name": "gpu_memory_used_gb", "value": 71.2, "labels": { "gpu": "0" } },
    { "name": "gpu_temperature_celsius", "value": 72.0, "labels": { "gpu": "0" } },
    { "name": "gpu_power_watts", "value": 380.5, "labels": { "gpu": "0" } },
    { "name": "ecc_correctable_errors", "value": 2, "labels": { "mc": "0" } }
  ]
}
```

**Inventory** (`sysops.gpu-server-01.inventory`):
Full system inventory JSON (see "System Inventory" section above)

### NATS Configuration Example

```toml
[nats]
enabled = true
url = "nats://nats-server:4222"

# Cluster configuration
# urls = ["nats://n1:4222", "nats://n2:4222", "nats://n3:4222"]

# Authentication
# credential_file = "/etc/sysops-agent/nats.creds"   # NKey auth
# token = "${NATS_TOKEN}"                             # token auth
# user = "sysops"                                     # user/password
# password = "${NATS_PASSWORD}"

# Subject configuration
subject_prefix = "sysops"              # → sysops.{hostname}.*

# Transmission periods
metrics_interval_secs = 30             # metrics (default 30s)
inventory_interval_secs = 300          # inventory (default 5min)
heartbeat_interval_secs = 60           # heartbeat (default 1min)

# Optimization
batch_size = 100                       # metric batch size
compression = true                     # zstd compression (bandwidth saving)
max_reconnect_attempts = -1            # infinite reconnection
reconnect_delay_secs = 5
```

### Central Subscription Example (Go/Python)

```bash
# Subscribe test with nats CLI
nats sub "sysops.>"                    # all agents
nats sub "sysops.gpu-server-01.>"      # specific server
nats sub "sysops.*.alerts"             # alerts from all servers only
```

---

## 🚀 Usage

### CLI Commands

```bash
# Basic execution
sysops-agent --config /etc/sysops-agent/config.toml

# Foreground + debugging
sysops-agent --config config.toml --log-level debug

# Configuration validation only
sysops-agent --config config.toml --validate

# One-time inventory output (for installation verification)
sysops-agent --config config.toml --inventory-dump

# Version/help
sysops-agent --version
sysops-agent --help
```

### systemd Service Management

```bash
sudo systemctl start sysops-agent
sudo systemctl status sysops-agent
journalctl -u sysops-agent -f
```

### Prometheus Integration (optional)

```bash
curl http://localhost:9100/metrics
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [DESIGN.md](docs/DESIGN.md) | Architecture and detailed design (algorithms, security model, performance) |
| [METRICS.md](docs/METRICS.md) | Collected metrics catalog (80+ metrics) |
| [ALERTING.md](docs/ALERTING.md) | Alert system detailed design |
| [DEPLOYMENT.md](docs/DEPLOYMENT.md) | Deployment guide (systemd, RPM, DEB, Ansible) |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | Complete configuration reference |
| [BUILD-TEST-RESULTS.md](docs/BUILD-TEST-RESULTS.md) | Per-OS build/test results |

---

## 🤝 Contributing

1. Fork → Branch → PR
2. Must pass `cargo fmt && cargo clippy`
3. Update METRICS.md when adding new metrics

## 📄 License

MIT License — [LICENSE](LICENSE)