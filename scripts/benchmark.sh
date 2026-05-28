#!/usr/bin/env bash
# benchmark.sh - Build time benchmark for jcode
set -e

echo "=== jcode Build Time Benchmark ==="
echo "Date: $(date)"
echo ""

# Cold build
echo "[1] Cold build..."
START=$(date +%s)
cargo build -p jcode --lib 2>&1 | tail -1
END=$(date +%s)
COLD=$((END - START))
echo "Cold build: ${COLD}s"

# Warm build
echo ""
echo "[2] Warm build (should use sccache)..."
START=$(date +%s)
cargo build -p jcode --lib 2>&1 | tail -1
END=$(date +%s)
WARM=$((END - START))
echo "Warm build: ${WARM}s"

# sccache stats
echo ""
echo "[3] sccache stats..."
sccache --show-stats
