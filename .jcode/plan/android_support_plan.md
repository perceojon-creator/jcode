# Jcode Android/Termux Support - Ultra-Detailed Implementation Plan

**Created:** 2026-05-28  
**Status:** Draft  
**Target:** Jcode v0.14.x on Android/Termux (Bionic libc)  
**Branch:** `android-support` (current commit: 407f3e8c)

---

## 1. Root Cause Analysis

### Problem Summary
Binary pre-compiled for glibc (Linux desktop) fails on Android/Termux due to **libc incompatibility**.

### Technical Root Cause

| Component | glibc (Desktop) | Bionic (Android/Termux) |
|-----------|-----------------|------------------------|
| Libc | glibc 2.35+ | bionic (Android) |
| TLS | Native TLS | Different TLS model |
| Syscalls | glibc wrappers | Direct bionic syscalls |
| Thread-local | TLS variant | bionic-specific TLS |
| malloc | jemalloc/tcmalloc |jemalloc (ok) or bionic |
| Linker | ld-linux | /system/bin/linker |
| **Result** | ✅ Works | ❌ SIGILL/SEGV |

### Why Current Binary Fails

1. **Symbol versioning mismatch** - glibc symbols (`GLIBC_2.34`) don't exist in bionic
2. **TLS access model** - Thread-local storage incompatible
3. **Dynamic linker path** - Binary hardcodes `ld-linux-aarch64.so.1` which doesn't exist on Android
4. **Missing libc functions** - Some glibc extensions absent in bionic

### Version Mismatch Risk
```
android-support branch: v0.14.x (based on master)
Origin android-support: May have additional patches vs local
```

---

## 2. Solutions Architecture

### Option A: Cross-Compilation (Recommended)
```
Desktop Build Machine → Android-compatible binary
├── Android NDK toolchain (aarch64-linux-android)
├── Bionic target sysroot
└── Rust target: aarch64-linux-android
```

**Pros:**
- Native performance
- Full feature support
- Single binary distribution

**Cons:**
- Requires NDK setup
- Build complexity
- Testing on actual device needed

### Option B: Termux Patchelf + Glibc Compatibility Layer
```
Existing glibc binary → patched with patchelf
├── Change interpreter to bionic linker
├── Patch rpaths
└── LD_PRELOAD glibc compatibility (if exists)
```

**Pros:**
- Quick workaround
- No rebuild needed

**Cons:**
- Unstable
- Many glibc symbols missing in bionic
- Not recommended

### Option C: Static Compilation (musl)
```
Build with musl libc target
├── Static binary
└── No libc dependencies
```

**Pros:**
- Single binary
- Works everywhere

**Cons:**
- musl support in Rust may be incomplete
- Some Rust crates incompatible
- Larger binary

### Recommended: Option A + Option C hybrid
- Primary: Cross-compilation with NDK
- Fallback: Static musl for portability

---

## 3. Build Setup

### 3.1 Android NDK Installation

```bash
# In Termux or Desktop build environment
# Option 1: Termux (if NDK available)
pkg install ndk

# Option 2: Manual download (recommended)
cd ~/android-ndk
wget https://dl.google.com/android/repository/android-ndk-r26b-linux.zip
unzip android-ndk-r26b-linux.zip

# Verify toolchain
ls android-ndk-r26b/toolchains/llvm/prebuilt/linux-x86_64/bin/
# Should see: aarch64-linux-android33-clang, etc.
```

### 3.2 Rust Target Setup

```bash
# Install cross-compilation targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi

# Verify
rustup target list --installed
```

### 3.3 .cargo/config.toml Configuration

Create `~/jcode/.cargo/config.toml`:

```toml
[build]
# Android NDK paths
ndk_path = "/path/to/android-ndk-r26b"

[target.aarch64-linux-android]
linker = "aarch64-linux-android33-clang"
ar = "llvm-ar"
# For C dependencies (libz, libssl, etc.)
rustflags = [
    "-C", "link-arg=--sysroot=/path/to/android-ndk-r26b/toolchains/llvm/prebuilt/linux-x86_64/sysroot",
    "-C", "link-arg=-L/path/to/android-ndk-r26b/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/33",
]

[target.armv7-linux-androideabi]
linker = "armv7a-linux-androideabi33-clang"
ar = "llvm-ar"
```

