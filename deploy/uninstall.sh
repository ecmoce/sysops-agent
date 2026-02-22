#!/usr/bin/env bash
set -euo pipefail

# SysOps Agent — Uninstall Script

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }

if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}[✗]${NC} Run as root (sudo)" >&2
    exit 1
fi

# Stop and disable service
if systemctl is-active sysops-agent &>/dev/null; then
    log "Stopping sysops-agent service"
    systemctl stop sysops-agent
fi
if systemctl is-enabled sysops-agent &>/dev/null; then
    log "Disabling sysops-agent service"
    systemctl disable sysops-agent
fi

# Remove files
log "Removing binary"
rm -f /usr/local/bin/sysops-agent

log "Removing systemd service"
rm -f /etc/systemd/system/sysops-agent.service
systemctl daemon-reload 2>/dev/null || true

# Ask about config and data
read -p "Remove config (/etc/sysops-agent)? [y/N] " -r
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf /etc/sysops-agent
    log "Config removed"
fi

read -p "Remove data (/var/lib/sysops-agent)? [y/N] " -r
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf /var/lib/sysops-agent
    log "Data removed"
fi

# Remove user
if id sysops-agent &>/dev/null; then
    read -p "Remove sysops-agent user? [y/N] " -r
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        userdel sysops-agent 2>/dev/null || true
        log "User removed"
    fi
fi

echo ""
echo -e "${GREEN}SysOps Agent uninstalled.${NC}"
