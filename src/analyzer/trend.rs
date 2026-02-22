use crate::config::AnalyzerConfig;
use crate::storage::Storage;
use crate::types::{Alert, MetricId, Severity};
use super::Analyzer;

/// Trend-based anomaly detection using linear regression.
/// Predicts resource exhaustion time and alerts accordingly.
pub struct TrendAnalyzer {
    window_hours: u32,
    hostname: String,
}

impl TrendAnalyzer {
    pub fn new(config: &AnalyzerConfig) -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into());
        Self {
            window_hours: config.trend_window_hours,
            hostname,
        }
    }

    /// Perform simple linear regression on (x, y) pairs.
    /// Returns (slope, intercept, r_squared).
    fn linear_regression(xs: &[f64], ys: &[f64]) -> Option<(f64, f64, f64)> {
        let n = xs.len() as f64;
        if n < 2.0 { return None; }

        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = ys.iter().sum();
        let sum_xy: f64 = xs.iter().zip(ys).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = xs.iter().map(|x| x * x).sum();
        let sum_y2: f64 = ys.iter().map(|y| y * y).sum();

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 { return None; }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / n;

        // R-squared
        let ss_res: f64 = xs.iter().zip(ys).map(|(x, y)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        }).sum();
        let mean_y = sum_y / n;
        let ss_tot: f64 = ys.iter().map(|y| (y - mean_y).powi(2)).sum();
        let r_squared = if ss_tot > 1e-10 { 1.0 - ss_res / ss_tot } else { 0.0 };

        Some((slope, intercept, r_squared))
    }

    fn check_exhaustion(&self, storage: &Storage, metric: MetricId, limit: f64, hours_warn: f64, hours_crit: f64) -> Option<Alert> {
        let samples_needed = (self.window_hours as usize) * 360; // 10s intervals
        let samples = storage.recent(metric, samples_needed);
        if samples.len() < 60 { return None; } // At least 10 minutes of data

        let xs: Vec<f64> = samples.iter()
            .map(|s| s.timestamp.timestamp() as f64)
            .collect();
        let ys: Vec<f64> = samples.iter().map(|s| s.value).collect();

        let (slope, _intercept, r_squared) = Self::linear_regression(&xs, &ys)?;

        // Only alert if trend is increasing with good fit
        if slope <= 0.0 || r_squared < 0.5 { return None; }

        let current = *ys.last()?;
        if current >= limit { return None; } // Already exceeded

        let remaining = limit - current;
        let hours_to_exhaustion = remaining / (slope * 3600.0);

        if hours_to_exhaustion > hours_warn { return None; }

        let severity = if hours_to_exhaustion <= hours_crit {
            Severity::Critical
        } else {
            Severity::Warn
        };

        Some(Alert {
            timestamp: samples.last()?.timestamp,
            severity,
            metric,
            value: current,
            threshold: Some(limit),
            message: format!(
                "{} trending toward exhaustion: {:.1}h remaining (current={:.1}%, slope={:.4}/s, R²={:.2})",
                metric, hours_to_exhaustion, current, slope, r_squared
            ),
            labels: samples.last()?.labels.clone(),
            hostname: self.hostname.clone(),
        })
    }
}

impl Analyzer for TrendAnalyzer {
    fn name(&self) -> &str { "trend" }

    fn analyze(&mut self, storage: &Storage) -> Vec<Alert> {
        let mut alerts = Vec::new();

        // Disk: warn at 72h, critical at 24h
        if let Some(a) = self.check_exhaustion(storage, MetricId::DiskUsage, 100.0, 72.0, 24.0) {
            alerts.push(a);
        }

        // Memory: warn at 12h, critical at 6h
        if let Some(a) = self.check_exhaustion(storage, MetricId::MemUsage, 100.0, 12.0, 6.0) {
            alerts.push(a);
        }

        // FD: warn at 24h, critical at 6h
        if let Some(a) = self.check_exhaustion(storage, MetricId::FdSystemUsage, 100.0, 24.0, 6.0) {
            alerts.push(a);
        }

        alerts
    }
}