### 3.4 Environment Variables

```bash
export ANDROID_NDK_ROOT=~/android-ndk-r26b
export ANDROID_SDK_ROOT=~/android-sdk
export PATH=$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH

# For linking C libraries
export LIBZ_DIR=$ANDROID_NDK_ROOT/sysroot/usr/include
```

---

## 4. Patching Strategy

### 4.1 Current android-support Patches

From commit `407f3e8c`:

```rust
// src/auth/cursor.rs
#[cfg(target_os = "android")]
fn empty_relatives() -> Vec<Relative> { vec![] }

#[cfg(not(target_os = "android"))]
fn empty_relatives() -> Vec<Relative> {
    // desktop implementation
}
```

```toml
# Cargo.toml - Likely patched arboard dependency
# See exact patch in Cargo.lock changes
```

### 4.2 Required Patches Checklist

| Component | Issue | Solution | Status |
|-----------|-------|----------|--------|
| arboard | Platform feature gate | Add `android` feature or patch crate | 🔴 TODO |
| tokio | Platform detection | Check bionic compatibility | 🟡 Review |
| ring/smallvec | TLS usage | Verify bionic compat | 🟡 Review |
| OpenSSL bindings | System SSL | Use bionic SSL or bundle | 🟡 Review |
| Signal handling | POSIX vs Android | Verify stack traces | 🟡 Review |
| Termios | TTY operations | Check available APIs | 🟢 Likely OK |
| File I/O | Permissions model | Standard POSIX | 🟢 OK |
| Sockets | Network APIs | Standard BSD sockets | 🟢 OK |

### 4.3 Crate-Specific Patches

#### arboard (clipboard)
```toml
# Cargo.toml override
[patch.crates-io.arboard]
git = "https://github.com/your-fork/arboard"
branch = "android-support"
features = ["mandatory-only"]
```

#### tokio (async runtime)
```toml
# Usually works, but verify with:
CARGO_CFG_SOCKET=1 cargo build --target aarch64-linux-android 2>&1 | grep -i error
```

#### OpenSSL
```toml
# Use bionic SSL (API 21+) or rustls instead
openssl = { version = "0.10", features = ["vendored"] }

# Alternative: Use rustls
rustls = "0.21"
```

### 4.4 Patch Application Workflow

```bash
#!/bin/bash
# scripts/android/apply-patches.sh

set -e

BRANCH="android-support"
PKG_VERSION=$(cat Cargo.toml | grep "^version" | cut -d'"' -f2)

echo "Applying Android patches for jcode $PKG_VERSION..."

# 1. Apply core platform patches
git apply patches/core/*.patch || true

# 2. Apply crate-specific patches
for crate in crates/*/Cargo.toml; do
    if [ -f "patches/crates/$(basename $(dirname $crate)).patch" ]; then
        patch -p1 < "patches/crates/$(basename $(dirname $crate)).patch"
    fi
done

# 3. Verify build
cargo check --target aarch64-linux-android
```

---

## 5. Build Process

### 5.1 Build Commands

```bash
# Full build
cargo build --release --target aarch64-linux-android

# With specific features
cargo build --release --target aarch64-linux-android --features "full,tui"

# Incremental (for testing)
cargo build --target aarch64-linux-android
```

### 5.2 Build Environment Docker (Optional)

```dockerfile
# docker/android-builder.Dockerfile
FROM ubuntu:22.04

# Install Android NDK
ARG NDK_VERSION=r26b
RUN apt-get update && apt-get install -y wget unzip

RUN wget -q https://dl.google.com/android/repository/android-ndk-${NDK_VERSION}-linux.zip && \
    unzip -q android-ndk-${NDK_VERSION}-linux.zip -d /opt && \
    rm android-ndk-${NDK_VERSION}-linux.zip

ENV ANDROID_NDK_ROOT=/opt/android-ndk-${NDK_VERSION}
ENV PATH=$PATH:$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=$PATH:/root/.cargo/bin

# Rust targets
RUN rustup target add aarch64-linux-android armv7-linux-androideabi

WORKDIR /workspace/jcode
COPY . .

# Build
CMD cargo build --release --target aarch64-linux-android
```

