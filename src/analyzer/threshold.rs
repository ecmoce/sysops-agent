use crate::config::ThresholdConfig;
use crate::storage::Storage;
use crate::types::{Alert, MetricId, Severity};
use super::Analyzer;

/// Simple threshold-based anomaly detection.
/// Compares latest metric values against configured warn/critical thresholds.
pub struct ThresholdAnalyzer {
    thresholds: ThresholdConfig,
    hostname: String,
}

impl ThresholdAnalyzer {
    pub fn new(config: &ThresholdConfig) -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into());
        Self {
            thresholds: config.clone(),
            hostname,
        }
    }

    fn check_metric(&self, storage: &Storage, metric: MetricId, warn: f64, crit: f64) -> Option<Alert> {
        let latest = storage.latest(metric)?;

        let (severity, threshold) = if latest.value >= crit {
            (Severity::Critical, crit)
        } else if latest.value >= warn {
            (Severity::Warn, warn)
        } else {
            return None;
        };

        Some(Alert {
            timestamp: latest.timestamp,
            severity,
            metric,
            value: latest.value,
            threshold: Some(threshold),
            message: format!("{} is {:.1}% (threshold: {:.1}%)", metric, latest.value, threshold),
            labels: latest.labels.clone(),
            hostname: self.hostname.clone(),
        })
    }
}

impl Analyzer for ThresholdAnalyzer {
    fn name(&self) -> &str { "threshold" }

    fn analyze(&mut self, storage: &Storage) -> Vec<Alert> {
        let mut alerts = Vec::new();

        if let Some(a) = self.check_metric(storage, MetricId::CpuUsage,
            self.thresholds.cpu_warn_percent, self.thresholds.cpu_critical_percent) {
            alerts.push(a);
        }
        if let Some(a) = self.check_metric(storage, MetricId::MemUsage,
            self.thresholds.memory_warn_percent, self.thresholds.memory_critical_percent) {
            alerts.push(a);
        }
        if let Some(a) = self.check_metric(storage, MetricId::DiskUsage,
            self.thresholds.disk_warn_percent, self.thresholds.disk_critical_percent) {
            alerts.push(a);
        }
        if let Some(a) = self.check_metric(storage, MetricId::FdSystemUsage,
            self.thresholds.fd_warn_percent, self.thresholds.fd_critical_percent) {
            alerts.push(a);
        }

        alerts
    }
}
