use crate::config::AnalyzerConfig;
use crate::storage::Storage;
use crate::types::{Alert, MetricId, Severity};
use super::Analyzer;

/// Z-Score based anomaly detection.
/// Detects values that deviate significantly from the recent average.
pub struct ZScoreAnalyzer {
    window_size: u32,
    threshold: f64,
    hostname: String,
}

impl ZScoreAnalyzer {
    pub fn new(config: &AnalyzerConfig) -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into());
        Self {
            window_size: config.zscore_window,
            threshold: config.zscore_threshold,
            hostname,
        }
    }

    fn check_metric(&self, storage: &Storage, metric: MetricId) -> Option<Alert> {
        let samples = storage.recent(metric, self.window_size as usize);
        if samples.len() < 30 {
            return None; // Not enough data
        }

        let values: Vec<f64> = samples.iter().map(|s| s.value).collect();
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let stddev = variance.sqrt();

        if stddev < 1e-10 {
            return None; // No variation
        }

        let latest = values.last()?;
        let z = (latest - mean) / stddev;

        if z.abs() > self.threshold {
            let severity = if z.abs() > self.threshold * 2.0 {
                Severity::Critical
            } else {
                Severity::Warn
            };

            Some(Alert {
                timestamp: samples.last()?.timestamp,
                severity,
                metric,
                value: *latest,
                threshold: None,
                message: format!(
                    "{} anomaly detected: z-score={:.2} (value={:.1}, mean={:.1}, stddev={:.1})",
                    metric, z, latest, mean, stddev
                ),
                labels: samples.last()?.labels.clone(),
                hostname: self.hostname.clone(),
            })
        } else {
            None
        }
    }
}

impl Analyzer for ZScoreAnalyzer {
    fn name(&self) -> &str { "zscore" }

    fn analyze(&mut self, storage: &Storage) -> Vec<Alert> {
        let metrics = [
            MetricId::CpuUsage,
            MetricId::CpuIoWait,
            MetricId::MemUsage,
            MetricId::NetRxRate,
            MetricId::NetTxRate,
        ];

        metrics.iter()
            .filter_map(|m| self.check_metric(storage, *m))
            .collect()
    }
}
