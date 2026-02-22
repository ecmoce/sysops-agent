# SysOps Agent

> 경량 시스템 모니터링 에이전트 — Rust로 작성된 보안 중심의 Linux 서버 모니터링 데몬

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 개요

SysOps Agent는 Linux 서버에서 실행되는 데몬 스타일의 시스템 모니터링 에이전트입니다. 실시간 이상 탐지, 트렌드 기반 분석, 리소스 누수 감지, 커널/시스템 로그 분석을 수행하며, 다양한 채널(Discord, Slack, Telegram, Email, Webhook)을 통해 알림을 전송합니다.

### 핵심 특징

- **단일 정적 바이너리** — 런타임 의존성 없음, 간편한 배포
- **최소 리소스 사용** — RSS <50MB, 유휴 시 CPU <1%
- **root 불필요** — Linux capabilities 기반 권한 관리
- **보안 최우선** — 기본적으로 수신 포트 없음 (push-only)
- **TOML 설정** — 직관적인 설정 파일

### 지원 배포판

CentOS 7/8/9 · Rocky Linux 8/9 · Ubuntu 20.04/22.04/24.04

## 아키텍처

```
┌─────────────────────────────────────────────────────────┐
│                     SysOps Agent                        │
│                                                         │
│  ┌───────────┐    ┌───────────┐    ┌───────────┐       │
│  │ Collector  │───▶│ Analyzer  │───▶│  Alerter  │──────▶│ Discord
│  │           │    │           │    │           │       │ Slack
│  │ • CPU     │    │ • Z-Score │    │ • Rate    │       │ Telegram
│  │ • Memory  │    │ • Moving  │    │   Limit   │       │ Email
│  │ • Disk    │    │   Average │    │ • Dedup   │       │ Webhook
│  │ • Network │    │ • Trend   │    │ • Group   │       │
│  │ • Process │    │ • Leak    │    │           │       │
│  │ • Kernel  │    │   Detect  │    │           │       │
│  └─────┬─────┘    └─────┬─────┘    └───────────┘       │
│        │                │                               │
│        ▼                ▼                               │
│  ┌─────────────────────────────┐                       │
│  │   Storage (Ring Buffer)     │                       │
│  │   + Optional SQLite         │                       │
│  └─────────────────────────────┘                       │
│                                                         │
│  ┌───────────────────────────────────────┐             │
│  │  Log Analyzer                         │             │
│  │  • dmesg • journal • syslog           │             │
│  └───────────────────────────────────────┘             │
│                                                         │
│  ┌───────────────┐  ┌──────────────────┐               │
│  │ Config (TOML) │  │ Prometheus (opt) │               │
│  └───────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────────┘
```

## 빠른 시작

### 빌드

```bash
# 정적 바이너리 빌드 (musl)
cargo build --release --target x86_64-unknown-linux-musl

# 모든 feature 포함 빌드
cargo build --release --features "prometheus,sqlite"
```

### 설정

```toml
# /etc/sysops-agent/config.toml

[agent]
hostname = "web-server-01"
collect_interval_secs = 10
log_level = "info"

[thresholds]
cpu_percent = 90.0
memory_percent = 85.0
disk_percent = 90.0
fd_percent = 80.0

[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/..."

[alerting.rate_limit]
min_interval_secs = 300
max_alerts_per_hour = 20
```

### 실행

```bash
# 직접 실행
sysops-agent --config /etc/sysops-agent/config.toml

# systemd 서비스
sudo systemctl enable --now sysops-agent
```

## 문서

| 문서 | 설명 |
|------|------|
| [DESIGN.md](docs/DESIGN.md) | 아키텍처 및 설계 문서 |
| [METRICS.md](docs/METRICS.md) | 수집 메트릭 카탈로그 |
| [ALERTING.md](docs/ALERTING.md) | 알림 시스템 설계 |
| [DEPLOYMENT.md](docs/DEPLOYMENT.md) | 배포 가이드 |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | 설정 레퍼런스 |

## 라이선스

MIT License