### 5.3 Build Artifacts

```
target/aarch64-linux-android/release/
├── jcode              # Main binary
├── libjcode_core.so   # Dynamic libraries (if any)
└── *.so               # Native dependencies
```

### 5.4 Binary Stripping

```bash
# Reduce binary size
aarch64-linux-android-strip target/aarch64-linux-android/release/jcode

# Aggressive stripping
aarch64-linux-android-strip --strip-all target/aarch64-linux-android/release/jcode

# Check size
ls -lh target/aarch64-linux-android/release/jcode
```

---

## 6. Testing Strategy

### 6.1 Local Testing (Termux)

```bash
# Transfer binary to Termux
adb push target/aarch64-linux-android/release/jcode /data/data/com.termux/files/home/bin/

# Or via Termux API / scp
scp jcode user@termux:/data/data/com.termux/files/home/bin/

# Make executable
chmod +x ~/bin/jcode

# Test basic functionality
./jcode --version

# Test auth flow
./jcode auth-test

# Test TUI (if DISPLAY or TERMUX_APP__EXTRA_LAUNCH_ARGS available)
export TERM=xterm-256color
./jcode
```

### 6.2 Automated Test Suite

Create `tests/android_smoke_tests.sh`:

```bash
#!/bin/bash
# Android/Termux smoke tests

set -e

JCODE_BIN="${1:-$HOME/bin/jcode}"
HOST="${2:-127.0.0.1}"
PORT="${3:-8080}"

echo "=== Jcode Android Smoke Tests ==="

# Test 1: Binary runs
echo "[1/5] Testing binary execution..."
$JCODE_BIN --version || { echo "FAIL: Binary not executable"; exit 1; }

# Test 2: No immediate crash
echo "[2/5] Testing no immediate crash..."
timeout 5 $JCODE_BIN --help > /dev/null 2>&1 || true

# Test 3: Help output
echo "[3/5] Testing help output..."
$JCODE_BIN --help | grep -q "jcode" || { echo "FAIL: Help broken"; exit 1; }

# Test 4: Server mode (background)
echo "[4/5] Testing server mode..."
$JCODE_BIN server --host $HOST --port $PORT &
SERVER_PID=$!
sleep 3

if kill -0 $SERVER_PID 2>/dev/null; then
    echo "PASS: Server started"
    kill $SERVER_PID 2>/dev/null || true
else
    echo "FAIL: Server crashed"
    exit 1
fi

# Test 5: Auth test
echo "[5/5] Testing auth subsystem..."
timeout 30 $JCODE_BIN auth-test 2>&1 | head -20

echo "=== All smoke tests passed ==="
```

### 6.3 Integration Testing

```bash
# Test with actual API keys
export JCODE_PROVIDER=openai
export JCODE_API_KEY="sk-..."

# Run actual agent task
./jcode --provider openai --model gpt-4 "Write hello world in Rust"

# Check exit code
if [ $? -eq 0 ]; then
    echo "SUCCESS: Full integration works"
else
    echo "FAIL: Integration broken"
fi
```

### 6.4 Test Matrix

| Test | Termux (aarch64) | Android Emulator | Physical Device |
|------|------------------|------------------|-----------------|
| Binary runs | ✅ | ✅ | ✅ |
| TUI renders | ❓ (headless) | ❓ | ❓ |
| Clipboard | ❓ | ❓ | ❓ |
| Provider API | ✅ | ✅ | ✅ |
| File operations | ✅ | ✅ | ✅ |
| Memory persistence | ✅ | ✅ | ✅ |

---

## 7. CI/CD Pipeline

### 7.1 GitHub Actions Workflow

Create `.github/workflows/android-build.yml`:

