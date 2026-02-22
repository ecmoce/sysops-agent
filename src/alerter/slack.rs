use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::config::SlackConfig;
use crate::types::{Alert, Severity};
use super::AlertChannel;

/// Slack webhook alert channel
pub struct SlackChannel {
    webhook_url: String,
    channel: Option<String>,
    severity_filter: Vec<Severity>,
    client: reqwest::Client,
}

impl SlackChannel {
    pub fn new(config: &SlackConfig) -> Result<Self> {
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
            channel: config.channel.clone(),
            severity_filter,
            client: reqwest::Client::new(),
        })
    }

    fn severity_color(severity: &Severity) -> &'static str {
        match severity {
            Severity::Info => "#2ecc71",
            Severity::Warn => "#f39c12",
            Severity::Critical => "#e74c3c",
            Severity::Emergency => "#9b59b6",
        }
    }
}

#[async_trait]
impl AlertChannel for SlackChannel {
    fn name(&self) -> &str { "slack" }

    fn accepts_severity(&self, severity: &Severity) -> bool {
        self.severity_filter.is_empty() || self.severity_filter.contains(severity)
    }

    async fn send(&self, alert: &Alert) -> Result<()> {
        let mut payload = json!({
            "attachments": [{
                "color": Self::severity_color(&alert.severity),
                "title": format!("[{}] {}", alert.severity, alert.message),
                "fields": [
                    { "title": "Host", "value": &alert.hostname, "short": true },
                    { "title": "Metric", "value": alert.metric.to_string(), "short": true },
                    { "title": "Value", "value": format!("{:.2}", alert.value), "short": true },
                ],
                "ts": alert.timestamp.timestamp(),
            }]
        });

        if let Some(ref ch) = self.channel {
            payload["channel"] = json!(ch);
        }

        self.client.post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
