# 설정 레퍼런스

SysOps Agent는 TOML 형식의 설정 파일을 사용합니다. 환경 변수 참조는 `${ENV_VAR}` 문법을 지원합니다.

## 전체 설정 옵션

### `[agent]` — 에이전트 기본 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `hostname` | string | 시스템 hostname | 알림에 표시될 호스트 이름 |
| `log_level` | string | `"info"` | 로그 레벨: trace, debug, info, warn, error |
| `log_file` | string | — | 로그 파일 경로 (미설정 시 stderr) |
| `pid_file` | string | — | PID 파일 경로 |
| `data_dir` | string | `"/var/lib/sysops-agent"` | 데이터 저장 디렉토리 |
| `proc_root` | string | `"/proc"` | procfs 마운트 경로 |
| `sys_root` | string | `"/sys"` | sysfs 마운트 경로 |

### `[collector]` — 수집기 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `default_interval_secs` | u64 | `10` | 기본 수집 주기 (초) |
| `enabled_collectors` | string[] | 전체 | 활성화할 collector 목록 |

### `[collector.cpu]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | CPU 수집 활성화 |
| `interval_secs` | u64 | `10` | 수집 주기 |
| `per_core` | bool | `true` | 코어별 수집 |

### `[collector.memory]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | 메모리 수집 활성화 |
| `interval_secs` | u64 | `10` | 수집 주기 |

### `[collector.disk]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | 디스크 수집 활성화 |
| `interval_secs` | u64 | `60` | 사용량 수집 주기 |
| `io_interval_secs` | u64 | `10` | I/O 통계 수집 주기 |
| `exclude_fstypes` | string[] | `["tmpfs", "devtmpfs", "sysfs", "proc"]` | 제외할 파일시스템 타입 |
| `exclude_mountpoints` | string[] | `[]` | 제외할 마운트 포인트 |

### `[collector.network]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | 네트워크 수집 활성화 |
| `interval_secs` | u64 | `10` | 수집 주기 |
| `exclude_interfaces` | string[] | `["lo"]` | 제외할 인터페이스 |

### `[collector.process]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | 프로세스 수집 활성화 |
| `interval_secs` | u64 | `30` | 수집 주기 |
| `track_patterns` | string[] | `[]` | 추적할 프로세스 이름 패턴 |
| `track_top_n` | u32 | `20` | RSS 기준 상위 N개 추적 |

### `[collector.log]`

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `true` | 로그 분석 활성화 |
| `sources` | string[] | `["dmesg", "syslog"]` | 분석 소스 |
| `syslog_path` | string | 자동 감지 | syslog 파일 경로 |
| `custom_patterns` | table[] | `[]` | 커스텀 패턴 목록 |

```toml
[[collector.log.custom_patterns]]
name = "app_error"
pattern = "MyApp.*FATAL"
severity = "critical"
```

### `[thresholds]` — 임계값 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `cpu_warn_percent` | f64 | `80.0` | CPU 경고 임계값 |
| `cpu_critical_percent` | f64 | `95.0` | CPU 위험 임계값 |
| `memory_warn_percent` | f64 | `80.0` | 메모리 경고 임계값 |
| `memory_critical_percent` | f64 | `90.0` | 메모리 위험 임계값 |
| `disk_warn_percent` | f64 | `80.0` | 디스크 경고 임계값 |
| `disk_critical_percent` | f64 | `90.0` | 디스크 위험 임계값 |
| `fd_warn_percent` | f64 | `80.0` | FD 경고 임계값 |
| `fd_critical_percent` | f64 | `95.0` | FD 위험 임계값 |

### `[analyzer]` — 분석기 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `zscore_window` | u32 | `360` | Z-Score 윈도우 크기 (샘플 수) |
| `zscore_threshold` | f64 | `3.0` | Z-Score 이상 판정 기준 |
| `ema_alpha` | f64 | `0.1` | EMA smoothing factor |
| `trend_window_hours` | u32 | `6` | 트렌드 분석 윈도우 (시간) |
| `leak_min_observation_mins` | u32 | `30` | 누수 감지 최소 관찰 시간 |
| `leak_r_squared_threshold` | f64 | `0.8` | 누수 판정 R² 최소값 |

### `[storage]` — 저장 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `ring_buffer_size` | u32 | `8640` | 메트릭당 ring buffer 크기 |
| `sqlite_enabled` | bool | `false` | SQLite 영속 저장 활성화 (feature 필요) |
| `sqlite_path` | string | `"data_dir/metrics.db"` | SQLite DB 경로 |
| `sqlite_retention_days` | u32 | `30` | SQLite 보존 기간 |

