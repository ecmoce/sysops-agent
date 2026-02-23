# Alert System Design

## 1. Channel Configuration

### Discord Webhook

```toml
[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/ID/TOKEN"
# or environment variable reference
# webhook_url = "${DISCORD_WEBHOOK_URL}"
username = "SysOps Agent"
severity_filter = ["warn", "critical", "emergency"]
```

Transmitted in Discord embed format:
- Color: by severity (green/yellow/red/purple)
- Title: `[CRITICAL] CPU usage 95.2% on web-01`
- Fields: metric value, threshold, trend information
- Timestamp: ISO 8601

### Slack Webhook

```toml
[alerting.slack]
enabled = true
webhook_url = "https://hooks.slack.com/services/T.../B.../..."
channel = "#alerts"
severity_filter = ["warn", "critical", "emergency"]
```

Slack Block Kit format:
- Section block: alert content
- Context block: hostname, timestamp
- Color attachment: color by severity

### Telegram Bot

```toml
[alerting.telegram]
enabled = true
bot_token = "${TELEGRAM_BOT_TOKEN}"
chat_id = "-1001234567890"
severity_filter = ["critical", "emergency"]
parse_mode = "HTML"
```

### Email (SMTP)

```toml
[alerting.email]
enabled = true
smtp_host = "smtp.gmail.com"
smtp_port = 587
smtp_tls = true
username = "${SMTP_USER}"
password = "${SMTP_PASSWORD}"
from = "sysops@example.com"
to = ["admin@example.com", "oncall@example.com"]
severity_filter = ["critical", "emergency"]
```

### Custom Webhook

```toml
[alerting.webhook]
enabled = true
url = "https://api.example.com/alerts"
method = "POST"
headers = { "Authorization" = "Bearer ${API_TOKEN}" }
severity_filter = ["warn", "critical", "emergency"]
```

JSON payload:
```json
{
  "hostname": "web-01",
  "metric": "cpu.usage_percent",
  "value": 95.2,
  "severity": "critical",
  "message": "CPU usage exceeded critical threshold",
  "timestamp": "2025-01-15T10:30:00Z",
  "labels": {"core": "all"}
}
```

### Local Syslog

```toml
[alerting.syslog]
enabled = true
facility = "daemon"
severity_filter = ["info", "warn", "critical", "emergency"]
```

syslog severity mapping:
- Info → LOG_INFO
- Warn → LOG_WARNING
- Critical → LOG_CRIT
- Emergency → LOG_EMERG

## 2. Alert Template System

Message templates can be customized for each channel:

```toml
[alerting.templates]
# Default template
default = "[{{severity}}] {{hostname}}: {{message}} ({{metric}}={{value}})"

# Channel-specific overrides
discord = """
**[{{severity}}]** {{hostname}}
> {{message}}
> `{{metric}}` = **{{value}}** (threshold: {{threshold}})
> 🕐 {{timestamp}}
"""
```

### Template Variables

| Variable | Description |
|----------|-------------|
| `{{hostname}}` | Agent hostname |
| `{{severity}}` | Severity level |
| `{{metric}}` | Metric name |
| `{{value}}` | Current value |
| `{{threshold}}` | Configured threshold |
| `{{message}}` | Alert message |
| `{{timestamp}}` | ISO 8601 timestamp |
| `{{labels}}` | Metric labels |
| `{{trend}}` | Trend information (predicted depletion time, etc.) |

## 3. Rate Limiting & Deduplication

### Rate Limiting

Token Bucket algorithm:

```rust
struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,  // tokens per second
    last_refill: Instant,
}

impl RateLimiter {
    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
```

Default configuration:
- Per channel: 10 per minute, 60 per hour
- Global: 100 per hour
- Emergency alerts: can bypass rate limit

### Deduplication

Prevent duplicate alert retransmission:

```rust
struct DeduplicationEntry {
    key: DeduplicationKey,  // (metric, severity, label_hash)
    last_sent: Instant,
    occurrence_count: u32,
}
```

- Default deduplication period: 5 minutes (Warn), 1 minute (Critical), none (Emergency)
- On reoccurrence after suppression period: send with "occurred X times in last N minutes" information

## 4. Escalation Rules

Automatic escalation for repeated alerts:

```toml
[alerting.escalation]
# Escalate Warn to Critical after N consecutive occurrences
warn_to_critical_after = 5
# Escalate Critical to Emergency after N minutes unresolved
critical_to_emergency_after_mins = 30
```

Operation:
1. `cpu.usage > 80%` → Warn triggered
2. Persists 5 consecutive times (50 seconds) → Escalate to Critical
3. Unresolved for 30 minutes → Emergency

## 5. Alert Grouping

Group related alerts for single transmission:

```toml
[alerting.grouping]
enabled = true
window_secs = 30  # Group alerts occurring within 30 seconds
```

Example:
- Within same 30-second window, `disk.usage_percent` warnings from 3 mount points
- → Send as single grouped alert: "3 disk warnings: /(91%), /var(88%), /home(85%)"

Grouping key: `(severity, metric_category, timestamp_window)`

## 6. Recovery Alerts

Send recovery alerts when alert conditions are resolved:

```toml
[alerting.recovery]
enabled = true
# Send recovery alert only once initially
notify_once = true
```

Message example: `[RESOLVED] CPU usage normalized: 45.2% (was: 95.2%)`