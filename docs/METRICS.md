# Metrics Catalog

Detailed specifications of all collected metrics.

## Notation Convention

- **Type**: `gauge` (current value), `counter` (cumulative value, converted to rate)
- **Interval**: Default collection period (changeable via configuration)
- **Detection**: Applied anomaly detection methods

## CPU

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `cpu.usage_percent` | gauge | `/proc/stat` (delta calculation) | 10s | warn: 80%, crit: 95% | threshold, z-score |
| `cpu.usage_per_core` | gauge | `/proc/stat` cpuN lines | 10s | crit: 100% (1 core) | threshold |
| `cpu.iowait_percent` | gauge | `/proc/stat` iowait field | 10s | warn: 30%, crit: 60% | threshold, z-score |
| `cpu.steal_percent` | gauge | `/proc/stat` steal field | 10s | warn: 10%, crit: 30% | threshold |
| `cpu.load_1m` | gauge | `/proc/loadavg` | 10s | warn: nproc×2, crit: nproc×4 | threshold, trend |
| `cpu.load_5m` | gauge | `/proc/loadavg` | 10s | — | trend |
| `cpu.load_15m` | gauge | `/proc/loadavg` | 10s | — | trend |
| `cpu.context_switches` | counter | `/proc/stat` ctxt | 10s | — | z-score |
| `cpu.interrupts` | counter | `/proc/stat` intr | 10s | — | z-score |

### CPU Calculation Method

Read user, nice, system, idle, iowait, irq, softirq, steal values from the `cpu` line in `/proc/stat`, and calculate delta with previous sample:

```
usage% = 100 × (1 - (delta_idle + delta_iowait) / delta_total)
```

## Memory

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `mem.usage_percent` | gauge | `/proc/meminfo` | 10s | warn: 80%, crit: 90% | threshold, trend |
| `mem.available_bytes` | gauge | `/proc/meminfo` MemAvailable | 10s | crit: <256MB | threshold |
| `mem.used_bytes` | gauge | calculation: Total - Available | 10s | — | trend |
| `mem.buffers_bytes` | gauge | `/proc/meminfo` Buffers | 10s | — | — |
| `mem.cached_bytes` | gauge | `/proc/meminfo` Cached | 10s | — | — |
| `mem.swap_usage_percent` | gauge | `/proc/meminfo` SwapTotal/Free | 10s | warn: 50%, crit: 80% | threshold |
| `mem.swap_in_rate` | counter | `/proc/vmstat` pswpin | 10s | — | z-score |
| `mem.swap_out_rate` | counter | `/proc/vmstat` pswpout | 10s | — | z-score |
| `mem.oom_score_adj` | gauge | `/proc/[pid]/oom_score_adj` | 60s | — | — |

### Memory Calculation Method

For older kernels without `MemAvailable` (some CentOS 7), fallback calculation:
```
available = MemFree + Buffers + Cached - Shmem
```

## Disk

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `disk.usage_percent` | gauge | `statvfs()` on mountpoints | 60s | warn: 80%, crit: 90% | threshold, trend |
| `disk.available_bytes` | gauge | `statvfs()` | 60s | crit: <1GB | threshold, trend |
| `disk.inode_usage_percent` | gauge | `statvfs()` | 60s | warn: 80%, crit: 95% | threshold |
| `disk.read_bytes_rate` | counter | `/proc/diskstats` field 6 | 10s | — | z-score |
| `disk.write_bytes_rate` | counter | `/proc/diskstats` field 10 | 10s | — | z-score |
| `disk.read_ops_rate` | counter | `/proc/diskstats` field 4 | 10s | — | z-score |
| `disk.write_ops_rate` | counter | `/proc/diskstats` field 8 | 10s | — | z-score |
| `disk.io_time_percent` | counter | `/proc/diskstats` field 13 | 10s | warn: 80%, crit: 95% | threshold |
| `disk.await_ms` | gauge | calculation: io_time / ops | 10s | warn: 100ms, crit: 500ms | threshold, z-score |

### Labels
- `device`: sda, nvme0n1, etc.
- `mountpoint`: /, /home, /var, etc.
- `fstype`: ext4, xfs, tmpfs, etc. (option to exclude tmpfs)

### Trend Analysis

Apply linear regression to disk usage to predict capacity depletion time:
- Predicted depletion < 24 hours: Critical
- Predicted depletion < 72 hours: Warn

## Network

| Metric | Type | Source | Interval | Default Threshold | Detection |
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
| `net.tcp_time_wait` | gauge | `/proc/net/tcp` state filter | 30s | warn: 10000 | threshold |
| `net.tcp_retransmits` | counter | `/proc/net/snmp` | 10s | — | z-score |

### Labels
- `interface`: eth0, ens192, etc. (lo excluded by default, configurable)

## Process

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `proc.count` | gauge | `/proc/` directory | 30s | warn: 1000 | threshold |
| `proc.rss_bytes` | gauge | `/proc/[pid]/status` VmRSS | 30s | — | leak detection |
| `proc.cpu_percent` | gauge | `/proc/[pid]/stat` | 30s | warn: 80% (per process) | threshold |
| `proc.thread_count` | gauge | `/proc/[pid]/status` Threads | 30s | — | leak detection |
| `proc.fd_count` | gauge | `/proc/[pid]/fd/` readdir | 30s | — | leak detection |
| `proc.voluntary_ctxt_switches` | counter | `/proc/[pid]/status` | 30s | — | — |

### Target Process Selection

Tracking all processes wastes resources. Filter in configuration:

```toml
[collector.process]
# Filter by name patterns
track_patterns = ["nginx", "java", "python", "node", "postgres"]
# Or top-N (by RSS)
track_top_n = 20
```

## File Descriptors

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `fd.system_used` | gauge | `/proc/sys/fs/file-nr` field 1 | 30s | — | trend |
| `fd.system_max` | gauge | `/proc/sys/fs/file-nr` field 3 | 30s | — | — |
| `fd.system_usage_percent` | gauge | calculation | 30s | warn: 80%, crit: 95% | threshold |

## Kernel

| Metric | Type | Source | Interval | Default Threshold | Detection |
|--------|------|--------|----------|-------------------|-----------|
| `kernel.entropy_available` | gauge | `/proc/sys/kernel/random/entropy_avail` | 60s | warn: <200 | threshold |
| `kernel.uptime_secs` | gauge | `/proc/uptime` | 60s | — | — |
| `kernel.oom_kills` | counter | dmesg pattern matching | event | warn: >0 | event |
| `kernel.hung_tasks` | counter | dmesg pattern matching | event | crit: >0 | event |
| `kernel.hardware_errors` | counter | dmesg pattern matching | event | crit: >0 | event |
| `kernel.fs_errors` | counter | dmesg pattern matching | event | crit: >0 | event |