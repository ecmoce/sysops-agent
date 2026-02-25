#![cfg(feature = "nats")]

use anyhow::Result;
use async_nats::Client;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn, error};

use crate::storage::Storage;

#[derive(Deserialize)]
struct ExecRequest {
    command: String,
}

#[derive(Serialize)]
struct ExecResponse {
    command: String,
    stdout: String,
    stderr: String,
    exit_code: i32,
    duration_ms: u64,
}

#[derive(Serialize)]
struct SnapshotResponse {
    hostname: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    cpu: CpuSnapshot,
    memory: MemorySnapshot,
    disk: Vec<DiskSnapshot>,
    network: Vec<NetSnapshot>,
    processes: Vec<ProcessSnapshot>,
    load: LoadSnapshot,
    uptime_secs: Option<u64>,
    recent_logs: Vec<String>,
}

#[derive(Serialize)]
struct CpuSnapshot {
    usage_percent: f64,
    iowait_percent: f64,
    core_count: usize,
}

#[derive(Serialize)]
struct MemorySnapshot {
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
    usage_percent: f64,
    swap_usage_percent: f64,
}

#[derive(Serialize)]
struct DiskSnapshot {
    mount: String,
    device: String,
    total_bytes: u64,
    used_bytes: u64,
    usage_percent: f64,
}

#[derive(Serialize)]
struct NetSnapshot {
    interface: String,
    rx_bytes_rate: f64,
    tx_bytes_rate: f64,
    rx_errors: u64,
    tx_errors: u64,
}

#[derive(Serialize)]
struct ProcessSnapshot {
    pid: u32,
    name: String,
    cpu_percent: f64,
    rss_bytes: u64,
    state: String,
}

#[derive(Serialize)]
struct LoadSnapshot {
    load_1m: f64,
    load_5m: f64,
    load_15m: f64,
}

/// Start NATS request-reply handlers for snapshot and exec
pub fn start_handlers(
    client: Client,
    hostname: String,
    prefix: String,
    storage: Storage,
) {
    // Snapshot handler
    {
        let client = client.clone();
        let hostname = hostname.clone();
        let subject = format!("{}.{}.snapshot", prefix, hostname);
        let storage = storage.clone();
        tokio::spawn(async move {
            let mut sub = match client.subscribe(subject.clone()).await {
                Ok(s) => s,
                Err(e) => { error!(error=%e, "Failed to subscribe to snapshot"); return; }
            };
            info!(subject=%subject, "Listening for snapshot requests");

            while let Some(msg) = sub.next().await {
                let snapshot = collect_snapshot(&hostname, &storage).await;
                let payload = serde_json::to_vec(&snapshot).unwrap_or_default();
                if let Some(reply) = msg.reply {
                    if let Err(e) = client.publish(reply, payload.into()).await {
                        error!(error=%e, "Failed to reply with snapshot");
                    }
                }
            }
        });
    }

    // Exec handler
    {
        let client = client.clone();
        let hostname = hostname.clone();
        let subject = format!("{}.{}.exec", prefix, hostname);
        tokio::spawn(async move {
            let mut sub = match client.subscribe(subject.clone()).await {
                Ok(s) => s,
                Err(e) => { error!(error=%e, "Failed to subscribe to exec"); return; }
            };
            info!(subject=%subject, "Listening for exec requests");

            while let Some(msg) = sub.next().await {
                let response = match serde_json::from_slice::<ExecRequest>(&msg.payload) {
                    Ok(req) => execute_command(&req.command).await,
                    Err(_) => ExecResponse {
                        command: String::new(),
                        stdout: String::new(),
                        stderr: "Invalid request payload".into(),
                        exit_code: -1,
                        duration_ms: 0,
                    },
                };
                let payload = serde_json::to_vec(&response).unwrap_or_default();
                if let Some(reply) = msg.reply {
                    if let Err(e) = client.publish(reply, payload.into()).await {
                        error!(error=%e, "Failed to reply with exec result");
                    }
                }
            }
        });
    }
}

async fn execute_command(cmd: &str) -> ExecResponse {
    let start = std::time::Instant::now();

    // Safety: run via sh -c with a timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(55),
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output(),
    ).await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => ExecResponse {
            command: cmd.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).chars().take(10_000).collect(),
            stderr: String::from_utf8_lossy(&output.stderr).chars().take(10_000).collect(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        },
        Ok(Err(e)) => ExecResponse {
            command: cmd.to_string(),
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
            exit_code: -1,
            duration_ms,
        },
        Err(_) => ExecResponse {
            command: cmd.to_string(),
            stdout: String::new(),
            stderr: "Command timed out (55s)".into(),
            exit_code: -1,
            duration_ms,
        },
    }
}

async fn collect_snapshot(hostname: &str, _storage: &Storage) -> SnapshotResponse {
    let cpu = read_cpu_snapshot().unwrap_or(CpuSnapshot { usage_percent: 0.0, iowait_percent: 0.0, core_count: 0 });
    let memory = read_memory_snapshot().unwrap_or(MemorySnapshot { total_bytes: 0, used_bytes: 0, available_bytes: 0, usage_percent: 0.0, swap_usage_percent: 0.0 });
    let disk = read_disk_snapshot().unwrap_or_default();
    let load = read_load_snapshot().unwrap_or(LoadSnapshot { load_1m: 0.0, load_5m: 0.0, load_15m: 0.0 });
    let processes = read_top_processes().unwrap_or_default();
    let network = read_network_snapshot().unwrap_or_default();
    let uptime = read_uptime();
    let recent_logs = read_recent_logs();

    SnapshotResponse {
        hostname: hostname.to_string(),
        timestamp: chrono::Utc::now(),
        cpu,
        memory,
        disk,
        network,
        processes,
        load,
        uptime_secs: uptime,
        recent_logs,
    }
}

