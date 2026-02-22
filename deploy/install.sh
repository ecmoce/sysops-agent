#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────
# SysOps Agent — Installation Script
# Supports: Ubuntu 20.04+, Rocky 8+, CentOS 7+
# Usage: sudo ./install.sh [--nats-url nats://server:4222]
# ─────────────────────────────────────────────────────────

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/sysops-agent"
DATA_DIR="/var/lib/sysops-agent"
SERVICE_USER="sysops-agent"
SERVICE_GROUP="sysops-agent"
BINARY_NAME="sysops-agent"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log()  { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[✗]${NC} $*" >&2; }
info() { echo -e "${BLUE}[i]${NC} $*"; }

# ── Parse arguments ──
NATS_URL="nats://localhost:4222"
BINARY_PATH=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --nats-url)   NATS_URL="$2"; shift 2 ;;
        --binary)     BINARY_PATH="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: sudo $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --nats-url URL    NATS server URL (default: nats://localhost:4222)"
            echo "  --binary PATH     Path to pre-built binary (skip build)"
            echo "  -h, --help        Show this help"
            exit 0
            ;;
        *) err "Unknown option: $1"; exit 1 ;;
    esac
done

# ── Root check ──
if [[ $EUID -ne 0 ]]; then
    err "This script must be run as root (use sudo)"
    exit 1
fi

# ── Detect OS ──
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        echo "$ID"
    elif [[ -f /etc/centos-release ]]; then
        echo "centos"
    else
        echo "unknown"
    fi
}

OS=$(detect_os)
log "Detected OS: $OS"

# ── Create service user ──
if ! id "$SERVICE_USER" &>/dev/null; then
    log "Creating service user: $SERVICE_USER"
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
else
    info "Service user $SERVICE_USER already exists"
fi

# ── Install binary ──
if [[ -n "$BINARY_PATH" ]]; then
    log "Installing binary from: $BINARY_PATH"
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
elif [[ -f "$SCRIPT_DIR/../target/release/$BINARY_NAME" ]]; then
    log "Installing binary from build output"
    cp "$SCRIPT_DIR/../target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
else
    err "No binary found. Build first with: cargo build --release --features nats"
    err "Or provide path with: --binary /path/to/sysops-agent"
    exit 1
fi

chmod 755 "$INSTALL_DIR/$BINARY_NAME"
log "Binary installed: $INSTALL_DIR/$BINARY_NAME"

# ── Verify binary ──
if ! "$INSTALL_DIR/$BINARY_NAME" --version &>/dev/null; then
    # Some binaries might not support --version, try --help
    if ! "$INSTALL_DIR/$BINARY_NAME" --help &>/dev/null; then
        warn "Binary verification failed (may still work)"
    fi
fi

# ── Create directories ──
log "Creating directories"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"

# ── Install config ──
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    log "Installing default config"
    if [[ -f "$SCRIPT_DIR/config.toml" ]]; then
        cp "$SCRIPT_DIR/config.toml" "$CONFIG_DIR/config.toml"
    else
        cat > "$CONFIG_DIR/config.toml" <<EOF
[agent]
log_level = "info"
data_dir = "$DATA_DIR"

[collector]
default_interval_secs = 10

[collector.cpu]
enabled = true

[collector.memory]
enabled = true

[collector.disk]
enabled = true
interval_secs = 30

[collector.network]
enabled = true

[collector.process]
enabled = true
interval_secs = 30

[collector.log]
enabled = true
sources = ["dmesg", "syslog"]

[thresholds]
cpu_warn_percent = 90.0
cpu_critical_percent = 95.0
memory_warn_percent = 85.0
memory_critical_percent = 95.0
disk_warn_percent = 90.0
disk_critical_percent = 95.0

[storage]
ring_buffer_size = 8640

[alerting]
rate_limit_per_minute = 10
rate_limit_per_hour = 60

[nats]
enabled = true
url = "$NATS_URL"
subject_prefix = "sysops"
metrics_interval_secs = 30
inventory_interval_secs = 300
heartbeat_interval_secs = 60
compression = false
EOF
    fi
    # Update NATS URL in config
    sed -i "s|url = \"nats://.*\"|url = \"$NATS_URL\"|" "$CONFIG_DIR/config.toml"
else
    warn "Config already exists, skipping (update manually if needed)"
fi

# ── Set permissions ──
log "Setting permissions"
chmod 600 "$CONFIG_DIR/config.toml"
chown -R "$SERVICE_USER:$SERVICE_GROUP" "$CONFIG_DIR"
chown -R "$SERVICE_USER:$SERVICE_GROUP" "$DATA_DIR"

# ── Install systemd service ──
if command -v systemctl &>/dev/null; then
    log "Installing systemd service"
    if [[ -f "$SCRIPT_DIR/sysops-agent.service" ]]; then
        cp "$SCRIPT_DIR/sysops-agent.service" /etc/systemd/system/
    else
        cat > /etc/systemd/system/sysops-agent.service <<EOF
[Unit]
Description=SysOps Agent - System Monitoring Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $CONFIG_DIR/config.toml
Restart=always
RestartSec=10
User=$SERVICE_USER
Group=$SERVICE_GROUP
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
NoNewPrivileges=yes
ReadOnlyPaths=/proc /sys /var/log
ReadWritePaths=$DATA_DIR
CapabilityBoundingSet=CAP_DAC_READ_SEARCH CAP_SYSLOG
AmbientCapabilities=CAP_DAC_READ_SEARCH CAP_SYSLOG
MemoryMax=128M
StandardOutput=journal
StandardError=journal
SyslogIdentifier=sysops-agent

[Install]
WantedBy=multi-user.target
EOF
    fi
    systemctl daemon-reload
    log "Systemd service installed"
    info "Start with: sudo systemctl start sysops-agent"
    info "Enable on boot: sudo systemctl enable sysops-agent"
else
    warn "systemd not found, skipping service installation"
fi

# ── Summary ──
echo ""
echo -e "${GREEN}════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SysOps Agent installed successfully!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════${NC}"
echo ""
echo "  Binary:  $INSTALL_DIR/$BINARY_NAME"
echo "  Config:  $CONFIG_DIR/config.toml"
echo "  Data:    $DATA_DIR"
echo "  User:    $SERVICE_USER"
echo "  NATS:    $NATS_URL"
echo ""
echo "  Commands:"
echo "    sudo systemctl start sysops-agent"
echo "    sudo systemctl enable sysops-agent"
echo "    sudo systemctl status sysops-agent"
echo "    journalctl -u sysops-agent -f"
echo ""
