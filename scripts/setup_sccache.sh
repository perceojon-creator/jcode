#!/usr/bin/env bash
# setup_sccache.sh - Install and configure sccache for Termux
set -e

SCCACHE="/data/data/com.termux/files/usr/bin/sccache"

echo "=== SCCache Setup ==="

if [ ! -f "$SCCACHE" ]; then
    echo "Installing sccache..."
    cargo install sccache
    SCCACHE="$(command -v sccache)"
fi

export RUSTC_WRAPPER="$SCCACHE"
export SCCACHE_GHA_ENABLED="true"
export SCCACHE_DIR="$HOME/.cache/sccache"
export SCCACHE_CACHE_SIZE="10G"

echo "RUSTC_WRAPPER=$RUSTC_WRAPPER"
$SCCACHE --start-server 2>/dev/null || true

echo ""
echo "=== sccache status ==="
$SCCACHE --show-stats 2>/dev/null | head -10
