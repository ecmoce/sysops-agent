# Changelog

All notable changes to sysops-agent will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).

## [0.1.0] - 2024-01-15

### Added
- Multi-socket CPU, NVIDIA GPU, and system inventory collectors
- NATS publisher for metrics, alerts, inventory, and heartbeat
- LLM-powered health check with admin approval workflow
- Threshold, trend, and z-score alert analyzers
- Discord, Slack, and webhook alert channels
- Log analyzer with pattern matching
- systemd service, install/uninstall deploy scripts
- Dockerfile for containerized deployment
- CI workflow with Trivy security scanning
- Comprehensive documentation (DESIGN, METRICS, ALERTING, DEPLOYMENT, CONFIGURATION)

### Fixed
- Command injection protection in exec handler
- CPU iowait calculation accuracy
- futures-util dependency for NATS feature
- Docker build compatibility
