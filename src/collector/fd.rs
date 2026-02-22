use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;

use crate::types::{MetricId, MetricSample};
use super::Collector;

/// Collects system-wide file descriptor usage from /proc/sys/fs/file-nr
pub struct FdCollector;

impl FdCollector {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl Collector for FdCollector {
    fn name(&self) -> &str { "fd" }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let content = tokio::fs::read_to_string("/proc/sys/fs/file-nr").await?;
        let now = Utc::now();
        let parts: Vec<&str> = content.split_whitespace().collect();

        if parts.len() >= 3 {
            let used: f64 = parts[0].parse().unwrap_or(0.0);
            let max: f64 = parts[2].parse().unwrap_or(1.0);
            let usage_pct = 100.0 * used / max;

            Ok(vec![MetricSample {
                timestamp: now,
                metric: MetricId::FdSystemUsage,
                value: usage_pct,
                labels: smallvec![],
            }])
        } else {
            Ok(vec![])
        }
    }

    fn interval_secs(&self) -> u64 { 30 }
}
