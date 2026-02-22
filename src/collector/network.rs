use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smallvec::smallvec;
use std::collections::HashMap;

use crate::config::NetworkCollectorConfig;
use crate::types::{Label, MetricId, MetricSample};
use super::Collector;

/// Collects network interface metrics from /proc/net/dev
pub struct NetworkCollector {
    interval: u64,
    exclude_interfaces: Vec<String>,
    prev_values: HashMap<String, (u64, u64)>, // interface -> (rx_bytes, tx_bytes)
}

impl NetworkCollector {
    pub fn new(config: &NetworkCollectorConfig) -> Result<Self> {
        Ok(Self {
            interval: config.interval_secs,
            exclude_interfaces: config.exclude_interfaces.clone(),
            prev_values: HashMap::new(),
        })
    }
}

#[async_trait]
impl Collector for NetworkCollector {
    fn name(&self) -> &str { "network" }

    async fn collect(&mut self) -> Result<Vec<MetricSample>> {
        let content = tokio::fs::read_to_string("/proc/net/dev").await?;
        let now = Utc::now();
        let mut samples = Vec::new();

        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 17 { continue; }

            let iface = parts[0].trim_end_matches(':');
            if self.exclude_interfaces.iter().any(|e| e == iface) { continue; }

            let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
            let tx_bytes: u64 = parts[9].parse().unwrap_or(0);
            let rx_errors: u64 = parts[3].parse().unwrap_or(0);
            let tx_errors: u64 = parts[11].parse().unwrap_or(0);

            let labels = smallvec![Label { key: "interface".into(), value: iface.to_string() }];

            if let Some((prev_rx, prev_tx)) = self.prev_values.get(iface) {
                let rx_rate = rx_bytes.saturating_sub(*prev_rx) as f64 / self.interval as f64;
                let tx_rate = tx_bytes.saturating_sub(*prev_tx) as f64 / self.interval as f64;

                samples.push(MetricSample {
                    timestamp: now, metric: MetricId::NetRxRate,
                    value: rx_rate, labels: labels.clone(),
                });
                samples.push(MetricSample {
                    timestamp: now, metric: MetricId::NetTxRate,
                    value: tx_rate, labels: labels.clone(),
                });
            }

            if rx_errors > 0 {
                samples.push(MetricSample {
                    timestamp: now, metric: MetricId::NetRxErrors,
                    value: rx_errors as f64, labels: labels.clone(),
                });
            }
            if tx_errors > 0 {
                samples.push(MetricSample {
                    timestamp: now, metric: MetricId::NetTxErrors,
                    value: tx_errors as f64, labels: labels.clone(),
                });
            }

            self.prev_values.insert(iface.to_string(), (rx_bytes, tx_bytes));
        }

        Ok(samples)
    }

    fn interval_secs(&self) -> u64 { self.interval }
}
