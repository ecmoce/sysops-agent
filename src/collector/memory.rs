use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;

use crate::config::MemoryCollectorConfig;
use crate::types::{MetricId, MetricSample};
use super::Collector;

/// Collects memory usage metrics from /proc/meminfo
pub struct MemoryCollector {
    interval: u64,
}

impl MemoryCollector {
    pub fn new(config: &MemoryCollectorConfig) -> Result<Self> {
        Ok(Self { interval: config.interval_secs })
    }
}

#[async_trait]
impl Collector for MemoryCollector {
    fn name(&self) -> &str { "memory" }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let content = tokio::fs::read_to_string("/proc/meminfo").await?;
        let now = Utc::now();
        let mut samples = Vec::new();

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        let mut free_kb = 0u64;
        let mut buffers_kb = 0u64;
        let mut cached_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;
        let mut has_available = false;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            let val: u64 = parts[1].parse().unwrap_or(0);
            match parts[0] {
                "MemTotal:" => total_kb = val,
                "MemAvailable:" => { available_kb = val; has_available = true; }
                "MemFree:" => free_kb = val,
                "Buffers:" => buffers_kb = val,
                "Cached:" => cached_kb = val,
                "SwapTotal:" => swap_total_kb = val,
                "SwapFree:" => swap_free_kb = val,
                _ => {}
            }
        }

        // Fallback for kernels without MemAvailable
        if !has_available {
            available_kb = free_kb + buffers_kb + cached_kb;
        }

        if total_kb > 0 {
            let usage_pct = 100.0 * (1.0 - available_kb as f64 / total_kb as f64);
            samples.push(MetricSample {
                timestamp: now,
                metric: MetricId::MemUsage,
                value: usage_pct,
                labels: smallvec![],
            });
            samples.push(MetricSample {
                timestamp: now,
                metric: MetricId::MemAvailable,
                value: (available_kb * 1024) as f64,
                labels: smallvec![],
            });
        }

        if swap_total_kb > 0 {
            let swap_pct = 100.0 * (1.0 - swap_free_kb as f64 / swap_total_kb as f64);
            samples.push(MetricSample {
                timestamp: now,
                metric: MetricId::MemSwapUsage,
                value: swap_pct,
                labels: smallvec![],
            });
        }

        Ok(samples)
    }

    fn interval_secs(&self) -> u64 { self.interval }
}
