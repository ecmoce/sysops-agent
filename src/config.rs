use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Top-level configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub agent: AgentConfig,
    #[serde(default)]
    pub collector: CollectorConfig,
    #[serde(default)]
    pub thresholds: ThresholdConfig,
    #[serde(default)]
    pub analyzer: AnalyzerConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub alerting: AlertingConfig,
    #[serde(default)]
    pub prometheus: PrometheusConfig,
    #[cfg(feature = "nats")]
    #[serde(default)]
    pub nats: NatsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    #[serde(default = "default_hostname")]
    pub hostname: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    pub log_file: Option<String>,
    pub pid_file: Option<String>,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_proc_root")]
    pub proc_root: String,
    #[serde(default = "default_sys_root")]
    pub sys_root: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CollectorConfig {
    #[serde(default = "default_interval")]
    pub default_interval_secs: u64,
    #[serde(default)]
    pub cpu: CpuCollectorConfig,
    #[serde(default)]
    pub memory: MemoryCollectorConfig,
    #[serde(default)]
    pub disk: DiskCollectorConfig,
    #[serde(default)]
    pub network: NetworkCollectorConfig,
    #[serde(default)]
    pub process: ProcessCollectorConfig,
    #[serde(default)]
    pub log: LogCollectorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CpuCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_true")]
    pub per_core: bool,
}

impl Default for CpuCollectorConfig {
    fn default() -> Self {
        Self { enabled: true, interval_secs: 10, per_core: true }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemoryCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

impl Default for MemoryCollectorConfig {
    fn default() -> Self {
        Self { enabled: true, interval_secs: 10 }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiskCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_disk_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_interval")]
    pub io_interval_secs: u64,
    #[serde(default = "default_exclude_fstypes")]
    pub exclude_fstypes: Vec<String>,
    #[serde(default)]
    pub exclude_mountpoints: Vec<String>,
}

impl Default for DiskCollectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 60,
            io_interval_secs: 10,
            exclude_fstypes: default_exclude_fstypes(),
            exclude_mountpoints: vec![],
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_exclude_interfaces")]
    pub exclude_interfaces: Vec<String>,
}

impl Default for NetworkCollectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 10,
            exclude_interfaces: default_exclude_interfaces(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_process_interval")]
    pub interval_secs: u64,
    #[serde(default)]
    pub track_patterns: Vec<String>,
    #[serde(default = "default_top_n")]
    pub track_top_n: u32,
}

impl Default for ProcessCollectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 30,
            track_patterns: vec![],
            track_top_n: 20,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogCollectorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_log_sources")]
    pub sources: Vec<String>,
    pub syslog_path: Option<String>,
    #[serde(default)]
    pub custom_patterns: Vec<CustomPattern>,
}

impl Default for LogCollectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sources: default_log_sources(),
            syslog_path: None,
            custom_patterns: vec![],
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomPattern {
    pub name: String,
    pub pattern: String,
    pub severity: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThresholdConfig {
    #[serde(default = "default_80")]
    pub cpu_warn_percent: f64,
    #[serde(default = "default_95")]
    pub cpu_critical_percent: f64,
    #[serde(default = "default_80")]
    pub memory_warn_percent: f64,
    #[serde(default = "default_90")]
    pub memory_critical_percent: f64,
    #[serde(default = "default_80")]
    pub disk_warn_percent: f64,
    #[serde(default = "default_90")]
    pub disk_critical_percent: f64,
    #[serde(default = "default_80")]
    pub fd_warn_percent: f64,
    #[serde(default = "default_95")]
    pub fd_critical_percent: f64,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            cpu_warn_percent: 80.0,
            cpu_critical_percent: 95.0,
            memory_warn_percent: 80.0,
            memory_critical_percent: 90.0,
            disk_warn_percent: 80.0,
            disk_critical_percent: 90.0,
            fd_warn_percent: 80.0,
            fd_critical_percent: 95.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerConfig {
    #[serde(default = "default_zscore_window")]
    pub zscore_window: u32,
    #[serde(default = "default_zscore_threshold")]
    pub zscore_threshold: f64,
    #[serde(default = "default_ema_alpha")]
    pub ema_alpha: f64,
    #[serde(default = "default_trend_window")]
    pub trend_window_hours: u32,
    #[serde(default = "default_leak_observation")]
    pub leak_min_observation_mins: u32,
    #[serde(default = "default_r_squared")]
    pub leak_r_squared_threshold: f64,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            zscore_window: 360,
            zscore_threshold: 3.0,
            ema_alpha: 0.1,
            trend_window_hours: 6,
            leak_min_observation_mins: 30,
            leak_r_squared_threshold: 0.8,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    #[serde(default = "default_ring_buffer_size")]
    pub ring_buffer_size: u32,
    #[serde(default)]
    pub sqlite_enabled: bool,
    pub sqlite_path: Option<String>,
    #[serde(default = "default_retention_days")]
    pub sqlite_retention_days: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            ring_buffer_size: 8640,
            sqlite_enabled: false,
            sqlite_path: None,
            sqlite_retention_days: 30,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AlertingConfig {
    #[serde(default = "default_rate_per_min")]
    pub rate_limit_per_minute: u32,
    #[serde(default = "default_rate_per_hour")]
    pub rate_limit_per_hour: u32,
    #[serde(default = "default_dedup_window")]
    pub dedup_window_secs: u64,
    #[serde(default = "default_group_window")]
    pub group_window_secs: u64,
    #[serde(default = "default_true")]
    pub recovery_enabled: bool,
    #[serde(default)]
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
    #[serde(default)]
    pub email: Option<EmailConfig>,
    #[serde(default)]
    pub webhook: Option<WebhookConfig>,
    #[serde(default)]
    pub syslog: Option<SyslogConfig>,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 10,
            rate_limit_per_hour: 60,
            dedup_window_secs: 300,
            group_window_secs: 30,
            recovery_enabled: true,
            discord: None,
            slack: None,
            telegram: None,
            email: None,
            webhook: None,
            syslog: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    pub webhook_url: String,
    pub username: Option<String>,
    #[serde(default)]
    pub severity_filter: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlackConfig {
    #[serde(default)]
    pub enabled: bool,
    pub webhook_url: String,
    pub channel: Option<String>,
    #[serde(default)]
    pub severity_filter: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    #[serde(default)]
    pub severity_filter: Vec<String>,
    #[serde(default = "default_parse_mode")]
    pub parse_mode: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: Vec<String>,
    #[serde(default)]
    pub severity_filter: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    pub url: String,
    #[serde(default = "default_post")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub severity_filter: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SyslogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_facility")]
    pub facility: String,
    #[serde(default)]
    pub severity_filter: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PrometheusConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_prom_bind")]
    pub bind: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self { enabled: false, bind: "127.0.0.1:9100".to_string() }
    }
}

#[cfg(feature = "nats")]
#[derive(Debug, Deserialize, Clone)]
pub struct NatsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_nats_url")]
    pub url: String,
    #[serde(default = "default_nats_prefix")]
    pub subject_prefix: String,
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval_secs: u64,
    #[serde(default = "default_inventory_interval")]
    pub inventory_interval_secs: u64,
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    #[serde(default)]
    pub compression: bool,
    #[serde(default)]
    pub credential_file: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[cfg(feature = "nats")]
impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_nats_url(),
            subject_prefix: default_nats_prefix(),
            metrics_interval_secs: 30,
            inventory_interval_secs: 300,
            heartbeat_interval_secs: 60,
            compression: false,
            credential_file: None,
            token: None,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        // Expand environment variables
        let expanded = expand_env_vars(&content);

        let config: Config = toml::from_str(&expanded)
            .with_context(|| "Failed to parse configuration")?;

        Ok(config)
    }
}

/// Expand ${ENV_VAR} references in config string
fn expand_env_vars(input: &str) -> String {
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).unwrap_or_default()
    })
    .to_string()
}