```yaml
name: Android Build (Termux)

on:
  push:
    branches: [master, android-support]
  pull_request:
    branches: [master, android-support]
  release:
    types: [published]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  android-build:
    runs-on: ubuntu-22.04
    
    steps:
    - name: Checkout
      uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: aarch64-linux-android, armv7-linux-androideabi

    - name: Cache Rust
      uses: Swatinem/rust-cache@v2
      with:
        workspaces: "./target -> /root/.cargo/registry"

    - name: Install Android NDK
      id: ndk
      run: |
        cd /opt
        wget -q https://dl.google.com/android/repository/android-ndk-r26b-linux.zip
        unzip -q android-ndk-r26b-linux.zip
        echo "ndk_path=/opt/android-ndk-r26b" >> $GITHUB_OUTPUT

    - name: Configure Cargo
      run: |
        cat > .cargo/config.toml << 'EOF'
        [build]
        ndk_path = "${{ steps.ndk.outputs.ndk_path }}"
        
        [target.aarch64-linux-android]
        linker = "aarch64-linux-android33-clang"
        rustflags = ["-C", "link-arg=--sysroot=${{ steps.ndk.outputs.ndk_path }}/toolchains/llvm/prebuilt/linux-x86_64/sysroot"]
        EOF

    - name: Build aarch64
      run: |
        cargo build --release --target aarch64-linux-android

    - name: Build armv7
      run: |
        cargo build --release --target armv7-linux-androideabi

    - name: Strip binaries
      run: |
        ${{ steps.ndk.outputs.ndk_path }}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android-strip target/aarch64-linux-android/release/jcode
        ${{ steps.ndk.outputs.ndk_path }}/toolchains/llvm/prebuilt/linux-x86_64/bin/arm-linux-androideabi-strip target/armv7-linux-androideabi/release/jcode

    - name: Upload artifacts
      uses: actions/upload-artifact@v4
      with:
        name: jcode-android
        path: |
          target/aarch64-linux-android/release/jcode
          target/armv7-linux-androideabi/release/jcode
        retention-days: 30

    - name: Create Termux package (optional)
      if: github.event_name == 'release'
      run: |
        # Create Termux package structure
        mkdir -p com.termux/jcode
        cp target/aarch64-linux-android/release/jcode com.termux/jcode/jcode-arm64-v{{ steps.version.outputs.ver }}.deb
        
        # Generate .deb metadata
        # ... (see Termux packaging guide)
```

### 7.2 Build Artifacts

```
builds/
├── jcode-v0.14.x-aarch64          # Primary (most devices)
├── jcode-v0.14.x-armv7           # Fallback (older devices)
└── checksums.txt                  # SHA256 for verification
```

### 7.3 Versioning Strategy

```bash
# Extract version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | cut -d'"' -f2)
BUILD_DATE=$(date -u +%Y%m%d-%H%M%S)
BUILD_GIT=$(git rev-parse --short HEAD)

# Final artifact naming
ARTIFACT="jcode-${VERSION}-android-${BUILD_DATE}-${BUILD_GIT}"
```

---

## 8. Distribution

### 8.1 Termux Package

Create `termux-packages/jcode/` structure:

```
termux-packages/jcode/
├── debian/
│   ├── changelog
│   ├── control
│   ├── copyright
│   └── rules
├── packaging/
│   └── jcode.sh
└── src/
    └── (build from source)
```

### 8.2 APK Distribution (Alternative)

For non-Termux Android:

```bash
# Build standalone APK with bundled Rust binary
# Using termux-boot or similar wrapper
```

### 8.3 Direct Download

```bash
# Update release assets
curl -L https://github.com/1jehuang/jcode/releases/latest/download/jcode-android-aarch64
chmod +x jcode-android-aarch64
./jcode-android-aarch64
```

---

## 9. Maintenance Strategy

### 9.1 Upstream Sync

