#!/usr/bin/env bash
# validate_sccache.sh - Validate sccache is working
set -e

echo "=== sccache Validation ==="

# Check installation
if ! sccache --version &>/dev/null; then
    echo "ERROR: sccache not installed"
    echo "Run: ./scripts/setup_sccache.sh"
    exit 1
fi

# Set env if not set
export RUSTC_WRAPPER="$(which sccache)"

# Run two builds
echo "[1] Build 1 (cold)..."
cargo build -p jcode --lib >/dev/null 2>&1

echo "[2] Build 2 (should hit cache)..."
cargo build -p jcode --lib >/dev/null 2>&1

# Check stats
echo ""
echo "[3] sccache stats..."
sccache --show-stats

# Verify cache hit
if sccache --show-stats 2>/dev/null | grep -q "Cache hits"; then
    echo ""
    echo "✓ sccache is working"
else
    echo ""
    echo "⚠ Check sccache configuration"
fi
