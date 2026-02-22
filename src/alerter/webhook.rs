use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::config::WebhookConfig;
use crate::types::{Alert, Severity};
use super::AlertChannel;

/// Custom webhook alert channel
pub struct WebhookChannel {
    url: String,
    headers: std::collections::HashMap<String, String>,
    severity_filter: Vec<Severity>,
    client: reqwest::Client,
}

impl WebhookChannel {
    pub fn new(config: &WebhookConfig) -> Result<Self> {
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
            url: config.url.clone(),
            headers: config.headers.clone(),
            severity_filter,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl AlertChannel for WebhookChannel {
    fn name(&self) -> &str { "webhook" }

    fn accepts_severity(&self, severity: &Severity) -> bool {
        self.severity_filter.is_empty() || self.severity_filter.contains(severity)
    }

    async fn send(&self, alert: &Alert) -> Result<()> {
        let payload = json!({
            "hostname": &alert.hostname,
            "metric": alert.metric.to_string(),
            "value": alert.value,
            "severity": alert.severity.to_string(),
            "message": &alert.message,
            "timestamp": alert.timestamp.to_rfc3339(),
        });

        let mut req = self.client.post(&self.url).json(&payload);
        for (k, v) in &self.headers {
            req = req.header(k, v);
        }

        req.send().await?.error_for_status()?;
        Ok(())
    }
}