```bash
#!/bin/bash
# scripts/android/sync-upstream.sh

set -e

UPSTREAM="upstream"
CURRENT=$(git rev-parse --abbrev-ref HEAD)

echo "Syncing Android support with upstream..."

# Fetch latest
git fetch upstream
git fetch origin

# Merge upstream changes
git checkout android-support
git merge upstream/master --no-ff -m "Merge upstream into android-support"

# Resolve conflicts
# ... (manual resolution if needed)

# Run build test
cargo build --target aarch64-linux-android

# Run smoke tests
./scripts/android/smoke_tests.sh

# Push
git push origin android-support
```

### 9.2 Version Tracking

| Jcode Version | Android Support Status | Last Tested | Notes |
|-------------|----------------------|-------------|-------|
| v0.14.3 | ⚠️ Partial | 2026-05-28 | Core patches exist |
| v0.13.x | ❌ Outdated | - | Old branch |

### 9.3 Dependency Updates

```bash
# Monthly dependency audit for Android compatibility
cargo audit
cargo tree -i android-support  # Show Android-specific deps

# Test after updating any crate
cargo update
cargo build --target aarch64-linux-android
```

---

## 10. Alternative Approaches

### 10.1 WebAssembly (WASM)

```bash
# Experimental: Compile to WASM
cargo build --release --target wasm32-wasip1

# Use wasmtime runner
wasmtime jcode.wasm
```

**Pros:** Cross-platform, no native build needed
**Cons:** Limited system access, performance overhead

### 10.2 Remote Build Service

```
User (Termux) → Build Server API → Returns binary
```

**Pros:** No build environment on device
**Cons:** Network dependency, latency, privacy concerns

### 10.3 Containerized Build

```bash
# Build in Docker container
docker run --rm -v $(pwd):/workspace jcode-android-builder
```

---

## 11. Implementation Roadmap

### Phase 1: Build Infrastructure (Week 1)
- [ ] Install Android NDK
- [ ] Configure Rust cross-compilation targets
- [ ] Create `.cargo/config.toml`
- [ ] Verify basic `cargo build` works
- [ ] First successful binary generation

### Phase 2: Crate Compatibility (Week 2)
- [ ] Identify broken crates (arboard, OpenSSL, etc.)
- [ ] Apply necessary patches
- [ ] Test each dependency on Android target
- [ ] Create workaround patches

### Phase 3: Testing & Validation (Week 3)
- [ ] Setup Termux testing environment
- [ ] Run smoke tests
- [ ] Test actual agent functionality
- [ ] Document test results

### Phase 4: CI/CD & Distribution (Week 4)
- [ ] Configure GitHub Actions workflow
- [ ] Create Termux package structure
- [ ] Set up release automation
- [ ] Publish first Android release

---

## 12. Troubleshooting Guide

### Issue: `aarch64-linux-android33-clang: command not found`
**Solution:** Add NDK bin to PATH, verify `which aarch64-linux-android33-clang`

### Issue: `error: linking failed: crtbegin.o not found`
**Solution:** Set `--sysroot` to correct NDK sysroot path

### Issue: `SIGILL` on binary execution
**Cause:** Binary compiled for wrong architecture or incompatible libc
**Solution:** Recompile with NDK toolchain, verify `file jcode` output

### Issue: `undefined reference to 'pthread_create'`
**Cause:** Missing pthread linking
**Solution:** Add `-pthread` to RUSTFLAGS

### Issue: OpenSSL symbol errors
**Solution:** Use `openssl = { features = ["vendored"] }` in Cargo.toml

---

## 13. Quick Start Commands

```bash
# 1. Clone and checkout
cd ~/jcode
git checkout android-support

# 2. Install NDK (if not present)
pkg install ndk

# 3. Add Rust targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# 4. Configure
cat > .cargo/config.toml << 'EOF'
[build]
ndk_path = "$PREFIX/opt/android-ndk-r26b"

[target.aarch64-linux-android]
linker = "aarch64-linux-android33-clang"
EOF

# 5. Build
cargo build --release --target aarch64-linux-android

# 6. Test
scp target/aarch64-linux-android/release/jcode termux:/data/data/com.termux/files/home/bin/
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-28  
**Maintainer:** @perceojon-creator  
**Upstream:** 1jehuang/jcode