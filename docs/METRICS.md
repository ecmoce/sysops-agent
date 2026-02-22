# 메트릭 카탈로그

수집되는 모든 메트릭의 상세 사양입니다.

## 표기 규칙

- **Type**: `gauge` (현재 값), `counter` (누적 값, rate로 변환)
- **Interval**: 기본 수집 주기 (설정으로 변경 가능)
- **Detection**: 적용되는 이상 탐지 방법

## CPU

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `cpu.usage_percent` | gauge | `/proc/stat` (delta 계산) | 10s | warn: 80%, crit: 95% | threshold, z-score |
| `cpu.usage_per_core` | gauge | `/proc/stat` cpuN lines | 10s | crit: 100% (1개 코어) | threshold |
| `cpu.iowait_percent` | gauge | `/proc/stat` iowait field | 10s | warn: 30%, crit: 60% | threshold, z-score |
| `cpu.steal_percent` | gauge | `/proc/stat` steal field | 10s | warn: 10%, crit: 30% | threshold |
| `cpu.load_1m` | gauge | `/proc/loadavg` | 10s | warn: nproc×2, crit: nproc×4 | threshold, trend |
| `cpu.load_5m` | gauge | `/proc/loadavg` | 10s | — | trend |
| `cpu.load_15m` | gauge | `/proc/loadavg` | 10s | — | trend |
| `cpu.context_switches` | counter | `/proc/stat` ctxt | 10s | — | z-score |
| `cpu.interrupts` | counter | `/proc/stat` intr | 10s | — | z-score |

### CPU 계산 방법

`/proc/stat`의 `cpu` 라인에서 user, nice, system, idle, iowait, irq, softirq, steal 값을 읽고, 이전 샘플과의 delta를 계산합니다:

```
usage% = 100 × (1 - (delta_idle + delta_iowait) / delta_total)
```

## Memory

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `mem.usage_percent` | gauge | `/proc/meminfo` | 10s | warn: 80%, crit: 90% | threshold, trend |
| `mem.available_bytes` | gauge | `/proc/meminfo` MemAvailable | 10s | crit: <256MB | threshold |
| `mem.used_bytes` | gauge | 계산: Total - Available | 10s | — | trend |
| `mem.buffers_bytes` | gauge | `/proc/meminfo` Buffers | 10s | — | — |
| `mem.cached_bytes` | gauge | `/proc/meminfo` Cached | 10s | — | — |
| `mem.swap_usage_percent` | gauge | `/proc/meminfo` SwapTotal/Free | 10s | warn: 50%, crit: 80% | threshold |
| `mem.swap_in_rate` | counter | `/proc/vmstat` pswpin | 10s | — | z-score |
| `mem.swap_out_rate` | counter | `/proc/vmstat` pswpout | 10s | — | z-score |
| `mem.oom_score_adj` | gauge | `/proc/[pid]/oom_score_adj` | 60s | — | — |

### Memory 계산 방법

`MemAvailable`이 없는 오래된 커널(CentOS 7 일부)에서는 fallback 계산:
```
available = MemFree + Buffers + Cached - Shmem
```

## Disk

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `disk.usage_percent` | gauge | `statvfs()` on mountpoints | 60s | warn: 80%, crit: 90% | threshold, trend |
| `disk.available_bytes` | gauge | `statvfs()` | 60s | crit: <1GB | threshold, trend |
| `disk.inode_usage_percent` | gauge | `statvfs()` | 60s | warn: 80%, crit: 95% | threshold |
| `disk.read_bytes_rate` | counter | `/proc/diskstats` field 6 | 10s | — | z-score |
| `disk.write_bytes_rate` | counter | `/proc/diskstats` field 10 | 10s | — | z-score |
| `disk.read_ops_rate` | counter | `/proc/diskstats` field 4 | 10s | — | z-score |
| `disk.write_ops_rate` | counter | `/proc/diskstats` field 8 | 10s | — | z-score |
| `disk.io_time_percent` | counter | `/proc/diskstats` field 13 | 10s | warn: 80%, crit: 95% | threshold |
| `disk.await_ms` | gauge | 계산: io_time / ops | 10s | warn: 100ms, crit: 500ms | threshold, z-score |

