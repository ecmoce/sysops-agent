use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::config::DiscordConfig;
use crate::types::{Alert, Severity};
use super::AlertChannel;

/// Discord webhook alert channel
pub struct DiscordChannel {
    webhook_url: String,
    username: String,
    severity_filter: Vec<Severity>,
    client: reqwest::Client,
}

impl DiscordChannel {
    pub fn new(config: &DiscordConfig) -> Result<Self> {
        let severity_filter = config.severity_filter.iter()
            .filter_map(|s| match s.as_str() {
                "info" => Some(Severity::Info),
                "warn" => Some(Severity::Warn),
                "critical" => Some(Severity::Critical),
                "emergency" => Some(Severity::Emergency),
                _ => None,
            })
            .collect();

        Ok(Self {
            webhook_url: config.webhook_url.clone(),
            username: config.username.clone().unwrap_or_else(|| "SysOps Agent".into()),
            severity_filter,
            client: reqwest::Client::new(),
        })
    }

    fn severity_color(severity: &Severity) -> u32 {
        match severity {
            Severity::Info => 0x2ECC71,      // green
            Severity::Warn => 0xF39C12,      // yellow
            Severity::Critical => 0xE74C3C,  // red
            Severity::Emergency => 0x9B59B6, // purple
        }
    }
}

#[async_trait]
impl AlertChannel for DiscordChannel {
    fn name(&self) -> &str { "discord" }

    fn accepts_severity(&self, severity: &Severity) -> bool {
        self.severity_filter.is_empty() || self.severity_filter.contains(severity)
    }

    async fn send(&self, alert: &Alert) -> Result<()> {
        let payload = json!({
            "username": self.username,
            "embeds": [{
                "title": format!("[{}] {}", alert.severity, alert.message),
                "color": Self::severity_color(&alert.severity),
                "fields": [
                    { "name": "Host", "value": &alert.hostname, "inline": true },
                    { "name": "Metric", "value": alert.metric.to_string(), "inline": true },
                    { "name": "Value", "value": format!("{:.2}", alert.value), "inline": true },
                ],
                "timestamp": alert.timestamp.to_rfc3339(),
            }]
        });

        self.client.post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
