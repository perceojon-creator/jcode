# FASE 2: Build Speed Optimization - Implementation Plan
## jcode Performance Engineering

**Version:** 2.0  
**Date:** 2026-05-28  
**Author:** Senior Performance Engineering  
**Status:** READY FOR IMPLEMENTATION  
**Target:** Warm cargo check <5s, Warm selfdev build <30s

---

## TABLE OF CONTENTS

1. [Executive Summary](#1-executive-summary)
2. [Baseline Measurements](#2-baseline-measurements)
3. [Implementation Steps](#3-implementation-steps)
4. [Testing Strategy](#4-testing-strategy)
5. [Profiling & Debugging](#5-profiling--debugging)
6. [Validation Checklist](#6-validation-checklist)
7. [Rollback Procedures](#7-rollback-procedures)
8. [Milestones](#8-milestones)

---

## 1. EXECUTIVE SUMMARY

### 1.1 Current State

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Warm `cargo check` | ~8.5s | <5s | -41% |
| Warm selfdev build | ~47s | <30s | -36% |
| Cold build | ~5min | <4min | -20% |
| Cache hit rate | 0% | >80% | NEW CAPABILITY |

### 1.2 Bottlenecks Identified

```
Priority 1: sccache not configured (0% cache)
Priority 2: src/agent.rs (914 lines, 30.9s compile)
Priority 3: src/tool/browser.rs (931 lines, 13.7s compile)
Priority 4: Feature flags (embeddings + pdf always compiled)
Priority 5: Linker not optimized for AArch64
```

### 1.3 Implementation Phases

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 2.1 | 2-3 days | sccache setup + profiling baseline |
| 2.2 | 1 week | Cargo profile optimization |
| 2.3 | 1 week | agent.rs modularization |
| 2.4 | 1 week | Feature flag optimization |
| 2.5 | 1 week | Integration + validation |

**Total: ~4 weeks**

---

## 2. BASELINE MEASUREMENTS

### 2.1 Capture Current State (BEFORE ANY CHANGES)

```bash
# Navigate to project
cd ~/jcode

# Create baseline metrics file
cat > .jcode/plan/build_baseline.md << 'EOF'
# Build Baseline Metrics (Created: 2026-05-28)

## System Info
- CPU: $(nproc) cores
- RAM: $(free -h | grep Mem | awk '{print $2}')
- OS: $(uname -s)
- Rust: $(rustc --version)
- Cargo: $(cargo --version)

## Baseline Measurements

### Cold Build (clean)
```bash
cargo clean
time cargo build --release -p jcode 2>&1 | tail -5
```
Expected: ~5 minutes

### Warm Check (2nd run)
```bash
cargo check 2>&1 | grep "Finished"
```
Expected: ~8.5 seconds

### Selfdev Profile Build
```bash
cargo build --profile selfdev -p jcode 2>&1 | grep "Finished"
```
Expected: ~47 seconds

### Touched File Recompile
```bash
# Touch agent.rs and measure
touch src/agent.rs
time cargo check -p jcode --lib 2>&1 | tail -3
```
Expected: ~30 seconds
EOF

# Execute baseline capture
bash .jcode/plan/build_baseline.md 2>/dev/null || echo "Baseline capture script created"
```

### 2.2 Profiling Setup

```bash
# Install profiling tools
cargo install cargo-build-timings 2>/dev/null || true
cargo install flamegraph 2>/dev/null || true

# Enable timing in builds
export CARGO_PROFILE_DEV_DEBUG=1
export CARGO_LOG_BUILD_TIMINGS=1

# Profile a build
touch src/agent.rs
cargo build --timings 2>&1 | tail -10
```

### 2.3 Identify Hot Files

```bash
# Find files causing most recompilation
echo "=== Hot Files Analysis ==="
find src -name "*.rs" -exec sh -c '
  touch "$1"
  START=$(date +%s%N)
  cargo check -p jcode --lib 2>/dev/null | grep -q "Compiling" && RESULT="YES" || RESULT="NO"
  END=$(date +%s%N)
  echo "$1: $(($((END - START)) / 1000000))ms (recompiled: $RESULT)"
' _ {} \; 2>/dev/null | sort -t: -k2 -n | head -20
```

---

## 3. IMPLEMENTATION STEPS

### STEP 1: Setup sccache (Day 1)

#### 1.1 Install sccache

```bash
# Install via cargo (cross-platform)
cargo install sccache

# Or via system package manager
# Ubuntu/Debian: sudo apt install sccache
# macOS: brew install sccache
# Termux: pkg install sccache || cargo install sccache
```

#### 1.2 Configure sccache Environment

```bash
# Add to ~/.bashrc or ~/.zshrc
export RUSTC_WRAPPER="$(which sccache)"
export SCCACHE_GHA_ENABLED="true"  # GitHub Actions cache integration
export SCCACHE_DIR="$HOME/.cache/sccache"
export SCCACHE_CACHE_SIZE="10G"

# Start sccache server
sccache --start-server 2>/dev/null || true

# Verify
sccache --show-stats
```

#### 1.3 Create Setup Script

```bash
# Create scripts/setup_sccache.sh
cat > scripts/setup_sccache.sh << 'SCRIPT'
#!/usr/bin/env bash
set -e

echo "=== sccache Setup for jcode ==="

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)
        if command -v apt-get &>/dev/null; then
            sudo apt-get update && sudo apt-get install -y sccache
        elif command -v dnf &>/dev/null; then
            sudo dnf install -y sccache
        else
            cargo install sccache
        fi
        ;;
    Darwin*)
        brew install sccache
        ;;
    Android*)
        pkg install sccache 2>/dev/null || cargo install sccache
        ;;
    *)
        cargo install sccache
        ;;
esac

# Configure
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << 'TOML'

[build]
rustc-wrapper = "{SCCACHE}"

[env]
SCCACHE_GHA_ENABLED = "true"
TOML

# Start server
sccache --start-server 2>/dev/null || true

echo "=== sccache Status ==="
sccache --show-stats
echo ""
echo "Add to shell profile:"
echo '  export RUSTC_WRAPPER="$(which sccache)"'
SCRIPT

chmod +x scripts/setup_sccache.sh
```

#### 1.4 Validate sccache

```bash
# Run setup
./scripts/setup_sccache.sh

# Test cache
echo "=== First Build (cold) ==="
cargo build -p jcode --lib 2>&1 | tail -3

echo ""
echo "=== Second Build (should hit cache) ==="
cargo build -p jcode --lib 2>&1 | tail -3

echo ""
echo "=== sccache Stats ==="
sccache --show-stats
```

**Expected Result:** Second build shows "Cache hits" > 0

---

### STEP 2: Optimize Cargo Profiles (Day 2-3)

#### 2.1 Backup Current Config

```bash
cp Cargo.toml Cargo.toml.backup.$(date +%Y%m%d)
cp .cargo/config.toml .cargo/config.toml.backup.$(date +%Y%m%d) 2>/dev/null || true
```

#### 2.2 Create Optimized .cargo/config.toml

```bash
cat > .cargo/config.toml << 'TOML'
# jcode Cargo Configuration
# Optimized for build speed

[build]
# Parallel compilation
jobs = 6

# Target-specific settings
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-Wl,--thinlto",
]

[target.aarch64-linux-android]
linker = "clang"
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-Wl,--thinlto",
]

# Profile: Development (cargo check, cargo test)
[profile.dev]
opt-level = 0
debug = 0
incremental = true
codegen-units = 16
split-debuginfo = "unpacked"

# Profile: Release builds (production)
[profile.release]
opt-level = 2
debug = 0
lto = "thin"
codegen-units = 32
incremental = false
panic = "abort"
strip = true

# Profile: Self-dev (cargo build --profile selfdev)
# Fast incremental builds for active development
[profile.selfdev]
inherits = "release"
opt-level = 0
debug = 0
lto = false
incremental = true
codegen-units = 1

# Profile: CI checks (cargo check without full features)
[profile.quickcheck]
inherits = "dev"
opt-level = 0
debug = 0
incremental = true
TOML

echo "✓ .cargo/config.toml updated"
```

#### 2.3 Optimize Cargo.toml Features

```bash
# Edit Cargo.toml - change default features
# BEFORE:
# default = ["pdf", "embeddings"]

# AFTER:
# default = []  # Features opt-in

# Add new features section if not exists
cat >> Cargo.toml << 'TOML'

# Optimized features (opt-in)
[features]
default = []
dev-bins = []
embeddings = ["dep:jcode-embedding"]
pdf = ["dep:jcode-pdf"]
quick = []  # For fast cargo check
full = ["embeddings", "pdf"]  # Full build
TOML
```

#### 2.4 Validate Profile Changes

```bash
echo "=== Testing Dev Profile ==="
time cargo check -p jcode --lib 2>&1 | tail -3

echo ""
echo "=== Testing Selfdev Profile ==="
time cargo build --profile selfdev -p jcode --lib 2>&1 | tail -3

echo ""
echo "=== Testing Quick Profile ==="
time cargo check --profile quickcheck -p jcode --lib 2>&1 | tail -3
```

---

### STEP 3: Modularize src/agent.rs (Week 2)

#### 3.1 Create Module Structure

```bash
# Create agent module directory
mkdir -p src/agent

# Move types to new module
cat > src/agent/types.rs << 'RUST'
//! Type definitions for agent module
//! Extracted to reduce compile times

use crate::bus::{Bus, BusEvent, SubagentStatus, ToolEvent, ToolStatus};
use crate::session::Session;
use crate::tool::Registry;

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Agent error types
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Session error: {0}")]
    Session(String),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// State snapshot for agent
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub turn_count: u64,
    pub last_turn_at: Option<std::time::Instant>,
}
RUST

# Move state management
cat > src/agent/state.rs << 'RUST'
//! Agent state management
//! Separated for faster incremental compilation

use std::sync::atomic::{AtomicU64, Ordering};

/// Agent execution state
pub struct AgentState {
    turn_count: AtomicU64,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            turn_count: AtomicU64::new(0),
        }
    }
    
    pub fn increment_turn(&self) -> u64 {
        self.turn_count.fetch_add(1, Ordering::Relaxed)
    }
    
    pub fn turn_count(&self) -> u64 {
        self.turn_count.load(Ordering::Relaxed)
    }
}
RUST

# Create module file
cat > src/agent/mod.rs << 'RUST'
//! Agent module - refactored for build speed
//! 
//! Split from src/agent.rs to enable parallel compilation
//! and faster incremental builds.

pub mod types;
pub mod state;

// Re-exports for backward compatibility
pub use types::{AgentError, AgentResult, StateSnapshot};
pub use state::AgentState;
RUST
```

#### 3.2 Update src/agent.rs

```bash
# At the top of src/agent.rs, add:
mod agent;

// Replace heavy imports with re-exports
use agent::types::*;
use agent::state::*;
```

#### 3.3 Test Modularization

```bash
echo "=== Test: Full Build ==="
time cargo build -p jcode --lib 2>&1 | tail -5

echo ""
echo "=== Test: Touch types.rs only ==="
touch src/agent/types.rs
time cargo check -p jcode --lib 2>&1 | grep -E "(Compiling|Finished)"

echo ""
echo "=== Test: Touch state.rs only ==="
touch src/agent/state.rs
time cargo check -p jcode --lib 2>&1 | grep -E "(Compiling|Finished)"
```

---

### STEP 4: Optimize Feature Flags (Week 3)

#### 4.1 Implement Feature Detection Script

```bash
cat > scripts/check_features.sh << 'SCRIPT'
#!/usr/bin/env bash
# Check which features are needed for current operation

echo "=== Feature Usage Analysis ==="
echo ""

# Count usage of embeddings-specific code
EMBED_USAGE=$(grep -r "jcode_embedding\|embedding::" src/ --include="*.rs" | wc -l)
echo "Embedding usage: $EMBED_USAGE occurrences"

# Count usage of PDF-specific code  
PDF_USAGE=$(grep -r "jcode_pdf\|pdf::\|Pdf" src/ --include="*.rs" | wc -l)
echo "PDF usage: $PDF_USAGE occurrences"

# Analyze dependencies
echo ""
echo "=== Dependency Analysis ==="
cargo tree -p jcode --edges normal --depth 1 2>/dev/null | grep -E "embedding|pdf" || echo "No heavy deps in default"
```

#### 4.2 Create Feature Toggle Script

```bash
cat > scripts/toggle_features.sh << 'SCRIPT'
#!/usr/bin/env bash
# Toggle feature flags for different workflows

FEATURE=${1:-dev}

case "$FEATURE" in
    dev)
        echo "Setting up for development (no heavy features)..."
        export CARGO_BUILD_FEATURES=""
        ;;
    embeddings)
        echo "Setting up with embeddings..."
        export CARGO_BUILD_FEATURES="--features embeddings"
        ;;
    full)
        echo "Setting up full build..."
        export CARGO_BUILD_FEATURES="--features full"
        ;;
    *)
        echo "Usage: $0 {dev|embeddings|full}"
        exit 1
        ;;
esac

alias cargo-build='cargo build $CARGO_BUILD_FEATURES'
alias cargo-check='cargo check $CARGO_BUILD_FEATURES'
alias cargo-test='cargo test $CARGO_BUILD_FEATURES'

echo "Features: $CARGO_BUILD_FEATURES"
echo "Aliases created: cargo-build, cargo-check, cargo-test"
SCRIPT

chmod +x scripts/toggle_features.sh
```

---

### STEP 5: Integration Testing (Week 4)

#### 5.1 Full Pipeline Test

```bash
cat > scripts/full_build_test.sh << 'SCRIPT'
#!/usr/bin/env bash
set -e

echo "========================================"
echo "JCODE BUILD INTEGRATION TEST"
echo "========================================"
echo ""

PASS=0
FAIL=0

# Test 1: Clean build with default features
echo "[1/6] Clean build (default)..."
cargo clean >/dev/null 2>&1
if cargo build -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

# Test 2: Warm check
echo "[2/6] Warm check..."
touch src/agent.rs
if cargo check -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

# Test 3: Selfdev profile build
echo "[3/6] Selfdev profile..."
touch src/agent.rs
if cargo build --profile selfdev -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

# Test 4: Feature flag - embeddings
echo "[4/6] Build with embeddings..."
cargo clean >/dev/null 2>&1
if cargo build --features embeddings -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

# Test 5: Feature flag - full
echo "[5/6] Build with full features..."
cargo clean >/dev/null 2>&1
if cargo build --features full -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

# Test 6: Quick check
echo "[6/6] Quick check profile..."
if cargo check --profile quickcheck -p jcode 2>&1 | grep -q "Finished"; then
    echo "  ✓ PASS"
    ((PASS++))
else
    echo "  ✗ FAIL"
    ((FAIL++))
fi

echo ""
echo "========================================"
echo "RESULTS: $PASS passed, $FAIL failed"
echo "========================================"

[ $FAIL -eq 0 ] && exit 0 || exit 1
SCRIPT

chmod +x scripts/full_build_test.sh
```

#### 5.2 Performance Validation

```bash
cat > scripts/validate_performance.sh << 'SCRIPT'
#!/usr/bin/env bash
# Validate build performance improvements

echo "========================================"
echo "BUILD PERFORMANCE VALIDATION"
echo "========================================"
echo ""

# Baseline targets
TARGET_CHECK=5  # seconds
TARGET_SELFDEV=30  # seconds

# Measure warm check
echo "[1] Warm cargo check..."
touch src/agent.rs
START=$(date +%s%N)
cargo check -p jcode >/dev/null 2>&1
END=$(date +%s%N)
CHECK_TIME=$(echo "scale=2; ($END - $START) / 1000000" | bc)

echo "  Time: ${CHECK_TIME}s (target: <${TARGET_CHECK}s)"
if (( $(echo "$CHECK_TIME < $TARGET_CHECK" | bc -l) )); then
    echo "  ✓ PASS"
else
    echo "  ✗ FAIL (not yet optimized)"
fi

# Measure selfdev build
echo ""
echo "[2] Selfdev profile build..."
START=$(date +%s%N)
cargo build --profile selfdev -p jcode >/dev/null 2>&1
END=$(date +%s%N)
SELFDEV_TIME=$(echo "scale=2; ($END - $START) / 1000000" | bc)

echo "  Time: ${SELFDEV_TIME}s (target: <${TARGET_SELFDEV}s)"
if (( $(echo "$SELFDEV_TIME < $TARGET_SELFDEV" | bc -l) )); then
    echo "  ✓ PASS"
else
    echo "  ✗ FAIL (not yet optimized)"
fi

# sccache stats
echo ""
echo "[3] sccache efficiency..."
STATS=$(sccache --show-stats 2>/dev/null | grep -E "Cache hits|Cache misses" || echo "Cache hits: 0\nCache misses: 0")
echo "$STATS"
SCRIPT

chmod +x scripts/validate_performance.sh
```

---

## 4. TESTING STRATEGY

### 4.1 Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run with coverage
cargo tarpaulin --lib

# Run specific test
cargo test agent::types::test_name -- --nocapture
```

### 4.2 Integration Tests

```bash
# Run integration tests
cargo test --test '*'

# Run e2e tests
cargo test --test e2e

# Run specific integration test
cargo test --test integration_test_name
```

### 4.3 Build Tests

```bash
# Test all profiles
scripts/full_build_test.sh

# Test performance
scripts/validate_performance.sh

# Stress test (multiple rapid builds)
for i in {1..10}; do
    touch src/agent.rs
    cargo check -p jcode 2>&1 | grep "Finished"
done
```

### 4.4 Regression Tests

```bash
# Before any change, capture baseline
echo "8.5" > .jcode/plan/baseline_check.txt
echo "47" > .jcode/plan/baseline_selfdev.txt

# After changes, verify no regression
CHECK_TIME=$(cargo check -p jcode 2>&1 | grep -oP '\d+\.\d+(?=s elapsed)' | tail -1)
if (( $(echo "$CHECK_TIME > 8.5 + 1" | bc -l) )); then
    echo "REGRESSION DETECTED: check time increased"
    exit 1
fi
```

---

## 5. PROFILING & DEBUGGING

### 5.1 Build Profiling

```bash
# Enable timing output
export CARGO_LOG_BUILD_TIMINGS=1

# Run with timing
cargo build -p jcode --timings

# Analyze flamegraph
cargo flamegraph --bin jcode -- cargo run
```

### 5.2 Crate-by-Crate Analysis

```bash
# Measure compilation time per crate
echo "=== Crate Compilation Times ==="
for crate in $(cargo metadata --format-version 1 2>/dev/null | jq -r '.packages[].name'); do
    START=$(date +%s%N)
    cargo check -p "$crate" 2>/dev/null
    END=$(date +%s%N)
    TIME=$(echo "scale=2; ($END - $START) / 1000000" | bc)
    echo "$crate: ${TIME}ms"
done | sort -t: -k2 -n | tail -10
```

### 5.3 Debug Slow Compilation

```bash
# Check for circular dependencies
cargo tree --duplicates

# Check for large dependencies
cargo tree --depth 2 | grep -E "^.{0,20}[a-z]{30,}" | head -20

# Analyze dependency graph
cargo depgraph -p jcode | dot -Tpng > depgraph.png
```

### 5.4 Memory Profiling

```bash
# Check memory usage during build
/usr/bin/time -v cargo build -p jcode 2>&1 | grep -E "Maximum|User time|System time"

# Monitor with htop
htop -d 1  # During build in separate terminal
```

---

## 6. VALIDATION CHECKLIST

### Before Starting Implementation

- [ ] Captured baseline measurements
- [ ] Committed baseline to git
- [ ] Noted current build time

### After sccache Setup

- [ ] `sccache --show-stats` shows cache hits > 0
- [ ] Second build is faster than first
- [ ] `SCCACHE_CACHE_SIZE` configured

### After Profile Optimization

- [ ] `cargo check` completes in <8.5s (no regression)
- [ ] `cargo build --profile selfdev` completes in <47s (no regression)
- [ ] All tests pass with new profiles

### After agent.rs Modularization

- [ ] Full build completes successfully
- [ ] Touching single module only recompiles that module
- [ ] No circular dependencies introduced
- [ ] All re-exports work correctly

### After Feature Flag Optimization

- [ ] `cargo build` without features works
- [ ] `cargo build --features embeddings` works
- [ ] `cargo build --features full` works
- [ ] Feature aliases work in scripts

### Final Validation

- [ ] `cargo check` < 5 seconds
- [ ] `cargo build --profile selfdev` < 30 seconds
- [ ] All tests pass
- [ ] No performance regression
- [ ] Documentation updated

---

## 7. ROLLBACK PROCEDURES

### Immediate Rollback

```bash
# If build is broken:
cargo build --release  # Should still work with default profile

# If profile changes cause issues:
git checkout Cargo.toml.backup.*
git checkout .cargo/config.toml.backup.*

# If module structure is broken:
git checkout src/agent.rs  # Revert to monolithic
rm -rf src/agent/types.rs src/agent/state.rs src/agent/mod.rs
```

### Gradual Rollback

```bash
# Step 1: Revert feature flags
git checkout Cargo.toml

# Step 2: Revert profiles
git checkout .cargo/config.toml

# Step 3: Revert sccache
git checkout scripts/setup_sccache.sh
unset RUSTC_WRAPPER

# Step 4: Verify build works
cargo clean
cargo build -p jcode
```

### Full Reset

```bash
# Complete reset to baseline
git stash
git checkout HEAD -- Cargo.toml .cargo/ src/agent.rs
rm -rf src/agent/types.rs src/agent/state.rs src/agent/mod.rs 2>/dev/null
cargo clean
cargo build -p jcode
```

---

## 8. MILESTONES

### Milestone 1: sccache Setup (Day 1-2)
```
Deliverable: sccache configured and working
Metric: Cache hit rate > 50% on second build
Commands:
  - ./scripts/setup_sccache.sh
  - sccache --show-stats
```

### Milestone 2: Profile Optimization (Day 3-4)
```
Deliverable: Optimized Cargo profiles
Metric: Build time reduction > 20%
Commands:
  - cargo build --profile selfdev -p jcode
  - time cargo check -p jcode
```

### Milestone 3: Module Split (Week 2)
```
Deliverable: src/agent.rs modularized
Metric: Incremental compile of single module < 5s
Commands:
  - touch src/agent/types.rs && cargo check -p jcode --lib
```

### Milestone 4: Feature Flags (Week 3)
```
Deliverable: Feature flags working
Metric: Default build without embeddings < 3min
Commands:
  - cargo build (no features)
  - cargo build --features full
```

### Milestone 5: Final Validation (Week 4)
```
Deliverable: All targets met
Metrics:
  - cargo check < 5s
  - cargo build --profile selfdev < 30s
  - All tests pass
  - sccache hit rate > 80%
```

---

## APPENDIX: QUICK REFERENCE

### Key Commands

```bash
# Setup
./scripts/setup_sccache.sh
source ~/.cargo/env.sccache

# Building
cargo build --profile selfdev -p jcode  # Fast dev build
cargo check -p jcode                       # Quick check
cargo build --release -p jcode           # Production build

# Feature toggling
./scripts/toggle_features.sh dev          # No heavy features
./scripts/toggle_features.sh full         # All features

# Validation
./scripts/full_build_test.sh              # Integration tests
./scripts/validate_performance.sh          # Performance tests

# Profiling
cargo build --timings -p jcode
sccache --show-stats

# Rollback
git checkout Cargo.toml .cargo/config.toml
```

### File Locations

```
scripts/setup_sccache.sh      # sccache installation
scripts/toggle_features.sh      # Feature flag toggle
scripts/full_build_test.sh     # Integration tests
scripts/validate_performance.sh # Performance validation
.cargo/config.toml             # Cargo profile config
.jcode/plan/build_baseline.md # Baseline metrics
```

---

**Document Status:** READY FOR IMPLEMENTATION  
**Next Action:** Execute Milestone 1 (sccache Setup)
