use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
#[cfg(feature = "nats")]
use std::sync::Arc;

use sysops_agent::{collector, analyzer, alerter, config, storage, log_analyzer};
#[cfg(feature = "nats")]
use sysops_agent::{nats_publisher, inventory};

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
    let mut alerter = alerter::AlertManager::new(&config.alerting)?;

    // Initialize log analyzer
    let log_analyzer = log_analyzer::LogAnalyzer::new(&config)?;

    // Initialize NATS publisher (if enabled)
    #[cfg(feature = "nats")]
    let nats_pub = if config.nats.enabled {
        match nats_publisher::NatsPublisher::new(
            config.nats.clone(),
            config.agent.hostname.clone(),
        ).await {
            Ok(np) => {
                info!("NATS publisher initialized");
                Some(Arc::new(np))
            }
            Err(e) => {
                error!(error = %e, "Failed to initialize NATS publisher, continuing without it");
                None
            }
        }
    } else {
        None
    };

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

    // Spawn storage ingestion task (+ NATS metric buffering)
    let storage_handle = storage.clone();
    let alert_tx_clone = alert_tx.clone();
    #[cfg(feature = "nats")]
    let nats_for_metrics = nats_pub.clone();
    tokio::spawn(async move {
        let mut rx = metric_rx;
        while let Some(sample) = rx.recv().await {
            #[cfg(feature = "nats")]
            if let Some(ref np) = nats_for_metrics {
                np.buffer_metric(&sample).await;
            }
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

    // Spawn alerter task (+ NATS alert publishing)
    let mut alert_rx = alert_rx;
    #[cfg(feature = "nats")]
    let nats_for_alerts = nats_pub.clone();
    tokio::spawn(async move {
        while let Some(alert) = alert_rx.recv().await {
            #[cfg(feature = "nats")]
            if let Some(ref np) = nats_for_alerts {
                np.publish_alert(&alert).await;
            }
            if let Err(e) = alerter.dispatch(alert).await {
                tracing::error!(error = %e, "Alert dispatch failed");
            }
        }
    });

    // Spawn NATS periodic tasks (metrics flush, heartbeat, inventory)
    #[cfg(feature = "nats")]
    if let Some(ref np) = nats_pub {
        // Small delay to let server subscribe first
        let np_init = np.clone();
        let proc_root_init = config.agent.proc_root.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            np_init.publish_heartbeat().await;
            let (hw, sw) = inventory::collect_inventory(&proc_root_init);
            np_init.publish_inventory(&hw, &sw).await;
        });

        // Metrics flush loop
        let np_flush = np.clone();
        let flush_interval = config.nats.metrics_interval_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(flush_interval));
            loop {
                interval.tick().await;
                np_flush.flush_metrics().await;
            }
        });

        // Heartbeat loop
        let np_hb = np.clone();
        let hb_interval = config.nats.heartbeat_interval_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(hb_interval));
            loop {
                interval.tick().await;
                np_hb.publish_heartbeat().await;
            }
        });

        // Inventory loop
        let np_inv = np.clone();
        let inv_interval = config.nats.inventory_interval_secs;
        let proc_root = config.agent.proc_root.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(inv_interval));
            loop {
                interval.tick().await;
                let (hw, sw) = inventory::collect_inventory(&proc_root);
                np_inv.publish_inventory(&hw, &sw).await;
            }
        });
    }

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, exiting");

    Ok(())
}
