use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;

use crate::config::DiskCollectorConfig;
use crate::types::{Label, MetricId, MetricSample};
use super::Collector;

/// Collects disk usage and I/O metrics
pub struct DiskCollector {
    interval: u64,
    exclude_fstypes: Vec<String>,
    exclude_mountpoints: Vec<String>,
}

impl DiskCollector {
    pub fn new(config: &DiskCollectorConfig) -> Result<Self> {
        Ok(Self {
            interval: config.interval_secs,
            exclude_fstypes: config.exclude_fstypes.clone(),
            exclude_mountpoints: config.exclude_mountpoints.clone(),
        })
    }
}

#[async_trait]
impl Collector for DiskCollector {
    fn name(&self) -> &str { "disk" }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let now = Utc::now();
        let mut samples = Vec::new();

        // Parse /proc/mounts for mounted filesystems
        let mounts = tokio::fs::read_to_string("/proc/mounts").await?;
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 { continue; }

            let _device = parts[0];
            let mountpoint = parts[1];
            let fstype = parts[2];

            if self.exclude_fstypes.iter().any(|f| f == fstype) { continue; }
            if self.exclude_mountpoints.iter().any(|m| m == mountpoint) { continue; }

            // Use statvfs to get disk usage (would use nix::sys::statvfs in real impl)
            // Skeleton: placeholder for actual statvfs call
            let labels = smallvec![
                Label { key: "mountpoint".into(), value: mountpoint.to_string() },
                Label { key: "fstype".into(), value: fstype.to_string() },
            ];

            // TODO: Implement actual statvfs call
            // let stat = nix::sys::statvfs::statvfs(mountpoint)?;
            // let total = stat.blocks() * stat.fragment_size();
            // let avail = stat.blocks_available() * stat.fragment_size();
            // let usage_pct = 100.0 * (1.0 - avail as f64 / total as f64);

            samples.push(MetricSample {
                timestamp: now,
                metric: MetricId::DiskUsage,
                value: 0.0, // placeholder
                labels: labels.clone(),
            });

            samples.push(MetricSample {
                timestamp: now,
                metric: MetricId::DiskAvailable,
                value: 0.0, // placeholder
                labels,
            });
        }

        Ok(samples)
    }

    fn interval_secs(&self) -> u64 { self.interval }
}