### Labels
- `device`: sda, nvme0n1 등
- `mountpoint`: /, /home, /var 등
- `fstype`: ext4, xfs, tmpfs 등 (tmpfs 제외 옵션 제공)

### Trend Analysis

디스크 사용량에 대해 선형 회귀를 적용하여 용량 소진 시점을 예측합니다:
- 예측 소진 시점 < 24시간: Critical
- 예측 소진 시점 < 72시간: Warn

## Network

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `net.rx_bytes_rate` | counter | `/proc/net/dev` | 10s | — | z-score |
| `net.tx_bytes_rate` | counter | `/proc/net/dev` | 10s | — | z-score |
| `net.rx_packets_rate` | counter | `/proc/net/dev` | 10s | — | z-score |
| `net.tx_packets_rate` | counter | `/proc/net/dev` | 10s | — | z-score |
| `net.rx_errors_rate` | counter | `/proc/net/dev` | 10s | warn: >0 | threshold |
| `net.tx_errors_rate` | counter | `/proc/net/dev` | 10s | warn: >0 | threshold |
| `net.rx_drops_rate` | counter | `/proc/net/dev` | 10s | warn: >0 | threshold |
| `net.tx_drops_rate` | counter | `/proc/net/dev` | 10s | warn: >0 | threshold |
| `net.tcp_connections` | gauge | `/proc/net/tcp` line count | 30s | — | z-score |
| `net.tcp_time_wait` | gauge | `/proc/net/tcp` state 필터 | 30s | warn: 10000 | threshold |
| `net.tcp_retransmits` | counter | `/proc/net/snmp` | 10s | — | z-score |

### Labels
- `interface`: eth0, ens192 등 (lo 제외 기본, 설정 가능)

## Process

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `proc.count` | gauge | `/proc/` 디렉토리 | 30s | warn: 1000 | threshold |
| `proc.rss_bytes` | gauge | `/proc/[pid]/status` VmRSS | 30s | — | leak detection |
| `proc.cpu_percent` | gauge | `/proc/[pid]/stat` | 30s | warn: 80% (per process) | threshold |
| `proc.thread_count` | gauge | `/proc/[pid]/status` Threads | 30s | — | leak detection |
| `proc.fd_count` | gauge | `/proc/[pid]/fd/` readdir | 30s | — | leak detection |
| `proc.voluntary_ctxt_switches` | counter | `/proc/[pid]/status` | 30s | — | — |

### 대상 프로세스 선택

모든 프로세스를 추적하면 리소스를 낭비합니다. 설정에서 필터링:

```toml
[collector.process]
# 이름 패턴으로 필터
track_patterns = ["nginx", "java", "python", "node", "postgres"]
# 또는 top-N (RSS 기준)
track_top_n = 20
```

## File Descriptors

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `fd.system_used` | gauge | `/proc/sys/fs/file-nr` field 1 | 30s | — | trend |
| `fd.system_max` | gauge | `/proc/sys/fs/file-nr` field 3 | 30s | — | — |
| `fd.system_usage_percent` | gauge | 계산 | 30s | warn: 80%, crit: 95% | threshold |

## Kernel

| 메트릭 | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `kernel.entropy_available` | gauge | `/proc/sys/kernel/random/entropy_avail` | 60s | warn: <200 | threshold |
| `kernel.uptime_secs` | gauge | `/proc/uptime` | 60s | — | — |
| `kernel.oom_kills` | counter | dmesg 패턴 매칭 | event | warn: >0 | event |
| `kernel.hung_tasks` | counter | dmesg 패턴 매칭 | event | crit: >0 | event |
| `kernel.hardware_errors` | counter | dmesg 패턴 매칭 | event | crit: >0 | event |
| `kernel.fs_errors` | counter | dmesg 패턴 매칭 | event | crit: >0 | event |
