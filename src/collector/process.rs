use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;

use crate::config::ProcessCollectorConfig;
use crate::types::{Label, MetricId, MetricSample};
use super::Collector;

/// Collects per-process metrics from /proc/[pid]/
pub struct ProcessCollector {
    interval: u64,
    track_patterns: Vec<String>,
    track_top_n: u32,
}

impl ProcessCollector {
    pub fn new(config: &ProcessCollectorConfig) -> Result<Self> {
        Ok(Self {
            interval: config.interval_secs,
            track_patterns: config.track_patterns.clone(),
            track_top_n: config.track_top_n,
        })
    }
}

#[async_trait]
impl Collector for ProcessCollector {
    fn name(&self) -> &str { "process" }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let now = Utc::now();
        let mut samples = Vec::new();

        // Count total processes
        let mut proc_count = 0u64;
        let mut entries = tokio::fs::read_dir("/proc").await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_name().to_str()
                .map(|s| s.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
            {
                proc_count += 1;
            }
        }

        samples.push(MetricSample {
            timestamp: now,
            metric: MetricId::ProcCount,
            value: proc_count as f64,
            labels: smallvec![],
        });

        // TODO: Implement per-process tracking
        // - Read /proc/[pid]/status for VmRSS, Threads
        // - Read /proc/[pid]/stat for CPU time
        // - Filter by track_patterns or top-N by RSS
        // - Track RSS over time for leak detection

        Ok(samples)
    }

    fn interval_secs(&self) -> u64 { self.interval }
}