// Default value functions
fn default_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
fn default_log_level() -> String { "info".to_string() }
fn default_data_dir() -> String { "/var/lib/sysops-agent".to_string() }
fn default_proc_root() -> String { "/proc".to_string() }
fn default_sys_root() -> String { "/sys".to_string() }
fn default_interval() -> u64 { 10 }
fn default_disk_interval() -> u64 { 60 }
fn default_process_interval() -> u64 { 30 }
fn default_true() -> bool { true }
fn default_top_n() -> u32 { 20 }
fn default_log_sources() -> Vec<String> { vec!["dmesg".into(), "syslog".into()] }
fn default_exclude_fstypes() -> Vec<String> {
    vec!["tmpfs".into(), "devtmpfs".into(), "sysfs".into(), "proc".into()]
}
fn default_exclude_interfaces() -> Vec<String> { vec!["lo".into()] }
fn default_80() -> f64 { 80.0 }
fn default_90() -> f64 { 90.0 }
fn default_95() -> f64 { 95.0 }
fn default_zscore_window() -> u32 { 360 }
fn default_zscore_threshold() -> f64 { 3.0 }
fn default_ema_alpha() -> f64 { 0.1 }
fn default_trend_window() -> u32 { 6 }
fn default_leak_observation() -> u32 { 30 }
fn default_r_squared() -> f64 { 0.8 }
fn default_ring_buffer_size() -> u32 { 8640 }
fn default_retention_days() -> u32 { 30 }
fn default_rate_per_min() -> u32 { 10 }
fn default_rate_per_hour() -> u32 { 60 }
fn default_dedup_window() -> u64 { 300 }
fn default_group_window() -> u64 { 30 }
fn default_parse_mode() -> String { "HTML".to_string() }
fn default_smtp_port() -> u16 { 587 }
fn default_post() -> String { "POST".to_string() }
fn default_facility() -> String { "daemon".to_string() }
fn default_prom_bind() -> String { "127.0.0.1:9100".to_string() }
#[cfg(feature = "nats")]
fn default_nats_url() -> String { "nats://localhost:4222".to_string() }
#[cfg(feature = "nats")]
fn default_nats_prefix() -> String { "sysops".to_string() }
#[cfg(feature = "nats")]
fn default_metrics_interval() -> u64 { 30 }
#[cfg(feature = "nats")]
fn default_inventory_interval() -> u64 { 300 }
#[cfg(feature = "nats")]
fn default_heartbeat_interval() -> u64 { 60 }
