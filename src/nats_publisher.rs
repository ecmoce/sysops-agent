#![cfg(feature = "nats")]

use anyhow::Result;
use async_nats::Client;
use chrono::Utc;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

use crate::config::NatsConfig;
use crate::types::{MetricSample, Alert};

/// Payload matching server's MetricBatch
#[derive(Serialize)]
struct MetricBatch {
    hostname: String,
    timestamp: chrono::DateTime<Utc>,
    metrics: Vec<MetricEntry>,
}

#[derive(Serialize)]
struct MetricEntry {
    name: String,
    value: f64,
    labels: serde_json::Value,
}

/// Payload matching server's Heartbeat
#[derive(Serialize)]
struct HeartbeatPayload {
    hostname: String,
    agent_version: String,
    uptime_seconds: Option<u64>,
    ip_address: Option<String>,
    os: Option<String>,
    arch: Option<String>,
}

/// Payload matching server's Inventory
#[derive(Serialize)]
struct InventoryPayload {
    hostname: String,
    collected_at: chrono::DateTime<Utc>,
    hardware: serde_json::Value,
    software: serde_json::Value,
}

/// Payload matching server's Alert
#[derive(Serialize)]
struct AlertPayload {
    severity: String,
    metric_name: String,
    value: f64,
    threshold: Option<f64>,
    message: String,
    labels: serde_json::Value,
}

pub struct NatsPublisher {
    client: Client,
    config: NatsConfig,
    hostname: String,
    metrics_buffer: Arc<Mutex<Vec<MetricEntry>>>,
    last_inventory_hash: Arc<Mutex<u64>>,
}

impl NatsPublisher {
    pub async fn new(config: NatsConfig, hostname: String) -> Result<Self> {
        let client = if let Some(ref cred) = config.credential_file {
            async_nats::ConnectOptions::with_credentials_file(std::path::PathBuf::from(cred))
                .await?
                .connect(&config.url)
                .await?
        } else if let Some(ref token) = config.token {
            async_nats::ConnectOptions::with_token(token.clone())
                .connect(&config.url)
                .await?
        } else {
            async_nats::connect(&config.url).await?
        };

        info!(url = %config.url, hostname = %hostname, "NATS publisher connected");

        Ok(Self {
            client,
            config,
            hostname,
            metrics_buffer: Arc::new(Mutex::new(Vec::new())),
            last_inventory_hash: Arc::new(Mutex::new(0)),
        })
    }

    fn subject(&self, kind: &str) -> String {
        format!("{}.{}.{}", self.config.subject_prefix, self.hostname, kind)
    }

    fn maybe_compress(&self, data: &[u8]) -> Vec<u8> {
        if self.config.compression {
            match zstd::encode_all(data, 3) {
                Ok(compressed) => compressed,
                Err(e) => {
                    warn!(error = %e, "zstd compression failed, sending uncompressed");
                    data.to_vec()
                }
            }
        } else {
            data.to_vec()
        }
    }

    /// Buffer a metric sample. Call flush_metrics() periodically.
    pub async fn buffer_metric(&self, sample: &MetricSample) {
        let labels = sample.labels.iter().map(|l| {
            (l.key.clone(), serde_json::Value::String(l.value.clone()))
        }).collect::<serde_json::Map<String, serde_json::Value>>();

        let entry = MetricEntry {
            name: sample.metric.to_string(),
            value: sample.value,
            labels: serde_json::Value::Object(labels),
        };

        self.metrics_buffer.lock().await.push(entry);
    }

    /// Flush buffered metrics as a batch
    pub async fn flush_metrics(&self) {
        let entries: Vec<MetricEntry> = {
            let mut buf = self.metrics_buffer.lock().await;
            if buf.is_empty() {
                return;
            }
            std::mem::take(&mut *buf)
        };

        let count = entries.len();
        let batch = MetricBatch {
            hostname: self.hostname.clone(),
            timestamp: Utc::now(),
            metrics: entries,
        };

        match serde_json::to_vec(&batch) {
            Ok(json) => {
                let payload = self.maybe_compress(&json);
                let subject = self.subject("metrics");
                if let Err(e) = self.client.publish(subject.clone(), payload.into()).await {
                    error!(error = %e, subject = %subject, "Failed to publish metrics");
                } else {
                    debug!(count, subject = %subject, "Published metrics batch");
                }
            }
            Err(e) => error!(error = %e, "Failed to serialize metrics"),
        }
    }

    /// Publish an alert immediately
    pub async fn publish_alert(&self, alert: &Alert) {
        let labels = alert.labels.iter().map(|l| {
            (l.key.clone(), serde_json::Value::String(l.value.clone()))
        }).collect::<serde_json::Map<String, serde_json::Value>>();

        let payload = AlertPayload {
            severity: alert.severity.to_string().to_lowercase(),
            metric_name: alert.metric.to_string(),
            value: alert.value,
            threshold: alert.threshold,
            message: alert.message.clone(),
            labels: serde_json::Value::Object(labels),
        };

        match serde_json::to_vec(&payload) {
            Ok(json) => {
                let data = self.maybe_compress(&json);
                let subject = self.subject("alerts");
                if let Err(e) = self.client.publish(subject.clone(), data.into()).await {
                    error!(error = %e, "Failed to publish alert");
                } else {
                    info!(subject = %subject, severity = %alert.severity, "Published alert");
                }
            }
            Err(e) => error!(error = %e, "Failed to serialize alert"),
        }
    }

    /// Publish inventory
    pub async fn publish_inventory(&self, hardware: &serde_json::Value, software: &serde_json::Value) {
        let payload = InventoryPayload {
            hostname: self.hostname.clone(),
            collected_at: Utc::now(),
            hardware: hardware.clone(),
            software: software.clone(),
        };

        match serde_json::to_vec(&payload) {
            Ok(json) => {
                let data = self.maybe_compress(&json);
                let subject = self.subject("inventory");
                if let Err(e) = self.client.publish(subject.clone(), data.into()).await {
                    error!(error = %e, "Failed to publish inventory");
                } else {
                    info!(subject = %subject, "Published inventory");
                }
            }
            Err(e) => error!(error = %e, "Failed to serialize inventory"),
        }
    }

    /// Publish heartbeat
    pub async fn publish_heartbeat(&self) {
        let uptime = read_uptime();
        let payload = HeartbeatPayload {
            hostname: self.hostname.clone(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            ip_address: get_primary_ip(),
            os: get_os_info(),
            arch: Some(std::env::consts::ARCH.to_string()),
        };

        match serde_json::to_vec(&payload) {
            Ok(json) => {
                let data = self.maybe_compress(&json);
                let subject = self.subject("heartbeat");
                if let Err(e) = self.client.publish(subject.clone(), data.into()).await {
                    error!(error = %e, "Failed to publish heartbeat");
                } else {
                    debug!(subject = %subject, "Published heartbeat");
                }
            }
            Err(e) => error!(error = %e, "Failed to serialize heartbeat"),
        }
    }
}

fn read_uptime() -> Option<u64> {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v as u64)
}

fn get_primary_ip() -> Option<String> {
    // Try to get a non-loopback IP from /proc/net/fib_trie or fallback
    std::fs::read_to_string("/proc/net/if_inet6")
        .ok();
    // Simple approach: read hostname's resolved IP
    None
}

fn get_os_info() -> Option<String> {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return Some(line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                }
            }
            None
        })
}