### `[alerting]` — 알림 공통 설정

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `rate_limit_per_minute` | u32 | `10` | 채널당 분당 최대 알림 수 |
| `rate_limit_per_hour` | u32 | `60` | 채널당 시간당 최대 알림 수 |
| `dedup_window_secs` | u64 | `300` | 중복 억제 윈도우 (초) |
| `group_window_secs` | u64 | `30` | 알림 그룹핑 윈도우 (초) |
| `recovery_enabled` | bool | `true` | 복구 알림 활성화 |

### `[prometheus]` — Prometheus Endpoint (feature 필요)

| 키 | 타입 | 기본값 | 설명 |
|----|------|--------|------|
| `enabled` | bool | `false` | Prometheus endpoint 활성화 |
| `bind` | string | `"127.0.0.1:9100"` | 바인딩 주소 |

---

## 예제 설정: 최소

```toml
[agent]
hostname = "my-server"

[thresholds]
cpu_critical_percent = 95.0
memory_critical_percent = 90.0
disk_critical_percent = 90.0

[alerting.discord]
enabled = true
webhook_url = "${DISCORD_WEBHOOK_URL}"
```

## 예제 설정: 전체

```toml
[agent]
hostname = "web-prod-01"
log_level = "info"
log_file = "/var/log/sysops-agent/agent.log"
pid_file = "/var/run/sysops-agent.pid"
data_dir = "/var/lib/sysops-agent"

[collector]
default_interval_secs = 10

[collector.cpu]
enabled = true
interval_secs = 10
per_core = true

[collector.memory]
enabled = true
interval_secs = 10

[collector.disk]
enabled = true
interval_secs = 60
io_interval_secs = 10
exclude_fstypes = ["tmpfs", "devtmpfs", "sysfs", "proc", "cgroup2"]
exclude_mountpoints = ["/boot/efi"]

[collector.network]
enabled = true
interval_secs = 10
exclude_interfaces = ["lo", "docker0"]

[collector.process]
enabled = true
interval_secs = 30
track_patterns = ["nginx", "java", "python3", "postgres", "redis"]
track_top_n = 20

[collector.log]
enabled = true
sources = ["dmesg", "syslog"]

[[collector.log.custom_patterns]]
name = "nginx_502"
pattern = "upstream.*502"
severity = "warn"

[thresholds]
cpu_warn_percent = 80.0
cpu_critical_percent = 95.0
memory_warn_percent = 80.0
memory_critical_percent = 90.0
disk_warn_percent = 80.0
disk_critical_percent = 90.0
fd_warn_percent = 80.0
fd_critical_percent = 95.0

[analyzer]
zscore_window = 360
zscore_threshold = 3.0
ema_alpha = 0.1
trend_window_hours = 6
leak_min_observation_mins = 30

[storage]
ring_buffer_size = 8640
sqlite_enabled = true
sqlite_retention_days = 30

[alerting]
rate_limit_per_minute = 10
rate_limit_per_hour = 60
dedup_window_secs = 300
group_window_secs = 30
recovery_enabled = true

[alerting.discord]
enabled = true
webhook_url = "${DISCORD_WEBHOOK_URL}"
severity_filter = ["warn", "critical", "emergency"]

[alerting.slack]
enabled = true
webhook_url = "${SLACK_WEBHOOK_URL}"
channel = "#infra-alerts"
severity_filter = ["critical", "emergency"]

[alerting.telegram]
enabled = false

[alerting.email]
enabled = true
smtp_host = "smtp.gmail.com"
smtp_port = 587
smtp_tls = true
username = "${SMTP_USER}"
password = "${SMTP_PASSWORD}"
from = "sysops@example.com"
to = ["oncall@example.com"]
severity_filter = ["emergency"]

[prometheus]
enabled = true
bind = "127.0.0.1:9100"
```

## 예제 설정: 고보안

```toml
[agent]
hostname = "secure-server"
log_level = "warn"
data_dir = "/var/lib/sysops-agent"

# 최소 수집만 활성화
[collector.cpu]
enabled = true
per_core = false

[collector.memory]
enabled = true

[collector.disk]
enabled = true

[collector.network]
enabled = false  # 네트워크 메트릭 불필요 시 비활성

[collector.process]
enabled = false  # 프로세스 추적 비활성

[collector.log]
enabled = true
sources = ["dmesg"]  # syslog 접근 불필요 시

[thresholds]
cpu_critical_percent = 90.0
memory_critical_percent = 85.0
disk_critical_percent = 85.0

[storage]
sqlite_enabled = false  # 디스크에 데이터 저장 안 함

[alerting]
rate_limit_per_minute = 5

# 외부 연결 최소화: syslog만 사용
[alerting.syslog]
enabled = true
severity_filter = ["warn", "critical", "emergency"]

# Prometheus endpoint 비활성 (수신 포트 없음)
[prometheus]
enabled = false
```
