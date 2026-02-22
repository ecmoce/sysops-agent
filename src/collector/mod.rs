pub mod cpu;
pub mod memory;
pub mod disk;
pub mod network;
pub mod process;
pub mod fd;

use anyhow::Result;
use async_trait::async_trait;

use crate::config::Config;
use crate::types::MetricSample;

/// Trait for all metric collectors.
///
/// Each collector is responsible for gathering one category of system metrics
/// from procfs/sysfs and returning them as `MetricSample` values.
#[async_trait]
pub trait Collector: Send + Sync {
    /// Human-readable name for this collector (e.g., "cpu", "memory")
    fn name(&self) -> &str;

    /// Collect metrics. Called periodically by the scheduler.
    /// Returns a vector of metric samples, or an error if collection failed.
    async fn collect(&mut self) -> Result<Vec<MetricSample>>;

    /// Collection interval in seconds
    fn interval_secs(&self) -> u64;
}

/// Create all enabled collectors based on configuration
pub fn create_collectors(config: &Config) -> Result<Vec<Box<dyn Collector>>> {
    let mut collectors: Vec<Box<dyn Collector>> = Vec::new();

    if config.collector.cpu.enabled {
        collectors.push(Box::new(cpu::CpuCollector::new(&config.collector.cpu)?));
    }

    if config.collector.memory.enabled {
        collectors.push(Box::new(memory::MemoryCollector::new(&config.collector.memory)?));
    }

    if config.collector.disk.enabled {
        collectors.push(Box::new(disk::DiskCollector::new(&config.collector.disk)?));
    }

    if config.collector.network.enabled {
        collectors.push(Box::new(network::NetworkCollector::new(&config.collector.network)?));
    }

    if config.collector.process.enabled {
        collectors.push(Box::new(process::ProcessCollector::new(&config.collector.process)?));
    }

    // FD collector is always enabled (lightweight)
    collectors.push(Box::new(fd::FdCollector::new()?));

    tracing::info!(count = collectors.len(), "Initialized collectors");
    Ok(collectors)
}
