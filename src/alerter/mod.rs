pub mod discord;
pub mod slack;
pub mod webhook;

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::config::AlertingConfig;
use crate::types::{Alert, MetricId, Severity};

/// Trait for alert delivery channels
#[async_trait]
pub trait AlertChannel: Send + Sync {
    /// Channel name
    fn name(&self) -> &str;

    /// Send a single alert
    async fn send(&self, alert: &Alert) -> Result<()>;

    /// Check if this channel accepts the given severity
    fn accepts_severity(&self, severity: &Severity) -> bool;
}

/// Manages alert dispatch, rate limiting, and deduplication
pub struct AlertManager {
    channels: Vec<Box<dyn AlertChannel>>,
    dedup_map: HashMap<DeduplicationKey, DedupEntry>,
    dedup_window_secs: u64,
    rate_limiter: RateLimiter,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DeduplicationKey {
    metric: MetricId,
    severity: Severity,
}

struct DedupEntry {
    last_sent: Instant,
    count: u32,
}

struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl RateLimiter {
    fn new(per_minute: u32) -> Self {
        Self {
            tokens: per_minute as f64,
            max_tokens: per_minute as f64,
            refill_rate: per_minute as f64 / 60.0,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl AlertManager {
    pub fn new(config: &AlertingConfig) -> Result<Self> {
        let mut channels: Vec<Box<dyn AlertChannel>> = Vec::new();

        if let Some(ref dc) = config.discord {
            if dc.enabled {
                channels.push(Box::new(discord::DiscordChannel::new(dc)?));
            }
        }

        if let Some(ref sc) = config.slack {
            if sc.enabled {
                channels.push(Box::new(slack::SlackChannel::new(sc)?));
            }
        }

        if let Some(ref wc) = config.webhook {
            if wc.enabled {
                channels.push(Box::new(webhook::WebhookChannel::new(wc)?));
            }
        }

        // TODO: Add Telegram, Email, Syslog channels

        tracing::info!(channels = channels.len(), "Initialized alert channels");

        Ok(Self {
            channels,
            dedup_map: HashMap::new(),
            dedup_window_secs: config.dedup_window_secs,
            rate_limiter: RateLimiter::new(config.rate_limit_per_minute),
        })
    }

    pub async fn dispatch(&mut self, alert: Alert) -> Result<()> {
        // Check deduplication
        let key = DeduplicationKey {
            metric: alert.metric,
            severity: alert.severity,
        };

        if let Some(entry) = self.dedup_map.get_mut(&key) {
            let elapsed = entry.last_sent.elapsed().as_secs();
            if elapsed < self.dedup_window_secs && alert.severity < Severity::Emergency {
                entry.count += 1;
                tracing::debug!(metric = %alert.metric, count = entry.count, "Alert deduplicated");
                return Ok(());
            }
        }

        // Check rate limit (bypass for Emergency)
        if alert.severity < Severity::Emergency && !self.rate_limiter.try_acquire() {
            tracing::warn!(metric = %alert.metric, "Alert rate limited");
            return Ok(());
        }

        // Dispatch to all matching channels
        for channel in &self.channels {
            if channel.accepts_severity(&alert.severity) {
                if let Err(e) = channel.send(&alert).await {
                    tracing::error!(
                        channel = channel.name(),
                        error = %e,
                        "Failed to send alert"
                    );
                }
            }
        }

        // Update dedup map
        self.dedup_map.insert(key, DedupEntry {
            last_sent: Instant::now(),
            count: 1,
        });

        Ok(())
    }
}
