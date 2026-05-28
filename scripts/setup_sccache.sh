#!/usr/bin/env bash
# setup_sccache.sh - Install and configure sccache for Rust compilation caching
set -e

echo "=== SCCache Setup Script ==="
echo "Milestone 1: Build Speed Optimization"
echo ""

# Check if sccache is installed
if command -v sccache &> /dev/null; then
    echo "[OK] sccache already installed: $(which sccache)"
    sccache --version
else
    echo "[INSTALL] sccache not found, installing..."
    cargo install sccache
fi

# Configure environment
mkdir -p ~/.cargo
export RUSTC_WRAPPER="$(which sccache)"
export SCCACHE_GHA_ENABLED="true"
export SCCACHE_DIR="${HOME}/.cache/sccache"
export SCCACHE_CACHE_SIZE="10G"

# Start sccache server
sccache --start-server 2>/dev/null || true

echo ""
echo "=== sccache Status ==="
sccache --show-stats
echo ""
echo "Add to your shell profile:"
echo "  export RUSTC_WRAPPER=\"$(which sccache)\""