fn read_cpu_snapshot() -> Option<CpuSnapshot> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let first_line = stat.lines().next()?;
    let fields: Vec<u64> = first_line.split_whitespace().skip(1)
        .filter_map(|s| s.parse().ok()).collect();
    if fields.len() < 8 { return None; }
    let total: u64 = fields.iter().sum();
    let idle = fields[3] + fields[4]; // idle + iowait
    let usage = if total > 0 { 100.0 * (1.0 - idle as f64 / total as f64) } else { 0.0 };
    let iowait = if total > 0 { 100.0 * fields[4] as f64 / total as f64 } else { 0.0 };
    let core_count = stat.lines().filter(|l| l.starts_with("cpu") && l.len() > 3 && l.chars().nth(3).map_or(false, |c| c.is_ascii_digit())).count();

    Some(CpuSnapshot { usage_percent: usage, iowait_percent: iowait, core_count })
}

fn read_memory_snapshot() -> Option<MemorySnapshot> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = 0u64;
    let mut available = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in meminfo.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let val: u64 = parts[1].parse().unwrap_or(0) * 1024; // kB to bytes
        match parts[0] {
            "MemTotal:" => total = val,
            "MemAvailable:" => available = val,
            "SwapTotal:" => swap_total = val,
            "SwapFree:" => swap_free = val,
            _ => {}
        }
    }

    let used = total.saturating_sub(available);
    let usage_percent = if total > 0 { 100.0 * used as f64 / total as f64 } else { 0.0 };
    let swap_usage = if swap_total > 0 { 100.0 * (swap_total - swap_free) as f64 / swap_total as f64 } else { 0.0 };

    Some(MemorySnapshot { total_bytes: total, used_bytes: used, available_bytes: available, usage_percent, swap_usage_percent: swap_usage })
}

fn read_disk_snapshot() -> Option<Vec<DiskSnapshot>> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    let mut disks = Vec::new();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 { continue; }
        let device = parts[0];
        let mount = parts[1];
        let fstype = parts[2];

        // Skip virtual filesystems
        if !device.starts_with('/') { continue; }
        if ["tmpfs", "devtmpfs", "sysfs", "proc", "squashfs"].contains(&fstype) { continue; }

        // Use statvfs
        if let Ok(output) = std::process::Command::new("df").arg("-B1").arg(mount).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(data_line) = text.lines().nth(1) {
                let fields: Vec<&str> = data_line.split_whitespace().collect();
                if fields.len() >= 5 {
                    let total: u64 = fields[1].parse().unwrap_or(0);
                    let used: u64 = fields[2].parse().unwrap_or(0);
                    let usage = if total > 0 { 100.0 * used as f64 / total as f64 } else { 0.0 };
                    disks.push(DiskSnapshot { mount: mount.to_string(), device: device.to_string(), total_bytes: total, used_bytes: used, usage_percent: usage });
                }
            }
        }
    }

    Some(disks)
}

fn read_load_snapshot() -> Option<LoadSnapshot> {
    let loadavg = std::fs::read_to_string("/proc/loadavg").ok()?;
    let parts: Vec<f64> = loadavg.split_whitespace().take(3)
        .filter_map(|s| s.parse().ok()).collect();
    if parts.len() < 3 { return None; }
    Some(LoadSnapshot { load_1m: parts[0], load_5m: parts[1], load_15m: parts[2] })
}

fn read_top_processes() -> Option<Vec<ProcessSnapshot>> {
    // Use ps to get top 20 by CPU
    let output = std::process::Command::new("ps")
        .args(["--no-headers", "-eo", "pid,comm,%cpu,rss,stat", "--sort=-%cpu"])
        .output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    let procs: Vec<ProcessSnapshot> = text.lines().take(20).filter_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 { return None; }
        Some(ProcessSnapshot {
            pid: fields[0].parse().unwrap_or(0),
            name: fields[1].to_string(),
            cpu_percent: fields[2].parse().unwrap_or(0.0),
            rss_bytes: fields[3].parse::<u64>().unwrap_or(0) * 1024,
            state: fields[4].to_string(),
        })
    }).collect();

    Some(procs)
}

fn read_network_snapshot() -> Option<Vec<NetSnapshot>> {
    let net_dev = std::fs::read_to_string("/proc/net/dev").ok()?;
    let nets: Vec<NetSnapshot> = net_dev.lines().skip(2).filter_map(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 { return None; }
        let iface = parts[0].trim_end_matches(':');
        if iface == "lo" { return None; }
        Some(NetSnapshot {
            interface: iface.to_string(),
            rx_bytes_rate: parts[1].parse().unwrap_or(0.0),
            tx_bytes_rate: parts[9].parse().unwrap_or(0.0),
            rx_errors: parts[3].parse().unwrap_or(0),
            tx_errors: parts[11].parse().unwrap_or(0),
        })
    }).collect();
    Some(nets)
}

fn read_uptime() -> Option<u64> {
    std::fs::read_to_string("/proc/uptime").ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v as u64)
}

fn read_recent_logs() -> Vec<String> {
    // Try journalctl for last 50 lines of errors/warnings
    if let Ok(output) = std::process::Command::new("journalctl")
        .args(["--no-pager", "-p", "warning", "-n", "50", "--output=short-iso"])
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(String::from)
                .collect();
        }
    }

    // Fallback: tail syslog
    if let Ok(output) = std::process::Command::new("tail")
        .args(["-n", "50", "/var/log/syslog"])
        .output()
    {
        return String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();
    }

    vec![]
}
