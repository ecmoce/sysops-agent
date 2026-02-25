pub mod collector;
pub mod analyzer;
pub mod alerter;
pub mod config;
pub mod storage;
pub mod log_analyzer;
#[cfg(feature = "nats")]
pub mod nats_publisher;
#[cfg(feature = "nats")]
pub mod inventory;
#[cfg(feature = "nats")]
pub mod nats_handlers;

/// Common types used across modules
pub mod types {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use smallvec::SmallVec;

    /// A single metric measurement
    #[derive(Debug, Clone, Serialize)]
    pub struct MetricSample {
        pub timestamp: DateTime<Utc>,
        pub metric: MetricId,
        pub value: f64,
        pub labels: SmallVec<[Label; 4]>,
    }

    /// Metric identifier
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum MetricId {
        CpuUsage,
        CpuUsagePerCore,
        CpuIoWait,
        CpuSteal,
        CpuLoad1m,
        CpuLoad5m,
        CpuLoad15m,
        MemUsage,
        MemAvailable,
        MemSwapUsage,
        DiskUsage,
        DiskAvailable,
        DiskInodeUsage,
        DiskReadRate,
        DiskWriteRate,
        DiskIoTime,
        NetRxRate,
        NetTxRate,
        NetRxErrors,
        NetTxErrors,
        ProcCount,
        ProcRss,
        ProcCpu,
        ProcFdCount,
        FdSystemUsage,
        KernelEntropy,
        KernelUptime,
    }

    /// A label key-value pair
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Label {
        pub key: String,
        pub value: String,
    }

    /// Alert severity levels
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum Severity {
        Info,
        Warn,
        Critical,
        Emergency,
    }

    /// An alert to be dispatched
    #[derive(Debug, Clone, Serialize)]
    pub struct Alert {
        pub timestamp: DateTime<Utc>,
        pub severity: Severity,
        pub metric: MetricId,
        pub value: f64,
        pub threshold: Option<f64>,
        pub message: String,
        pub labels: SmallVec<[Label; 4]>,
        pub hostname: String,
    }

    impl std::fmt::Display for Severity {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Severity::Info => write!(f, "INFO"),
                Severity::Warn => write!(f, "WARN"),
                Severity::Critical => write!(f, "CRITICAL"),
                Severity::Emergency => write!(f, "EMERGENCY"),
            }
        }
    }

    impl std::fmt::Display for MetricId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                MetricId::CpuUsage => "cpu.usage_percent",
                MetricId::CpuUsagePerCore => "cpu.usage_per_core",
                MetricId::CpuIoWait => "cpu.iowait_percent",
                MetricId::CpuSteal => "cpu.steal_percent",
                MetricId::CpuLoad1m => "cpu.load_1m",
                MetricId::CpuLoad5m => "cpu.load_5m",
                MetricId::CpuLoad15m => "cpu.load_15m",
                MetricId::MemUsage => "mem.usage_percent",
                MetricId::MemAvailable => "mem.available_bytes",
                MetricId::MemSwapUsage => "mem.swap_usage_percent",
                MetricId::DiskUsage => "disk.usage_percent",
                MetricId::DiskAvailable => "disk.available_bytes",
                MetricId::DiskInodeUsage => "disk.inode_usage_percent",
                MetricId::DiskReadRate => "disk.read_bytes_rate",
                MetricId::DiskWriteRate => "disk.write_bytes_rate",
                MetricId::DiskIoTime => "disk.io_time_percent",
                MetricId::NetRxRate => "net.rx_bytes_rate",
                MetricId::NetTxRate => "net.tx_bytes_rate",
                MetricId::NetRxErrors => "net.rx_errors_rate",
                MetricId::NetTxErrors => "net.tx_errors_rate",
                MetricId::ProcCount => "proc.count",
                MetricId::ProcRss => "proc.rss_bytes",
                MetricId::ProcCpu => "proc.cpu_percent",
                MetricId::ProcFdCount => "proc.fd_count",
                MetricId::FdSystemUsage => "fd.system_usage_percent",
                MetricId::KernelEntropy => "kernel.entropy_available",
                MetricId::KernelUptime => "kernel.uptime_secs",
            };
            write!(f, "{}", s)
        }
    }
}
