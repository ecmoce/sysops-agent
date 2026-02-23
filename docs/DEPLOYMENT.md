# Deployment Guide

## 1. systemd Unit File

```ini
# /etc/systemd/system/sysops-agent.service

[Unit]
Description=SysOps Agent - System Monitoring Daemon
Documentation=https://github.com/ecmoce/sysops-agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=sysops-agent
Group=sysops-agent
ExecStart=/usr/local/bin/sysops-agent --config /etc/sysops-agent/config.toml
Restart=on-failure
RestartSec=10
WatchdogSec=60

# Security Hardening
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
NoNewPrivileges=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
RestrictNamespaces=yes
RestrictRealtime=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
ReadOnlyPaths=/
ReadWritePaths=/var/lib/sysops-agent /var/log/sysops-agent

# Allow procfs access
ProtectProc=invisible
ProcSubset=all

# Resource Limits
MemoryMax=100M
CPUQuota=10%
TasksMax=32

# Capabilities
AmbientCapabilities=CAP_DAC_READ_SEARCH CAP_SYSLOG
CapabilityBoundingSet=CAP_DAC_READ_SEARCH CAP_SYSLOG

[Install]
WantedBy=multi-user.target
```

### Installation Script

```bash
#!/bin/bash
set -euo pipefail

# Create user
useradd --system --no-create-home --shell /sbin/nologin sysops-agent

# Create directories
mkdir -p /etc/sysops-agent /var/lib/sysops-agent /var/log/sysops-agent
chown sysops-agent:sysops-agent /var/lib/sysops-agent /var/log/sysops-agent

# Copy binary
cp sysops-agent /usr/local/bin/
chmod 755 /usr/local/bin/sysops-agent

# Copy config file (skip if already exists)
[ -f /etc/sysops-agent/config.toml ] || cp config.toml.example /etc/sysops-agent/config.toml
chmod 600 /etc/sysops-agent/config.toml
chown sysops-agent:sysops-agent /etc/sysops-agent/config.toml

# Set capabilities
setcap 'cap_dac_read_search,cap_syslog=ep' /usr/local/bin/sysops-agent

# Register with systemd
cp sysops-agent.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now sysops-agent
```

## 2. RPM Spec Skeleton (CentOS/Rocky)

```spec
# sysops-agent.spec
Name:           sysops-agent
Version:        0.1.0
Release:        1%{?dist}
Summary:        Lightweight system monitoring agent
License:        MIT
URL:            https://github.com/ecmoce/sysops-agent

Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo

%description
SysOps Agent is a lightweight system monitoring daemon for Linux servers.
Performs real-time anomaly detection, trend analysis, resource leak detection, and kernel log analysis.

%prep
%setup -q

%build
cargo build --release --target x86_64-unknown-linux-musl

%install
install -D -m 755 target/x86_64-unknown-linux-musl/release/%{name} %{buildroot}/usr/local/bin/%{name}
install -D -m 644 deploy/sysops-agent.service %{buildroot}/etc/systemd/system/%{name}.service
install -D -m 600 config.toml.example %{buildroot}/etc/%{name}/config.toml
mkdir -p %{buildroot}/var/lib/%{name}
mkdir -p %{buildroot}/var/log/%{name}

%pre
getent group sysops-agent >/dev/null || groupadd -r sysops-agent
getent passwd sysops-agent >/dev/null || useradd -r -g sysops-agent -s /sbin/nologin sysops-agent

%post
setcap 'cap_dac_read_search,cap_syslog=ep' /usr/local/bin/sysops-agent
systemctl daemon-reload
systemctl enable sysops-agent

%files
/usr/local/bin/%{name}
/etc/systemd/system/%{name}.service
%config(noreplace) /etc/%{name}/config.toml
%dir /var/lib/%{name}
%dir /var/log/%{name}
```

## 3. DEB Packaging (Ubuntu)

```
debian/
├── control
├── rules
├── postinst
├── prerm
└── sysops-agent.service
```

**debian/control:**
```
Package: sysops-agent
Version: 0.1.0
Section: admin
Priority: optional
Architecture: amd64
Maintainer: SysOps <admin@example.com>
Description: Lightweight system monitoring agent
 Linux server monitoring daemon written in Rust.
```

## 4. Ansible Playbook Skeleton

```yaml
# playbooks/deploy-sysops-agent.yml
---
- name: Deploy SysOps Agent
  hosts: monitored_servers
  become: yes
  vars:
    sysops_version: "0.1.0"
    sysops_config_template: "templates/config.toml.j2"

  tasks:
    - name: Create sysops-agent user
      user:
        name: sysops-agent
        system: yes
        create_home: no
        shell: /sbin/nologin

    - name: Create directories
      file:
        path: "{{ item }}"
        state: directory
        owner: sysops-agent
        group: sysops-agent
        mode: '0750'
      loop:
        - /etc/sysops-agent
        - /var/lib/sysops-agent
        - /var/log/sysops-agent

    - name: Download sysops-agent binary
      get_url:
        url: "https://github.com/ecmoce/sysops-agent/releases/download/v{{ sysops_version }}/sysops-agent-linux-amd64"
        dest: /usr/local/bin/sysops-agent
        mode: '0755'

    - name: Set capabilities
      community.general.capabilities:
        path: /usr/local/bin/sysops-agent
        capability: cap_dac_read_search,cap_syslog=ep

    - name: Deploy config
      template:
        src: "{{ sysops_config_template }}"
        dest: /etc/sysops-agent/config.toml
        owner: sysops-agent
        group: sysops-agent
        mode: '0600'
      notify: restart sysops-agent

    - name: Deploy systemd unit
      copy:
        src: files/sysops-agent.service
        dest: /etc/systemd/system/sysops-agent.service
      notify:
        - reload systemd
        - restart sysops-agent

    - name: Enable and start service
      systemd:
        name: sysops-agent
        enabled: yes
        state: started

  handlers:
    - name: reload systemd
      systemd:
        daemon_reload: yes

    - name: restart sysops-agent
      systemd:
        name: sysops-agent
        state: restarted
```

## 5. Docker (For Testing)

> ⚠️ **Docker is not recommended for production use.** Container access to host procfs/sysfs is limited.

```dockerfile
# Dockerfile
FROM rust:1.75-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.19
RUN adduser -D -H -s /sbin/nologin sysops-agent
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/sysops-agent /usr/local/bin/
USER sysops-agent
ENTRYPOINT ["sysops-agent"]
CMD ["--config", "/etc/sysops-agent/config.toml"]
```

```bash
# Test run (requires host procfs mount)
docker run -d \
  --name sysops-agent \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  -v ./config.toml:/etc/sysops-agent/config.toml:ro \
  -e PROC_ROOT=/host/proc \
  sysops-agent:latest
```