use anyhow::Result;
use chrono::Utc;
use regex::Regex;
use smallvec::smallvec;

use crate::config::Config;
use crate::types::{Alert, MetricId, Severity};

/// Analyzes kernel and system logs for critical events
pub struct LogAnalyzer {
    patterns: Vec<LogPattern>,
    hostname: String,
    // TODO: track file position for syslog tailing
}

struct LogPattern {
    name: String,
    regex: Regex,
    severity: Severity,
    metric: MetricId,
}

impl LogAnalyzer {
    pub fn new(config: &Config) -> Result<Self> {
        let mut patterns = vec![
            LogPattern {
                name: "OOM Kill".into(),
                regex: Regex::new(r"Out of memory: Killed process (\d+) \((.+)\)")?,
                severity: Severity::Critical,
                metric: MetricId::KernelEntropy, // placeholder
            },
            LogPattern {
                name: "Hardware Error".into(),
                regex: Regex::new(r"(?i)(Hardware Error|Machine check|MCE|ECC|EDAC|uncorrectable error)")?,
                severity: Severity::Critical,
                metric: MetricId::KernelEntropy,
            },
            LogPattern {
                name: "Filesystem Error".into(),
                regex: Regex::new(r"(?i)(EXT4-fs error|XFS.*error|Remounting filesystem read-only|I/O error)")?,
                severity: Severity::Critical,
                metric: MetricId::KernelEntropy,
            },
            LogPattern {
                name: "Hung Task".into(),
                regex: Regex::new(r"task .+ blocked for more than \d+ seconds")?,
                severity: Severity::Critical,
                metric: MetricId::KernelEntropy,
            },
            LogPattern {
                name: "Network Down".into(),
                regex: Regex::new(r"(?i)(link is not ready|NIC Link is Down|carrier lost)")?,
                severity: Severity::Warn,
                metric: MetricId::KernelEntropy,
            },
        ];

        // Add custom patterns from config
        for cp in &config.collector.log.custom_patterns {
            let severity = match cp.severity.as_str() {
                "info" => Severity::Info,
                "warn" => Severity::Warn,
                "critical" => Severity::Critical,
                "emergency" => Severity::Emergency,
                _ => Severity::Warn,
            };
            patterns.push(LogPattern {
                name: cp.name.clone(),
                regex: Regex::new(&cp.pattern)?,
                severity,
                metric: MetricId::KernelEntropy,
            });
        }

        let hostname = config.agent.hostname.clone();

        Ok(Self { patterns, hostname })
    }

    pub async fn check(&mut self) -> Result<Vec<Alert>> {
        let mut alerts = Vec::new();

        // Read dmesg (requires CAP_SYSLOG)
        if let Ok(output) = tokio::process::Command::new("dmesg")
            .arg("--time-format=iso")
            .arg("--level=err,crit,alert,emerg")
            .output()
            .await
        {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                for line in content.lines() {
                    for pattern in &self.patterns {
                        if pattern.regex.is_match(line) {
                            alerts.push(Alert {
                                timestamp: Utc::now(),
                                severity: pattern.severity,
                                metric: pattern.metric,
                                value: 1.0,
                                threshold: None,
                                message: format!("{}: {}", pattern.name, line.trim()),
                                labels: smallvec![],
                                hostname: self.hostname.clone(),
                            });
                        }
                    }
                }
            }
        }

        // TODO: Implement syslog file tailing
        // TODO: Implement systemd journal reading

        Ok(alerts)
    }
}
