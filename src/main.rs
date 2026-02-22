use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

mod collector;
mod analyzer;
mod alerter;
mod config;
mod storage;
mod log_analyzer;

#[derive(Parser, Debug)]
#[command(name = "sysops-agent", about = "Lightweight system monitoring agent")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/sysops-agent/config.toml")]
    config: String,

    /// Validate config and exit
    #[arg(long)]
    check: bool,

    /// Print version and exit
    #[arg(short, long)]
    version: bool,
}

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.version {
        println!("sysops-agent {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Load configuration
    let config = config::Config::load(&cli.config)?;

    if cli.check {
        println!("Configuration is valid.");
        return Ok(());
    }

    // Initialize logging
    init_logging(&config)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        hostname = %config.agent.hostname,
        "Starting SysOps Agent"
    );

    // Run the agent
    if let Err(e) = run(config).await {
        error!(error = %e, "Agent terminated with error");
        return Err(e);
    }

    Ok(())
}

fn init_logging(config: &config::Config) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(&config.agent.log_level)
        });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .init();

    Ok(())
}

async fn run(config: config::Config) -> Result<()> {
    // Initialize storage
    let storage = storage::Storage::new(&config.storage)?;

    // Initialize collectors
    let collectors = collector::create_collectors(&config)?;

    // Initialize analyzers
    let analyzers = analyzer::create_analyzers(&config)?;

    // Initialize alerter
    let alerter = alerter::AlertManager::new(&config.alerting)?;

    // Initialize log analyzer
    let log_analyzer = log_analyzer::LogAnalyzer::new(&config)?;

    // Create channels
    let (metric_tx, metric_rx) = tokio::sync::mpsc::channel(10_000);
    let (alert_tx, alert_rx) = tokio::sync::mpsc::channel(1_000);

    // Spawn collector tasks
    for mut c in collectors {
        let tx = metric_tx.clone();
        tokio::spawn(async move {
            loop {
                let interval = c.interval_secs();
                match c.collect().await {
                    Ok(samples) => {
                        for sample in samples {
                            if tx.send(sample).await.is_err() {
                                return; // Channel closed
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(collector = c.name(), error = %e, "Collection failed");
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }
        });
    }
    drop(metric_tx); // Drop our handle

    // Spawn storage ingestion task
    let storage_handle = storage.clone();
    let alert_tx_clone = alert_tx.clone();
    tokio::spawn(async move {
        let mut rx = metric_rx;
        while let Some(sample) = rx.recv().await {
            storage_handle.insert(sample);
        }
    });

    // Spawn analyzer task
    let storage_for_analyzer = storage.clone();
    tokio::spawn(async move {
        let mut analyzers = analyzers;
        loop {
            for analyzer in analyzers.iter_mut() {
                let alerts = analyzer.analyze(&storage_for_analyzer);
                for alert in alerts {
                    if alert_tx_clone.send(alert).await.is_err() {
                        return;
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });

    // Spawn log analyzer task
    let log_alert_tx = alert_tx.clone();
    tokio::spawn(async move {
        let mut log_analyzer = log_analyzer;
        loop {
            match log_analyzer.check().await {
                Ok(alerts) => {
                    for alert in alerts {
                        if log_alert_tx.send(alert).await.is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Log analysis failed");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });
    drop(alert_tx);

    // Spawn alerter task
    let mut alert_rx = alert_rx;
    tokio::spawn(async move {
        while let Some(alert) = alert_rx.recv().await {
            if let Err(e) = alerter.dispatch(alert).await {
                tracing::error!(error = %e, "Alert dispatch failed");
            }
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, exiting");

    Ok(())
}
