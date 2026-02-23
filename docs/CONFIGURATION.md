# Configuration Reference

SysOps Agent uses TOML format configuration files. Environment variable references support `${ENV_VAR}` syntax.

## All Configuration Options

### `[agent]` — Agent Basic Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `hostname` | string | system hostname | Host name to display in alerts |
| `log_level` | string | `"info"` | Log level: trace, debug, info, warn, error |
| `log_file` | string | — | Log file path (stderr if not set) |
| `pid_file` | string | — | PID file path |
| `data_dir` | string | `"/var/lib/sysops-agent"` | Data storage directory |
| `proc_root` | string | `"/proc"` | procfs mount path |
| `sys_root` | string | `"/sys"` | sysfs mount path |

### `[collector]` — Collector Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_interval_secs` | u64 | `10` | Default collection interval (seconds) |
| `enabled_collectors` | string[] | all | List of collectors to enable |

### `[collector.cpu]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable CPU collection |
| `interval_secs` | u64 | `10` | Collection interval |
| `per_core` | bool | `true` | Per-core collection |

### `[collector.memory]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable memory collection |
| `interval_secs` | u64 | `10` | Collection interval |

### `[collector.disk]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable disk collection |
| `interval_secs` | u64 | `60` | Usage collection interval |
| `io_interval_secs` | u64 | `10` | I/O statistics collection interval |
| `exclude_fstypes` | string[] | `["tmpfs", "devtmpfs", "sysfs", "proc"]` | Filesystem types to exclude |
| `exclude_mountpoints` | string[] | `[]` | Mount points to exclude |

### `[collector.network]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable network collection |
| `interval_secs` | u64 | `10` | Collection interval |
| `exclude_interfaces` | string[] | `["lo"]` | Interfaces to exclude |

### `[collector.process]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable process collection |
| `interval_secs` | u64 | `30` | Collection interval |
| `track_patterns` | string[] | `[]` | Process name patterns to track |
| `track_top_n` | u32 | `20` | Track top N by RSS |

### `[collector.log]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable log analysis |
| `sources` | string[] | `["dmesg", "syslog"]` | Analysis sources |
| `syslog_path` | string | auto-detect | syslog file path |
| `custom_patterns` | table[] | `[]` | Custom pattern list |

```toml
[[collector.log.custom_patterns]]
name = "app_error"
pattern = "MyApp.*FATAL"
severity = "critical"
```

### `[thresholds]` — Threshold Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cpu_warn_percent` | f64 | `80.0` | CPU warning threshold |
| `cpu_critical_percent` | f64 | `95.0` | CPU critical threshold |
| `memory_warn_percent` | f64 | `80.0` | Memory warning threshold |
| `memory_critical_percent` | f64 | `90.0` | Memory critical threshold |
| `disk_warn_percent` | f64 | `80.0` | Disk warning threshold |
| `disk_critical_percent` | f64 | `90.0` | Disk critical threshold |
| `fd_warn_percent` | f64 | `80.0` | FD warning threshold |
| `fd_critical_percent` | f64 | `95.0` | FD critical threshold |

### `[analyzer]` — Analyzer Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `zscore_window` | u32 | `360` | Z-Score window size (sample count) |
| `zscore_threshold` | f64 | `3.0` | Z-Score anomaly detection threshold |
| `ema_alpha` | f64 | `0.1` | EMA smoothing factor |
| `trend_window_hours` | u32 | `6` | Trend analysis window (hours) |
| `leak_min_observation_mins` | u32 | `30` | Leak detection minimum observation time |
| `leak_r_squared_threshold` | f64 | `0.8` | Leak detection R² minimum value |

### `[storage]` — Storage Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `ring_buffer_size` | u32 | `8640` | Ring buffer size per metric |
| `sqlite_enabled` | bool | `false` | Enable SQLite persistent storage (requires feature) |
| `sqlite_path` | string | `"data_dir/metrics.db"` | SQLite DB path |
| `sqlite_retention_days` | u32 | `30` | SQLite retention period |

### `[alerting]` — Common Alert Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `rate_limit_per_minute` | u32 | `10` | Max alerts per channel per minute |
| `rate_limit_per_hour` | u32 | `60` | Max alerts per channel per hour |
| `dedup_window_secs` | u64 | `300` | Deduplication window (seconds) |
| `group_window_secs` | u64 | `30` | Alert grouping window (seconds) |
| `recovery_enabled` | bool | `true` | Enable recovery alerts |

### `[prometheus]` — Prometheus Endpoint (requires feature)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `false` | Enable Prometheus endpoint |
| `bind` | string | `"127.0.0.1:9100"` | Bind address |

---

## Example Configuration: Minimal

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

## Example Configuration: Full

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

## Example Configuration: High Security

```toml
[agent]
hostname = "secure-server"
log_level = "warn"
data_dir = "/var/lib/sysops-agent"

# Enable minimal collection only
[collector.cpu]
enabled = true
per_core = false

[collector.memory]
enabled = true

[collector.disk]
enabled = true

[collector.network]
enabled = false  # Disable if network metrics unnecessary

[collector.process]
enabled = false  # Disable process tracking

[collector.log]
enabled = true
sources = ["dmesg"]  # If syslog access unnecessary

[thresholds]
cpu_critical_percent = 90.0
memory_critical_percent = 85.0
disk_critical_percent = 85.0

[storage]
sqlite_enabled = false  # No data storage on disk

[alerting]
rate_limit_per_minute = 5

# Minimize external connections: use syslog only
[alerting.syslog]
enabled = true
severity_filter = ["warn", "critical", "emergency"]

# Disable Prometheus endpoint (no listening ports)
[prometheus]
enabled = false
```