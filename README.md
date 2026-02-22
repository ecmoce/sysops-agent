# 🛡️ SysOps Agent

> **경량 시스템 모니터링 에이전트** — Rust로 작성된 보안 중심의 Linux 서버 모니터링 데몬

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

---

## 📋 목차

- [개요](#-개요)
- [아키텍처](#-아키텍처)
- [기능](#-기능)
- [빌드](#-빌드)
- [설치 및 배포](#-설치-및-배포)
- [설정](#-설정)
- [알림 채널 설정](#-알림-채널-설정)
- [NATS 텔레메트리](#-nats-텔레메트리)
- [사용법](#-사용법)
- [문서](#-문서)
- [라이선스](#-라이선스)

---

## 관련 프로젝트

| 프로젝트 | 설명 |
|----------|------|
| **sysops-agent** | 서버에 설치되는 모니터링 에이전트 (현재 레포) |
| [sysops-server](https://github.com/ecmoce/sysops-server) | 중앙 데이터 수집/API 서버 |
| [sysops-console](https://github.com/ecmoce/sysops-console) | 웹 대시보드 UI |

---

## 🔍 개요

SysOps Agent는 Linux 서버에서 데몬으로 실행되며, 시스템 리소스의 **실시간 이상 탐지**, **트렌드 기반 예측**, **리소스 누수 감지**, **커널/시스템 로그 분석**을 수행합니다. 이상 발견 시 Discord, Slack, Telegram, Email, Webhook, NATS 등 다양한 채널로 즉시 알림을 전송합니다.

멀티 CPU 소켓 서버, NVIDIA GPU, NUMA 토폴로지 등 **엔터프라이즈 서버 하드웨어**를 네이티브 지원하며, 시스템 인벤토리(OS, CPU, Memory, GPU 스펙)를 자동 수집하여 NATS를 통해 중앙 관리 시스템에 주기적으로 전송합니다.

### 핵심 특징

| 특징 | 설명 |
|------|------|
| 🦀 **단일 정적 바이너리** | 런타임 의존성 없음, `scp` 하나로 배포 |
| ⚡ **초경량** | RSS < 50MB, 유휴 시 CPU < 1% |
| 🔒 **root 불필요** | Linux capabilities 기반 최소 권한 |
| 🚫 **수신 포트 없음** | 기본 push-only, 공격 표면 최소화 |
| 🖥️ **엔터프라이즈 HW** | 멀티소켓 CPU, NVIDIA GPU, NUMA, ECC 메모리 |
| 📡 **NATS 텔레메트리** | 중앙 집계 시스템으로 메트릭/인벤토리 주기 전송 |
| 📊 **Prometheus 호환** | opt-in metrics endpoint 제공 |
| 📝 **TOML 설정** | 직관적이고 문서화된 설정 파일 |

### 지원 배포판

| 배포판 | 버전 | 빌드 검증 |
|--------|------|-----------|
| Ubuntu | 20.04 / 22.04 / 24.04 | ✅ |
| Rocky Linux | 8 / 9 | ✅ |
| CentOS | 7 / 8 / 9 | ✅ |

---

## 🏗️ 아키텍처

### 전체 시스템 구성도

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
│  │  ║  • OS release/kernel       ║       │  (주기적 전송)    │    │  │
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

### 데이터 흐름

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
                                                                    (주기: 5분)
```

### NATS 기반 중앙 집계 토폴로지

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
 │  sysops.{hostname}.metrics    ← 주기적 메트릭    │
 │  sysops.{hostname}.alerts     ← 이상 알림        │
 │  sysops.{hostname}.inventory  ← 시스템 인벤토리  │
 │  sysops.{hostname}.heartbeat  ← 생존 신호        │
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

## ✨ 기능

### 메트릭 수집

| 카테고리 | 메트릭 | 소스 | 주기 |
|----------|--------|------|------|
| **CPU** | usage%, per-core, per-socket, iowait, steal, load avg | `/proc/stat`, `/proc/loadavg` | 10초 |
| **CPU Topology** | socket별 사용률, NUMA node별 통계 | `/sys/devices/system/node/` | 10초 |
| **Memory** | used%, available, buffers/cached, swap, NUMA per-node | `/proc/meminfo`, `/sys/devices/system/node/*/meminfo` | 10초 |
| **Memory HW** | ECC 에러 count (correctable/uncorrectable) | `/sys/devices/system/edac/mc*/` | 60초 |
| **Disk** | usage%, inode%, I/O rate, latency, SMART health | `/proc/diskstats`, `statvfs()` | 10~60초 |
| **Network** | rx/tx bytes, packets, errors, drops, per-interface | `/proc/net/dev` | 10초 |
| **GPU (NVIDIA)** | utilization%, memory used/total, temperature, power, ECC | NVML / `nvidia-smi` | 10초 |
| **Process** | top-N by CPU/RSS, count, zombie count, GPU process | `/proc/[pid]/stat`, NVML | 30초 |
| **File Descriptors** | system-wide used/max, per-process fd count | `/proc/sys/fs/file-nr` | 30초 |
| **Kernel** | OOM kills, hardware errors, hung tasks, GPU Xid errors | dmesg, journal, syslog | 실시간 |

### 시스템 인벤토리 (자동 수집)

에이전트 시작 시 및 주기적(기본 5분)으로 시스템 하드웨어/소프트웨어 정보를 수집합니다.

| 카테고리 | 수집 항목 | 소스 |
|----------|-----------|------|
| **OS** | distro, version, kernel version, architecture, hostname | `/etc/os-release`, `uname` |
| **CPU** | model name, vendor, sockets, cores/socket, threads/core, MHz, cache sizes, flags (avx, sse), microcode | `/proc/cpuinfo`, `lscpu`, `/sys/devices/system/cpu/` |
| **NUMA** | node count, CPU-to-node mapping, memory per node | `/sys/devices/system/node/` |
| **Memory** | total, DIMM count, DIMM size/type/speed/manufacturer, ECC support | `/proc/meminfo`, `/sys/devices/system/memory/`, `dmidecode` |
| **GPU** | model, VRAM total, driver version, CUDA version, GPU count, PCIe gen/width, power limit | NVML / `nvidia-smi -q` |
| **Disk** | model, serial, capacity, interface (NVMe/SAS/SATA), firmware, SMART status | `/sys/block/*/device/`, `smartctl` |
| **Network** | interface name, MAC, speed, MTU, driver, firmware | `/sys/class/net/*/`, `ethtool` |
| **BIOS/Board** | vendor, version, serial, product name | `/sys/devices/virtual/dmi/id/`, `dmidecode` |

**인벤토리 JSON 예시:**

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

### 이상 탐지 알고리즘

| 알고리즘 | 용도 | 동작 방식 |
|----------|------|-----------|
| **Threshold** | 즉시 위험 감지 | 설정 임계값 초과 시 즉시 알림 |
| **Z-Score** | 통계적 이상 탐지 | 최근 1시간 데이터 기준 3σ 이탈 감지 |
| **EMA** | 급격한 변화 감지 | Exponential Moving Average 대비 편차 |
| **Trend (Linear Regression)** | 리소스 소진 예측 | 24시간 내 디스크 풀, 6시간 내 OOM 예측 |
| **Leak Detection** | FD/메모리 누수 | RSS 단조 증가 + R² > 0.8 패턴 감지 |

### 로그 분석

| 패턴 | Severity | 예시 |
|------|----------|------|
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

## 🔨 빌드

### 요구사항

- Rust 1.75+ (stable)
- Linux 또는 cross-compilation 환경

### 기본 빌드

```bash
cargo build --release
```

### Feature Flags

기본 빌드는 Core 기능만 포함합니다. 추가 기능은 feature flag로 활성화합니다.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Feature Flags                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─ Core (기본 포함) ───────────────────────────────────────────┐  │
│  │  • CPU, Memory, Disk, Network, Process, FD 수집              │  │
│  │  • Threshold, Z-Score, EMA, Trend, Leak 분석                 │  │
│  │  • Discord, Slack, Telegram, Email, Webhook, Syslog 알림     │  │
│  │  • Log Analyzer (dmesg, syslog, journal)                     │  │
│  │  • System Inventory (OS, CPU, Memory, Disk, Network)         │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ gpu ──────────────────────────────────────────────────────┐    │
│  │  NVIDIA GPU 모니터링 (NVML 바인딩)                          │    │
│  │  • GPU utilization, memory, temperature, power, ECC          │    │
│  │  • Per-process GPU 사용량, Xid error 감지                    │    │
│  │  • GPU 인벤토리 (model, VRAM, driver, CUDA version)          │    │
│  │  ⚠️  런타임 요구: NVIDIA driver + libnvidia-ml.so            │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ nats ─────────────────────────────────────────────────────┐    │
│  │  NATS 메시징 지원                                           │    │
│  │  • 메트릭/알림/인벤토리 주기적 publish                       │    │
│  │  • Heartbeat (생존 신호)                                     │    │
│  │  • Subject: sysops.{hostname}.{metrics|alerts|inventory}     │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ prometheus ───────────────────────────────────────────────┐    │
│  │  Prometheus metrics endpoint (:9100/metrics)                │    │
│  │  • 모든 수집 메트릭을 Prometheus 형식으로 노출                │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ sqlite ───────────────────────────────────────────────────┐    │
│  │  장기 메트릭 저장 (SQLite)                                  │    │
│  │  • 1분 평균 다운샘플링, 30일 보존                            │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ TLS (택 1) ───────────────────────────────────────────────┐    │
│  │  tls-rustls   순수 Rust TLS (외부 의존성 없음, 권장)        │    │
│  │  tls-native   OpenSSL 기반 TLS (시스템 CA 인증서 사용)      │    │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**빌드 예시:**

```bash
# 최소 빌드 (Core만, 알림 전용)
cargo build --release

# GPU 서버용
cargo build --release --features "gpu,nats,tls-rustls"

# 전체 기능
cargo build --release --features "gpu,nats,prometheus,sqlite,tls-rustls"

# 모니터링 서버 연동 (NATS + Prometheus)
cargo build --release --features "nats,prometheus,sqlite"
```

### 정적 바이너리 (musl)

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# → glibc 버전 무관, 어디서나 실행
```

### Docker 멀티 OS 빌드

```bash
# 개별 OS
docker build --platform linux/amd64 -f docker/Dockerfile.ubuntu2204 -t sysops-agent:ubuntu2204 .

# 전체 OS 빌드 & 테스트
./scripts/build-test-all.sh
```

### 테스트

```bash
cargo test
cargo test --features "gpu,nats,sqlite" -- --test-threads=1
```

---

## 📦 설치 및 배포

### 방법 1: 바이너리 직접 복사

```bash
# 빌드
cargo build --release --target x86_64-unknown-linux-musl --features "gpu,nats,tls-rustls"

# 배포
scp target/x86_64-unknown-linux-musl/release/sysops-agent user@server:/usr/local/bin/
scp config.toml user@server:/etc/sysops-agent/config.toml
```

### 방법 2: systemd 서비스

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

**systemd unit 파일** (`deploy/sysops-agent.service`):

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
# GPU 접근 필요 시
SupplementaryGroups=video

[Install]
WantedBy=multi-user.target
```

### 방법 3: Ansible

```bash
ansible-playbook -i inventory deploy/ansible/playbook.yml
```

---

## ⚙️ 설정

설정 파일: `/etc/sysops-agent/config.toml`

### 최소 설정

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

### 전체 설정 예시

```toml
[agent]
hostname = "gpu-server-01"
collect_interval_secs = 10
log_level = "info"                    # trace, debug, info, warn, error
data_dir = "/var/lib/sysops-agent"
pid_file = "/var/run/sysops-agent.pid"

# ─── 수집 주기 ───
[collector]
cpu_interval_secs = 10
memory_interval_secs = 10
disk_interval_secs = 60
network_interval_secs = 10
process_interval_secs = 30
fd_interval_secs = 30
gpu_interval_secs = 10                # feature "gpu" 필요

# ─── 시스템 인벤토리 ───
[inventory]
enabled = true
collect_interval_secs = 300           # 5분마다 수집/전송
include_dimm_details = true           # DIMM 상세 정보 (dmidecode, root 필요)
include_smart = false                 # SMART 정보 (smartctl, root 필요)
include_bios = true                   # BIOS/보드 정보

# ─── 임계값 ───
[thresholds]
cpu_percent = 90.0
cpu_per_socket_percent = 95.0         # 소켓별 임계값
memory_percent = 85.0
disk_percent = 90.0
disk_inode_percent = 85.0
fd_percent = 80.0
load_avg_multiplier = 2.0             # load > (CPU cores × multiplier)
network_error_rate = 0.01

# GPU 임계값 (feature "gpu")
gpu_utilization_percent = 95.0
gpu_memory_percent = 90.0
gpu_temperature_celsius = 85.0        # thermal throttling 전 알림
gpu_power_percent = 95.0              # power limit 대비

# ─── 멀티 소켓 / NUMA ───
[cpu]
per_socket_monitoring = true          # 소켓별 분리 모니터링
numa_monitoring = true                # NUMA node별 메모리 통계
ecc_monitoring = true                 # EDAC ECC 에러 카운트

# ─── 분석기 ───
[analyzer]
zscore_window = 360
zscore_threshold = 3.0
ema_alpha = 0.1
trend_window_hours = 6
leak_min_observation_hours = 1
leak_r_squared_threshold = 0.8

# ─── 저장소 ───
[storage]
ring_buffer_capacity = 8640
sqlite_enabled = false                # feature "sqlite" 필요
sqlite_path = "/var/lib/sysops-agent/metrics.db"
sqlite_retention_days = 30

# ─── 로그 분석 ───
[log_analyzer]
enabled = true
sources = ["dmesg", "syslog"]
syslog_path = "/var/log/syslog"
gpu_xid_monitoring = true             # NVIDIA Xid error 감지
custom_patterns = [
    { pattern = "FATAL.*database", severity = "critical", name = "db_fatal" },
    { pattern = "connection refused", severity = "warn", name = "conn_refused" },
]

# ─── NATS 텔레메트리 (feature "nats") ───
[nats]
enabled = true
url = "nats://nats-server:4222"       # NATS 서버 주소
# urls = ["nats://n1:4222", "nats://n2:4222"]  # 클러스터
credential_file = "/etc/sysops-agent/nats.creds"  # 인증 (optional)
# token = "${NATS_TOKEN}"             # 토큰 인증
subject_prefix = "sysops"             # → sysops.{hostname}.*
metrics_interval_secs = 30            # 메트릭 전송 주기
inventory_interval_secs = 300         # 인벤토리 전송 주기
heartbeat_interval_secs = 60          # 생존 신호 주기
include_alerts = true                 # 알림도 NATS로 전송
batch_size = 100                      # 메트릭 배치 크기
compression = true                    # 페이로드 압축 (zstd)

# ─── Prometheus (feature "prometheus") ───
[prometheus]
enabled = false
bind = "127.0.0.1:9100"
path = "/metrics"

# ─── 알림 공통 설정 ───
[alerting]
min_interval_secs = 300
max_alerts_per_hour = 60
dedup_window_secs = 600
emergency_bypass_rate_limit = true
```

---

## 📡 알림 채널 설정

### 채널별 설정 방법

#### 1. 📱 Discord (Webhook)

```
┌─────────────┐      HTTPS POST       ┌──────────────┐
│ SysOps Agent │ ───────────────────▶  │ Discord API  │
│              │  JSON (embeds)        │ /webhooks/   │
└─────────────┘                        └──────┬───────┘
                                              ▼
                                       ┌──────────────┐
                                       │  #alerts 채널 │
                                       └──────────────┘
```

**설정:** Discord 서버 → 채널 설정 → 연동 → Webhook → URL 복사

```toml
[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/1234567890/abcdefgh"
username = "SysOps Agent"
mention_roles = ["@devops"]           # Critical 이상 시 멘션
```

#### 2. 💬 Slack (Webhook)

```
┌─────────────┐      HTTPS POST       ┌──────────────┐
│ SysOps Agent │ ───────────────────▶  │ Slack API    │
│              │  JSON (blocks)        │ /incoming-   │
└─────────────┘                        │  webhooks/   │
                                       └──────────────┘
```

**설정:** Slack App → Incoming Webhooks 활성화 → 채널 선택

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

**설정:** @BotFather → `/newbot` → Token + Chat ID

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
min_severity = "critical"             # Critical 이상만 이메일
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

### Severity 라우팅

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

## 📡 NATS 텔레메트리

NATS 채널은 단방향 알림이 아닌, **주기적 텔레메트리 전송** 용도입니다. 메트릭, 인벤토리, 알림, 하트비트를 NATS subject로 publish하여 중앙 관리 시스템에서 구독합니다.

### Subject 구조

```
sysops.{hostname}.metrics      메트릭 배치 (30초마다)
sysops.{hostname}.alerts       이상 탐지 알림 (발생 시)
sysops.{hostname}.inventory    시스템 인벤토리 (5분마다)
sysops.{hostname}.heartbeat    생존 신호 (1분마다)
```

### 전송 데이터 형식

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
전체 시스템 인벤토리 JSON (위 "시스템 인벤토리" 섹션 참조)

### NATS 설정 예시

```toml
[nats]
enabled = true
url = "nats://nats-server:4222"

# 클러스터 구성
# urls = ["nats://n1:4222", "nats://n2:4222", "nats://n3:4222"]

# 인증
# credential_file = "/etc/sysops-agent/nats.creds"   # NKey 인증
# token = "${NATS_TOKEN}"                             # 토큰 인증
# user = "sysops"                                     # 사용자/비밀번호
# password = "${NATS_PASSWORD}"

# Subject 설정
subject_prefix = "sysops"              # → sysops.{hostname}.*

# 전송 주기
metrics_interval_secs = 30             # 메트릭 (기본 30초)
inventory_interval_secs = 300          # 인벤토리 (기본 5분)
heartbeat_interval_secs = 60           # 하트비트 (기본 1분)

# 최적화
batch_size = 100                       # 메트릭 배치 크기
compression = true                     # zstd 압축 (대역폭 절약)
max_reconnect_attempts = -1            # 무한 재연결
reconnect_delay_secs = 5
```

### 중앙 구독 예시 (Go/Python)

```bash
# nats CLI로 구독 테스트
nats sub "sysops.>"                    # 모든 에이전트
nats sub "sysops.gpu-server-01.>"      # 특정 서버
nats sub "sysops.*.alerts"             # 모든 서버의 알림만
```

---

## 🚀 사용법

### CLI 명령어

```bash
# 기본 실행
sysops-agent --config /etc/sysops-agent/config.toml

# foreground + 디버깅
sysops-agent --config config.toml --log-level debug

# 설정 검증만
sysops-agent --config config.toml --validate

# 인벤토리 1회 출력 (설치 확인용)
sysops-agent --config config.toml --inventory-dump

# 버전/도움말
sysops-agent --version
sysops-agent --help
```

### systemd 서비스 관리

```bash
sudo systemctl start sysops-agent
sudo systemctl status sysops-agent
journalctl -u sysops-agent -f
```

### Prometheus 연동 (optional)

```bash
curl http://localhost:9100/metrics
```

---

## 📚 문서

| 문서 | 설명 |
|------|------|
| [DESIGN.md](docs/DESIGN.md) | 아키텍처 및 상세 설계 (알고리즘, 보안 모델, 성능) |
| [METRICS.md](docs/METRICS.md) | 수집 메트릭 카탈로그 (80+ 메트릭) |
| [ALERTING.md](docs/ALERTING.md) | 알림 시스템 상세 설계 |
| [DEPLOYMENT.md](docs/DEPLOYMENT.md) | 배포 가이드 (systemd, RPM, DEB, Ansible) |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | 전체 설정 레퍼런스 |
| [BUILD-TEST-RESULTS.md](docs/BUILD-TEST-RESULTS.md) | OS별 빌드/테스트 결과 |

---

## 🤝 Contributing

1. Fork → Branch → PR
2. `cargo fmt && cargo clippy` 통과 필수
3. 새 메트릭 추가 시 METRICS.md 업데이트

## 📄 라이선스

MIT License — [LICENSE](LICENSE)
