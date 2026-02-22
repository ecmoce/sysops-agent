# 알림 시스템 설계

## 1. 채널 구성

### Discord Webhook

```toml
[alerting.discord]
enabled = true
webhook_url = "https://discord.com/api/webhooks/ID/TOKEN"
# 또는 환경 변수 참조
# webhook_url = "${DISCORD_WEBHOOK_URL}"
username = "SysOps Agent"
severity_filter = ["warn", "critical", "emergency"]
```

Discord embed 형식으로 전송합니다:
- Color: severity별 (green/yellow/red/purple)
- Title: `[CRITICAL] CPU usage 95.2% on web-01`
- Fields: 메트릭 값, 임계값, 트렌드 정보
- Timestamp: ISO 8601

### Slack Webhook

```toml
[alerting.slack]
enabled = true
webhook_url = "https://hooks.slack.com/services/T.../B.../..."
channel = "#alerts"
severity_filter = ["warn", "critical", "emergency"]
```

Slack Block Kit 형식:
- Section block: 알림 내용
- Context block: 호스트명, 시각
- Color attachment: severity별 색상

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

syslog severity 매핑:
- Info → LOG_INFO
- Warn → LOG_WARNING
- Critical → LOG_CRIT
- Emergency → LOG_EMERG

## 2. Alert Template 시스템

각 채널별로 메시지 템플릿을 커스터마이징할 수 있습니다:

```toml
[alerting.templates]
# 기본 템플릿
default = "[{{severity}}] {{hostname}}: {{message}} ({{metric}}={{value}})"

# 채널별 오버라이드
discord = """
**[{{severity}}]** {{hostname}}
> {{message}}
> `{{metric}}` = **{{value}}** (threshold: {{threshold}})
> 🕐 {{timestamp}}
"""
```

### 템플릿 변수

| 변수 | 설명 |
|------|------|
| `{{hostname}}` | 에이전트 호스트명 |
| `{{severity}}` | 심각도 레벨 |
| `{{metric}}` | 메트릭 이름 |
| `{{value}}` | 현재 값 |
| `{{threshold}}` | 설정된 임계값 |
| `{{message}}` | 알림 메시지 |
| `{{timestamp}}` | ISO 8601 시각 |
| `{{labels}}` | 메트릭 label들 |
| `{{trend}}` | 트렌드 정보 (예측 소진 시간 등) |

## 3. Rate Limiting & Deduplication

### Rate Limiting

Token Bucket 알고리즘:

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

기본 설정:
- 채널당: 분당 10개, 시간당 60개
- 전체: 시간당 100개
- Emergency 알림: rate limit 우회 가능

### Deduplication

동일 알림 재전송 방지:

```rust
struct DeduplicationEntry {
    key: DeduplicationKey,  // (metric, severity, label_hash)
    last_sent: Instant,
    occurrence_count: u32,
}
```

- 기본 중복 억제 기간: 5분 (Warn), 1분 (Critical), 없음 (Emergency)
- 억제 기간 후 재발생 시: "지난 N분간 X회 발생" 정보 포함하여 전송

## 4. Escalation Rules

반복 알림에 대한 자동 에스컬레이션:

```toml
[alerting.escalation]
# Warn이 N회 연속 발생하면 Critical로 승격
warn_to_critical_after = 5
# Critical이 N분간 해소되지 않으면 Emergency
critical_to_emergency_after_mins = 30
```

동작:
1. `cpu.usage > 80%` → Warn 발생
2. 5회 연속 (50초간) 지속 → Critical로 에스컬레이션
3. 30분간 해소되지 않음 → Emergency

## 5. Alert Grouping

관련 알림을 묶어 한 번에 전송합니다:

```toml
[alerting.grouping]
enabled = true
window_secs = 30  # 30초 내 발생한 알림을 하나로 묶음
```

예시:
- 같은 30초 윈도우 내 `disk.usage_percent` warn이 3개 마운트포인트에서 발생
- → 하나의 알림으로 묶어 전송: "3개 디스크 경고: /(91%), /var(88%), /home(85%)"

그룹핑 키: `(severity, metric_category, timestamp_window)`

## 6. Recovery 알림

알림 조건이 해소되면 recovery 알림을 전송합니다:

```toml
[alerting.recovery]
enabled = true
# recovery 알림은 최초 1회만 전송
notify_once = true
```

메시지 예시: `[RESOLVED] CPU usage normalized: 45.2% (was: 95.2%)`
