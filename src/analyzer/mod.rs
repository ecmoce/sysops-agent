pub mod threshold;
pub mod zscore;
pub mod trend;

use anyhow::Result;

use crate::config::Config;
use crate::storage::Storage;
use crate::types::Alert;

/// Trait for anomaly detection analyzers.
///
/// Each analyzer examines stored metrics and produces alerts when anomalies
/// are detected.
pub trait Analyzer: Send + Sync {
    /// Human-readable name for this analyzer
    fn name(&self) -> &str;

    /// Analyze stored metrics and return any alerts
    fn analyze(&mut self, storage: &Storage) -> Vec<Alert>;
}

/// Create all configured analyzers
pub fn create_analyzers(config: &Config) -> Result<Vec<Box<dyn Analyzer>>> {
    let mut analyzers: Vec<Box<dyn Analyzer>> = Vec::new();

    analyzers.push(Box::new(threshold::ThresholdAnalyzer::new(&config.thresholds)));
    analyzers.push(Box::new(zscore::ZScoreAnalyzer::new(&config.analyzer)));
    analyzers.push(Box::new(trend::TrendAnalyzer::new(&config.analyzer)));

    tracing::info!(count = analyzers.len(), "Initialized analyzers");
    Ok(analyzers)
}
