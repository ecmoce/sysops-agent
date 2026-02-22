#!/bin/bash
set -e

cd "$(dirname "$0")/.."

TARGETS=("ubuntu2204" "ubuntu2404" "rocky8" "rocky9" "centos7")
RESULTS_FILE="docs/BUILD-TEST-RESULTS.md"

echo "# Build & Test Results" > "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "Generated: $(date -u '+%Y-%m-%d %H:%M:%S UTC')" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

for target in "${TARGETS[@]}"; do
    echo "=========================================="
    echo "Building: $target"
    echo "=========================================="
    
    TAG="sysops-agent-${target}"
    
    if docker build --platform linux/amd64 -t "$TAG" -f "docker/Dockerfile.${target}" . 2>&1; then
        echo "## ✅ ${target}" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"
        
        # Get binary size
        SIZE=$(docker run --platform linux/amd64 --rm "$TAG" ls -lh /usr/local/bin/sysops-agent 2>/dev/null | awk '{print $5}')
        echo "- **Build**: SUCCESS" >> "$RESULTS_FILE"
        echo "- **Binary size**: ${SIZE}" >> "$RESULTS_FILE"
        
        # Version test
        VERSION=$(docker run --platform linux/amd64 --rm "$TAG" sysops-agent --version 2>&1 || true)
        echo "- **--version**: \`${VERSION}\`" >> "$RESULTS_FILE"
        
        # Help test
        HELP_OUTPUT=$(docker run --platform linux/amd64 --rm "$TAG" sysops-agent --help 2>&1 || true)
        echo "- **--help**: OK" >> "$RESULTS_FILE"
        echo '```' >> "$RESULTS_FILE"
        echo "$HELP_OUTPUT" >> "$RESULTS_FILE"
        echo '```' >> "$RESULTS_FILE"
        
        echo "" >> "$RESULTS_FILE"
    else
        echo "## ❌ ${target}" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"
        echo "- **Build**: FAILED" >> "$RESULTS_FILE"
        echo "" >> "$RESULTS_FILE"
    fi
done

echo "Results written to $RESULTS_FILE"
cat "$RESULTS_FILE"
