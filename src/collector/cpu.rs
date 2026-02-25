use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;

use crate::config::CpuCollectorConfig;
use crate::types::{Label, MetricId, MetricSample};
use super::Collector;

/// Collects CPU usage metrics from /proc/stat
pub struct CpuCollector {
    interval: u64,
    per_core: bool,
    prev_total: Vec<u64>,
    prev_idle: Vec<u64>,
}

impl CpuCollector {
    pub fn new(config: &CpuCollectorConfig) -> Result<Self> {
        Ok(Self {
            interval: config.interval_secs,
            per_core: config.per_core,
            prev_total: Vec::new(),
            prev_idle: Vec::new(),
        })
    }

    /// Parse /proc/stat and compute CPU usage percentages
    fn parse_stat(&mut self, content: &str) -> Vec<MetricSample> {
        let mut samples = Vec::new();
        let now = Utc::now();
        let mut core_idx = 0usize;

        for line in content.lines() {
            if line.starts_with("cpu") {
                let fields: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|f| f.parse().ok())
                    .collect();

                if fields.len() < 7 {
                    continue;
                }

                let total: u64 = fields.iter().sum();
                let idle = fields[3] + fields.get(4).copied().unwrap_or(0); // idle + iowait
                let iowait = fields.get(4).copied().unwrap_or(0);
                let steal = fields.get(7).copied().unwrap_or(0);

                let is_total = line.starts_with("cpu ");
                let idx = if is_total { 0 } else { core_idx };

                // Ensure we have previous values
                while self.prev_total.len() <= idx {
                    self.prev_total.push(0);
                    self.prev_idle.push(0);
                }

                if self.prev_total[idx] > 0 {
                    let d_total = total.saturating_sub(self.prev_total[idx]);
                    let d_idle = idle.saturating_sub(self.prev_idle[idx]);

                    if d_total > 0 {
                        let usage = 100.0 * (1.0 - d_idle as f64 / d_total as f64);

                        if is_total {
                            samples.push(MetricSample {
                                timestamp: now,
                                metric: MetricId::CpuUsage,
                                value: usage,
                                labels: smallvec![],
                            });

                            // iowait and steal as percentage of delta total
                            // Note: iowait/steal are cumulative, so use them as proportion of total
                            let iowait_pct = if d_total > 0 { 100.0 * iowait as f64 / total as f64 } else { 0.0 };
                            samples.push(MetricSample {
                                timestamp: now,
                                metric: MetricId::CpuIoWait,
                                value: iowait_pct,
                                labels: smallvec![],
                            });

                            // steal percentage
                            let steal_pct = if d_total > 0 { 100.0 * steal as f64 / total as f64 } else { 0.0 };
                            samples.push(MetricSample {
                                timestamp: now,
                                metric: MetricId::CpuSteal,
                                value: steal_pct,
                                labels: smallvec![],
                            });
                        } else if self.per_core {
                            samples.push(MetricSample {
                                timestamp: now,
                                metric: MetricId::CpuUsagePerCore,
                                value: usage,
                                labels: smallvec![Label {
                                    key: "core".into(),
                                    value: (core_idx - 1).to_string(),
                                }],
                            });
                        }
                    }
                }

                self.prev_total[idx] = total;
                self.prev_idle[idx] = idle;

                if !is_total {
                    core_idx += 1;
                } else {
                    core_idx = 1;
                }
            }
        }

        samples
    }
}

#[async_trait]
impl Collector for CpuCollector {
    fn name(&self) -> &str {
        "cpu"
    }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let content = tokio::fs::read_to_string("/proc/stat").await?;
        let mut samples = self.parse_stat(&content);

        // Load average from /proc/loadavg
        if let Ok(loadavg) = tokio::fs::read_to_string("/proc/loadavg").await {
            let parts: Vec<&str> = loadavg.split_whitespace().collect();
            if parts.len() >= 3 {
                let now = Utc::now();
                if let Ok(v) = parts[0].parse::<f64>() {
                    samples.push(MetricSample {
                        timestamp: now,
                        metric: MetricId::CpuLoad1m,
                        value: v,
                        labels: smallvec![],
                    });
                }
                if let Ok(v) = parts[1].parse::<f64>() {
                    samples.push(MetricSample {
                        timestamp: now,
                        metric: MetricId::CpuLoad5m,
                        value: v,
                        labels: smallvec![],
                    });
                }
                if let Ok(v) = parts[2].parse::<f64>() {
                    samples.push(MetricSample {
                        timestamp: now,
                        metric: MetricId::CpuLoad15m,
                        value: v,
                        labels: smallvec![],
                    });
                }
            }
        }

        Ok(samples)
    }

    fn interval_secs(&self) -> u64 {
        self.interval
    }
}
