#![cfg(feature = "nats")]

use serde_json::{json, Value};
use tracing::warn;

/// Collect system inventory information
pub fn collect_inventory(proc_root: &str) -> (Value, Value) {
    let hardware = collect_hardware(proc_root);
    let software = collect_software();
    (hardware, software)
}

fn collect_hardware(proc_root: &str) -> Value {
    let cpu = collect_cpu_info(proc_root);
    let memory = collect_memory_info(proc_root);
    let disks = collect_disk_info();
    let network = collect_network_info();

    json!({
        "cpu": cpu,
        "memory": memory,
        "disks": disks,
        "network": network,
    })
}

fn collect_software() -> Value {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let kernel = read_file_trimmed("/proc/version")
        .unwrap_or_default();

    let os = read_os_release();

    json!({
        "hostname": hostname,
        "kernel": kernel,
        "os": os,
    })
}

fn collect_cpu_info(proc_root: &str) -> Value {
    let path = format!("{}/cpuinfo", proc_root);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Failed to read cpuinfo");
            return json!({});
        }
    };

    let mut model_name = String::new();
    let mut cores = 0u32;
    let mut cpu_mhz = String::new();

    for line in content.lines() {
        if line.starts_with("model name") {
            if let Some(val) = line.split(':').nth(1) {
                model_name = val.trim().to_string();
            }
        } else if line.starts_with("processor") {
            cores += 1;
        } else if line.starts_with("cpu MHz") {
            if cpu_mhz.is_empty() {
                if let Some(val) = line.split(':').nth(1) {
                    cpu_mhz = val.trim().to_string();
                }
            }
        }
    }

    json!({
        "model": model_name,
        "cores": cores,
        "mhz": cpu_mhz,
        "arch": std::env::consts::ARCH,
    })
}

fn collect_memory_info(proc_root: &str) -> Value {
    let path = format!("{}/meminfo", proc_root);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Failed to read meminfo");
            return json!({});
        }
    };

    let mut total_kb = 0u64;
    let mut swap_total_kb = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = parse_meminfo_value(line);
        } else if line.starts_with("SwapTotal:") {
            swap_total_kb = parse_meminfo_value(line);
        }
    }

    json!({
        "total_bytes": total_kb * 1024,
        "swap_total_bytes": swap_total_kb * 1024,
    })
}

fn parse_meminfo_value(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn collect_disk_info() -> Value {
    // List block devices from /sys/block
    let mut disks = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip virtual devices
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
                continue;
            }
            let size_path = format!("/sys/block/{}/size", name);
            let size_sectors: u64 = read_file_trimmed(&size_path)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let size_bytes = size_sectors * 512;

            disks.push(json!({
                "name": name,
                "size_bytes": size_bytes,
            }));
        }
    }
    json!(disks)
}

fn collect_network_info() -> Value {
    let mut interfaces = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            let mac_path = format!("/sys/class/net/{}/address", name);
            let mac = read_file_trimmed(&mac_path).unwrap_or_default();
            let mtu_path = format!("/sys/class/net/{}/mtu", name);
            let mtu: u32 = read_file_trimmed(&mtu_path)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            interfaces.push(json!({
                "name": name,
                "mac": mac,
                "mtu": mtu,
            }));
        }
    }
    json!(interfaces)
}

fn read_os_release() -> Value {
    let content = match std::fs::read_to_string("/etc/os-release") {
        Ok(c) => c,
        Err(_) => return json!({}),
    };

    let mut id = String::new();
    let mut version = String::new();
    let mut pretty_name = String::new();

    for line in content.lines() {
        if line.starts_with("ID=") {
            id = line.trim_start_matches("ID=").trim_matches('"').to_string();
        } else if line.starts_with("VERSION_ID=") {
            version = line.trim_start_matches("VERSION_ID=").trim_matches('"').to_string();
        } else if line.starts_with("PRETTY_NAME=") {
            pretty_name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string();
        }
    }

    json!({
        "id": id,
        "version": version,
        "pretty_name": pretty_name,
    })
}

fn read_file_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}
