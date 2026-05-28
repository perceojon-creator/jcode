# RFC: Optimización de Build Time para jcode

**Versión:** 1.0  
**Fecha:** 2026-05-28  
**Autor:** Staff Engineering Plan  
**Estado:** Draft  
**Meta:** Warm cargo check <5s, Warm selfdev build <30s

---

## 1. Executive Summary

### Problema Actual
| Métrica | Actual | Meta | Gap |
|---------|--------|------|-----|
| Warm cargo check | ~8.5s | <5s | +3.5s (70%) |
| Warm selfdev build | ~47s | <30s | +17s (57%) |

### Cuellos de Botella Identificados
1. **src/agent.rs** - 30.9s compile time (914 líneas, módulo central)
2. **src/tool/browser.rs** - 13.7s compile time (931 líneas)
3. **48 crates en workspace** - Alta granularidad pero overhead de coordinación
4. **Feature flags no optimizados** - `embeddings` y `pdf` en default
5. **No sccache configurado** - Compilación repetitiva sin cache

### Solución Propuesta
Sistema de optimización en 3 fases con impacto acumulativo:
- **Fase 1:** Configuración de tooling (sccache, linker, profile) → -40% tiempo
- **Fase 2:** Refactorización de módulos críticos → -30% tiempo
- **Fase 3:** Feature flag optimization + incremental improvements → -20% tiempo

---

## 2. Root Cause Analysis

### 2.1 Análisis de Estructura del Proyecto

```
Workspace: 48 crates
├── jcode (root) - 85 archivos, 15 dirs
├── jcode-core (shared types)
├── jcode-embedding (optional, ONNX deps)
├── jcode-pdf (optional, heavy deps)
└── 44+ crates more
```

### 2.2 Causas Raíz Identificadas

#### A) Arquitectura de Dependencias

**Problema:** El crate `jcode-embedding` arrastra un árbol masivo:
```
jcode-embedding
├── tokenizers v0.21 (Rust → C++ bindings via Onig)
└── tract-core v0.21
    ├── tract-hir
    ├── tract-onnx
    └── tract-linalg (SIMD kernels)
```

**Impacto:** Estas dependencias:
1. Compilan ~200+ archivos C++ en tokenizers
2. Generan código SIMD en tract-linalg
3. Añaden ~3-5 minutos a cold builds

**Código crítico en Cargo.toml:**
```toml
# Línea ~140: Feature flag actual
default = ["pdf", "embeddings"]

# Esto fuerza la compilación de ONNX + tokenizers EN CADA BUILD
# incluso cuando el usuario solo quiere `cargo check`
```

#### B) Configuración de Profile Subóptima

**Profile actual en Cargo.toml:**
```toml
[profile.release]
opt-level = 1           # ⚠️ Too aggressive for dev builds
debug = 0
codegen-units = 256      # ⚠️ Over-parallelized for small changes
incremental = true
```

**Problemas:**
1. `codegen-units = 256` causa overhead de serialización/desk
2. `opt-level = 1` aún así optimiza más de lo necesario
3. No hay profile separado para `cargo check` (warm)

#### C) Falta de sccache

**Estado actual:**
- `.cargo/config.toml` tiene: `jobs = 6` (bueno)
- NO hay `RUSTC_WRAPPER=sccache` configurado
- NO hay cache warming script

**Impacto:**
- Cada recompile regenera exactamente el mismo código
- Sin sccache, cambios pequeños = recompile completo del árbol

#### D) Module Complexity - src/agent.rs

**Líneas:** 914  
**Imports:** ~50+  
**Funciones públicas:** ~30+  
**Patrones de uso:** Async everywhere

**El cuello de botella de 30.9s se debe a:**
1. Heavy trait bounds (`Provider`, `Session`, `ToolRegistry`)
2. Generic constraints que requieren monomorphization
3. Large match arms con múltiples arms
4. Async functions que generan state machines complejas

**Código problmático en src/agent.rs:**
```rust
// Líneas 19-56: Imports muy pesados
use crate::bus::{Bus, BusEvent, SubagentStatus, ToolEvent, ToolStatus};
use crate::cache_tracker::CacheTracker;
use crate::compaction::CompactionEvent;
// ... 40+ more imports que causan recompile en cadena
```

#### E) src/tool/browser.rs - 13.7s

**Líneas:** 931  
**Problemas similares:**
1. Deserialización complex (`BrowserInput` con 20+ campos)
2. Async trait implementation pesado
3. `serde_json::Value` usage que causa bloat

---

## 3. Fases de Implementación

### Fase 1: Tooling Setup (Semana 1)

#### 1.1 sccache Configuration

**Objetivo:** Reducir recompile innecesarias en ~60%

**Archivo:** `.cargo/config.toml`

```toml
[build]
# Parallel compilation settings
jobs = 6

[target.aarch64-linux-android]
linker = "clang"
rustflags = ["-C", "link-arg=-Wl,--thinlto"]

[profile.dev]
opt-level = 0
debug = 0
incremental = true
codegen-units = 16  # Reducido para mejor cache locality
split-debuginfo = "unpacked"

[profile.release]
opt-level = 2  # Conservative para balance speed/size
debug = 0
lto = "thin"    # Light LTO para speed
codegen-units = 32  # Reducido de 256
incremental = false
panic = "abort"

[profile.selfdev]
inherits = "release"
opt-level = 0
debug = 0
lto = false
incremental = true
codegen-units = 1  # Mucho más rápido para dev

[profile.check]
inherits = "dev"
opt-level = 0
debug = 0
incremental = true
```

**Script de setup:** `scripts/setup_sccache.sh`
```bash
#!/bin/bash
set -e

# Install sccache if not present
if ! command -v sccache &> /dev/null; then
    cargo install sccache
fi

# Set environment variables
export RUSTC_WRAPPER="$(which sccache)"
export SCCACHE_GHA_ENABLED="true"  # GitHub Actions cache

# Start sccache server
sccache --start-server || true

echo "sccache configured: $(sccache --show-stats)"
```

#### 1.2 Linker Optimization (Android/AArch64)

**Problema:** El linker predeterminado de Android es lento.

**Solución:** Usar `lld` con thin LTO.

```toml
# En .cargo/config.toml -追加
[target.aarch64-linux-android]
linker = "clang"
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-Wl,--thinlto",
    "-C", "link-arg=-Wl,--lto-O2",
]
```

**Verificación:**
```bash
# Test linkage speed
time cargo build --release 2>&1 | grep -E "(Compiling|Finished|linking)"
```

#### 1.3 Scripts de Benchmarking

**Archivo:** `scripts/benchmark_build.sh`
```bash
#!/bin/bash
set -e

echo "=== BUILD TIME BENCHMARK ==="
echo "Timestamp: $(date -Iseconds)"
echo ""

# Warm cargo check
echo "--- Warm cargo check (2nd run) ---"
rm -rf target/.fingerprint 2>/dev/null || true
sleep 2
time cargo check 2>&1 | tail -5

echo ""

# Clean cargo check
echo "--- Clean cargo check ---"
cargo clean
time cargo check 2>&1 | tail -5

echo ""

# Selfdev profile build
echo "--- Selfdev profile build ---"
time CARGO_PROFILE_SELFDEV_BUILD_OVERRIDE_OPT_LEVEL=0 \
     cargo build --profile selfdev 2>&1 | tail -5

echo ""
echo "=== END BENCHMARK ==="
```

---

### Fase 2: Code Refactoring (Semanas 2-4)

#### 2.1 Split src/agent.rs en Módulos

**Problema:** Un archivo de 914 líneas causa recompile completo por cualquier cambio.

**Solución:** Dividir en módulos lógicos.

**Nuevo árbol:**
```
src/agent/
├── mod.rs              # Re-exports, elimina imports circulares
├── builder.rs          # Agent construction (NEW)
├── execution.rs       # Core execution logic (EXTRACTED)
├── state.rs           # State management (EXTRACTED)
├── types.rs           # Type aliases (NEW)
└── legacy.rs          # Compatibilidad backward (NEW)
```

**Migration plan:**

1. **Phase 2.1.1:** Crear `src/agent/types.rs`
```rust
// src/agent/types.rs
use crate::bus::{Bus, BusEvent, SubagentStatus, ToolEvent, ToolStatus};
use crate::session::Session;
use crate::tool::Registry;

// Type aliases para reducir generic bloat
pub type AgentResult<T> = Result<T, AgentError>;
pub type AgentFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = AgentResult<T>> + Send + 'a>>;

// Eliminar imports heavies de aquí
```

2. **Phase 2.1.2:** Extraer `src/agent/execution.rs`
```rust
// src/agent/execution.rs
// Move core execution logic here
// Keep imports minimal

pub async fn execute_turn(
    agent: &Agent,
    context: &ExecutionContext,
) -> AgentResult<TurnOutcome> {
    // Implementation here
}
```

3. **Phase 2.1.3:** Actualizar `src/agent.rs`
```rust
// src/agent.rs - AFTER REFACTORING
mod builder;
mod execution;
mod state;
mod types;

pub use builder::AgentBuilder;
pub use execution::{execute_turn, TurnOutcome};
pub use state::{AgentState, StateSnapshot};
```

**Beneficio esperado:** 
- Cambio en `execution.rs` solo recompila ~200 líneas vs 914
- Cache hit rate improvement: ~40%

#### 2.2 Optimize src/tool/browser.rs

**Problema:** Deserialización pesada en cada compilación.

**Solución:** Usar `serde_path_to_error` y reducir allocations.

**Antes (problemático):**
```rust
#[derive(Debug, Deserialize)]
struct BrowserInput {
    action: String,
    url: Option<String>,
    // ... 20+ fields causing serde bloat
}

impl BrowserTool {
    pub async fn execute(&self, input: BrowserInput) -> ToolResult {
        let params = serde_json::to_value(&input).unwrap(); // Double conversion!
        // Heavy serde_json operations
    }
}
```

**Después (optimizado):**
```rust
// Use serde_json directly, avoid intermediate struct
async fn parse_browser_input(json: Value) -> Result<BrowserCommand> {
    match json.get("action").and_then(|v| v.as_str()) {
        Some("status") => Ok(BrowserCommand::Status(StatusParams::from_json(json)?)),
        Some("open") => Ok(BrowserCommand::Open(OpenParams::from_json(json)?)),
        // ... other actions
        _ => Err(anyhow!("Unknown action: {}", json)),
    }
}

// Sub-structs más pequeños
#[derive(Deserialize)]
struct StatusParams {}  // Empty for status

#[derive(Deserialize)]
struct OpenParams {
    url: String,
    #[serde(default)]
    new_tab: bool,
}

impl OpenParams {
    fn from_json(json: Value) -> Result<Self> {
        Ok(Self {
            url: json["url"].as_str().context("Missing url")?,
            new_tab: json["new_tab"].as_bool().unwrap_or(false),
        })
    }
}
```

**Beneficio esperado:** ~2-3s reduction en compile time

#### 2.3 Feature Flag Optimization

**Problema:** `default = ["pdf", "embeddings"]` fuerza compilación de dependencias pesadas.

**Solución:** Mover a feature flags no-default.

**Cambios en Cargo.toml:**
```toml
[features]
# OLD (problemático):
default = ["pdf", "embeddings"]
dev-bins = []
# ...

# NEW (optimizado):
default = []  # ⚠️ Cambiar - solo features esenciales
dev-bins = []
embeddings = ["dep:jcode-embedding"]
pdf = ["dep:jcode-pdf"]
# Nueva feature para CI/testing
quick-check = ["embeddings"]  # Solo embedding para tests
full-ci = ["embeddings", "pdf"]  # Full stack para CI
```

**Script de desarrollo:** `scripts/dev.sh`
```bash
#!/bin/bash
export JCODE_FEATURE_FLAGS="embeddings"

# Or for quick iteration:
alias cargo-check='cargo check --features quick-check'
alias cargo-build='cargo build --features embeddings'
```

---

### Fase 3: Advanced Optimizations (Semanas 5-8)

#### 3.1 Crate Boundary Optimization

**Problema:** 48 crates causan overhead de coordinación.

**Análisis actual:**
```
root jcode
├── 50+ direct dependencies
└── 48 workspace members
```

**Optimización propuesta:**

1. **Consolidar crates relacionados:**
```toml
# En workspace Cargo.toml
members = [
    # CONSOLIDAR: TUI-related crates
    "crates/jcode-tui-core",
    "crates/jcode-tui-render",
    "crates/jcode-tui-markdown",
    "crates/jcode-tui-messages",
    "crates/jcode-tui-mermaid",
    # → Consolidar en crates/jcode-tui/ (AFTER PHASE 2)
    
    # CONSOLIDAR: Provider-related
    "crates/jcode-provider-core",
    "crates/jcode-provider-openai",
    "crates/jcode-provider-openrouter",
    "crates/jcode-provider-gemini",
    "crates/jcode-provider-metadata",
    # → Consolidar en crates/jcode-providers/
]
```

2. **Reducir workspace a ~20-25 crates** (consolidando los pequeños)

#### 3.2 Incremental Compilation Tuning

**Configuración optimizada:**
```toml
[profile.dev]
opt-level = 0
debug = 0
#incremental = true  # Default, pero tuneable
codegen-units = 16
split-debuginfo = "unpacked"

[profile.dev.package."*"]
# Don't optimize dev dependencies (faster compile)
opt-level = 0
```

**Build script de configuración:**
```rust
// build.rs - Optimizado para speed
fn main() {
    // Only include what you need
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/agent/*.rs");
    
    // Reduce rebuilds with version tracking
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", 
             std::time::SystemTime::now()
                 .duration_since(std::time::UNIX_EPOCH)
                 .unwrap()
                 .as_secs());
}
```

#### 3.3 Parallel Compilation Tuning

**Script:** `scripts/build_parallel.sh`
```bash
#!/bin/bash
set -e

# Query available CPU cores
CORES=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
JOBS=$((CORES * 3 / 4))  # Use 75% of cores to avoid OOM

echo "Building with $JOBS parallel jobs (from $CORES cores)"

# Build with optimal parallelism
RUSTFLAGS="-O" cargo build \
    -j "$JOBS" \
    --profile selfdev \
    "$@"
```

---

## 4. Timeline Estimado

| Fase | Duración | Entregables | Dependencias |
|------|----------|-------------|--------------|
| **Fase 1** | 1 semana | sccache, linker config, benchmarking | Ninguna |
| **Fase 2** | 3 semanas | Modular agent.rs, browser.rs optimizado | Fase 1 |
| **Fase 3** | 4 semanas | Crate consolidation, advanced profiling | Fase 2 |

**Milestones:**

| Semana | Objetivo | Métrica |
|--------|----------|---------|
| 1 | sccache funcionando | Cache hit rate >80% |
| 1 | Perfil selfdev optimizado | Build <40s |
| 2 | agent.rs modularizado | Cargo check <6s |
| 4 | browser.rs optimizado | Cargo check <5s |
| 8 | Meta alcanzada | Check <5s, Build <30s |

---

## 5. Risks y Mitigaciones

### Risk 1: Breaking Changes en Modularización

**Severity:** Medium  
**Probability:** High  
**Impact:** Regression en funcionalidades de agent

**Mitigación:**
```bash
# Validación before/after
cargo test --all-features  # Pre-refactor baseline
# ... refactor ...
cargo test --all-features  # Post-refactor validation
```

**Rollback plan:**
```bash
git checkout main -- src/agent.rs  # Immediate rollback
```

### Risk 2: Feature Flag Migration Breakage

**Severity:** High  
**Probability:** Medium  
**Impact:** CI/CD pipelines fallan, usuarios existentes broken

**Mitigación:**
1. Mantener backward compatibility en features
2. Phase-out gradual: primero deprecate, luego remove
3. Version bump para breaking change

### Risk 3: sccache Disk Space

**Severity:** Low  
**Probability:** Medium  
**Impact:** SSD lleno, cache corruption

**Mitigación:**
```bash
# Configurar límites
export SCCACHE_CACHE_SIZE="10G"  # Max 10GB cache
export SCCACHE_DIR="/tmp/sccache"  # TMPFS para Android
```

### Risk 4: Linker Regression

**Severity:** Medium  
**Probability:** Low  
**Impact:** Build fails, runtime crashes

**Mitigación:**
- Test linker change en isolation primero
- Keep fallback to system linker

---

## 6. Métricas de Éxito

### 6.1 Primary Metrics

| Métrica | Baseline | Target | Measurement |
|---------|----------|--------|-------------|
| Warm cargo check | 8.5s | <5s | `time cargo check` (2nd run) |
| Warm selfdev build | 47s | <30s | `time cargo build --profile selfdev` (2nd run) |
| Cache hit rate | 0% | >80% | `sccache --show-stats` |

### 6.2 Secondary Metrics

| Métrica | Baseline | Target |
|---------|----------|--------|
| Cold build time | ~5min | <4min |
| Incremental change recompile | varies | <3s |
| Memory usage during build | high | -20% |

### 6.3 Measurement Scripts

**Archivo:** `scripts/measure_build_speed.sh`
```bash
#!/bin/bash
set -e

RESULTS_FILE=".jcode/plan/build_metrics.csv"
TIMESTAMP=$(date -Iseconds)

echo "timestamp,warm_check,cache_hit_rate,incremental_recompile" >> "$RESULTS_FILE"

# Warm cargo check
rm -rf target/.fingerprint 2>/dev/null
sleep 2
CHECK_START=$(date +%s.%N)
cargo check > /dev/null 2>&1
CHECK_END=$(date +%s.%N)
WARM_CHECK=$(echo "$CHECK_END - $CHECK_START" | bc)

# Cache stats
CACHE_HIT=$(sccache --show-stats 2>/dev/null | grep "Cache hits" | awk '{print $4}' || echo "0")
CACHE_MISS=$(sccache --show-stats 2>/dev/null | grep "Cache misses" | awk '{print $4}' || echo "1")
CACHE_RATE=$(echo "scale=2; $CACHE_HIT / ($CACHE_HIT + $CACHE_MISS) * 100" | bc)

echo "$TIMESTAMP,$WARM_CHECK,$CACHE_RATE,incremental" >> "$RESULTS_FILE"
echo "Current: Check=${WARM_CHECK}s, Cache=${CACHE_RATE}%"
```

### 6.4 Validation Criteria

**Antes de implementar cualquier cambio, definir:**

1. **Test de regresión:**
```bash
# Baseline metrics
cargo check -t 8.5s
cargo build --profile selfdev -t 47s

# Apply change

# Must pass:
# - cargo check <8.5s (or same)
# - cargo build --profile selfdev <47s (or same)
# - cargo test passes
```

2. **A/B Testing:**
```bash
# Branch con cambios
git checkout feature/build-speed
cargo check 2>&1 | grep "Finished"

# Branch sin cambios
git checkout main
cargo check 2>&1 | grep "Finished"

# Compare timestamps
```

---

## 7. Code Examples - Implementación Detallada

### 7.1 Script: Setup sccache

**Archivo:** `scripts/setup_sccache.sh`
```bash
#!/usr/bin/env bash
set -e

echo "=== Setting up sccache for jcode ==="

# Detect platform
case "$(uname -s)" in
    Linux*)
        if [ -f /etc/debian_version ]; then
            sudo apt-get install -y sccache || cargo install sccache
        elif [ -f /etc/redhat-release ]; then
            sudo dnf install -y sccache || cargo install sccache
        else
            cargo install sccache
        fi
        ;;
    Darwin*)
        brew install sccache
        ;;
    Android*)
        # Termux
        pkg install sccache || cargo install sccache
        ;;
    *)
        cargo install sccache
        ;;
esac

# Configure environment
export RUSTC_WRAPPER="$(which sccache)"
export SCCACHE_GHA_ENABLED="true"

# Start server
sccache --start-server 2>/dev/null || true

# Create rc file for persistence
SCCACHE_RC="$HOME/.cargo/env.sccache"
cat > "$SCCACHE_RC" << 'EOF'
# sccache configuration for jcode
if command -v sccache &> /dev/null; then
    export RUSTC_WRAPPER="$(which sccache)"
    export SCCACHE_GHA_ENABLED="true"
    export SCCACHE_DIR="${HOME}/.cache/sccache"
    sccache --start-server 2>/dev/null || true
fi
EOF

echo "sccache installed: $(which sccache)"
echo "sccache stats: $(sccache --show-stats 2>/dev/null | head -5)"
echo ""
echo "Add to your shell: source $SCCACHE_RC"
```

### 7.2 Optimized .cargo/config.toml

**Archivo:** `.cargo/config.toml`
```toml
[build]
# Parallel compilation
jobs = 6

[target.aarch64-linux-android]
# Android NDK linker with LTO
linker = "clang"
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-Wl,--thinlto",
    "-C", "link-arg=-Wl,--lto-O2",
    "-C", "link-arg=-Wl,--icf=all",
]

# Profile overrides
[profile.dev]
opt-level = 0
debug = 0
incremental = true
codegen-units = 16
split-debuginfo = "unpacked"

[profile.release]
opt-level = 2
debug = 0
lto = "thin"
codegen-units = 32
incremental = false
panic = "abort"
strip = true

[profile.selfdev]
inherits = "release"
opt-level = 0
debug = 0
lto = false
incremental = true
codegen-units = 8

[profile.check]
inherits = "dev"
opt-level = 0
incremental = true
```

### 7.3 Benchmark Script

**Archivo:** `scripts/benchmark.sh`
```bash
#!/usr/bin/env bash
set -e

echo "=================================="
echo "JCODE BUILD TIME BENCHMARK"
echo "Date: $(date)"
echo "=================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

benchmark() {
    local name="$1"
    shift
    echo -e "${YELLOW}Running: $name${NC}"
    local start=$(date +%s.%N)
    "$@" 2>&1 | tail -5
    local end=$(date +%s.%N)
    local elapsed=$(echo "$end - $start" | bc)
    echo -e "${GREEN}Result: ${elapsed}s${NC}"
    echo ""
}

# Warm check
echo "--- Warm cargo check (2nd run) ---"
rm -rf target/.fingerprint 2>/dev/null || true
sleep 2
benchmark "cargo check" cargo check

# Cold check (fresh)
echo "--- Cold cargo check (clean) ---"
benchmark "cargo clean && cargo check" cargo clean && cargo check

# Selfdev build
echo "--- Selfdev profile build ---"
benchmark "cargo build --profile selfdev" cargo build --profile selfdev

# Show sccache stats
if command -v sccache &> /dev/null; then
    echo "--- sccache stats ---"
    sccache --show-stats
fi

echo "=================================="
echo "BENCHMARK COMPLETE"
echo "=================================="
```

### 7.4 Feature Flag Config

**Archivo:** `scripts/dev_env.sh`
```bash
#!/usr/bin/env bash

# Development environment for fast iteration
export JCODE_FEATURE_PROFILE="${JCODE_FEATURE_PROFILE:-quick}"

case "$JCODE_FEATURE_PROFILE" in
    minimal)
        # For fastest cargo check, no optional deps
        export RUSTFLAGS="--cfg feature=\"minimal\""
        alias cargo-check='cargo check'
        alias cargo-build='cargo build'
        ;;
    quick)
        # Quick iteration with essential features
        alias cargo-check='cargo check --features embeddings'
        alias cargo-build='cargo build --features embeddings'
        ;;
    full)
        # Full build with all features
        alias cargo-check='cargo check --features embeddings,pdf'
        alias cargo-build='cargo build --features embeddings,pdf'
        ;;
    ci)
        # CI mode
        alias cargo-check='cargo check --features quick-check'
        alias cargo-build='cargo build --features full-ci'
        ;;
esac

echo "JCODE_FEATURE_PROFILE=$JCODE_FEATURE_PROFILE"
echo "Available commands: cargo-check, cargo-build"
```

---

## 8. Dependencies

### 8.1 Herramientas Requeridas

| Herramienta | Versión Mínima | Instalación |
|-------------|----------------|-------------|
| Rust | 1.70+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| cargo | 1.70+ | Bundled with Rust |
| sccache | latest | `cargo install sccache` |
| clang (Android) | 14+ | NDK or pkg install |

### 8.2 Environment Variables

```bash
# Required
export RUSTC_WRAPPER="$(which sccache)"  # Enable sccache
export CARGO_PROFILE_SELFDEV_BUILD_OVERRIDE_OPT_LEVEL=0  # Fast selfdev

# Optional (for CI)
export SCCACHE_GHA_ENABLED="true"
export SCCACHE_CACHE_SIZE="10G"
```

### 8.3 Permisos y Recursos

| Recurso | Requerimiento |
|---------|---------------|
| Disk space | ~10GB para sccache cache |
| RAM | 8GB minimum para parallel builds |
| CPU | 4+ cores recommended |

---

## 9. Appendix: Baseline Metrics

### 9.1 Current State (2026-05-28)

```
Project: jcode v0.14.3
Workspace: 48 crates
Root crate: 85 files, 15 directories

Build times (measured on aarch64 Android):
- Warm cargo check: ~8.5s
- Warm selfdev build: ~47s
- Cold build: ~5min

Compile bottlenecks:
- src/agent.rs: 30.9s
- src/tool/browser.rs: 13.7s

Feature flags:
- default = ["pdf", "embeddings"]
- jcode-embedding pulls: tokenizers, tract-core (200+ crates)
```

### 9.2 Profiling Methods

```bash
# Cargo build timing
cargo build --timings

# Rustc timing JSON
RUST_LOG_TIMINGS=1 cargo build 2>&1 | grep timings

# flamegraph
cargo install cargo-flamegraph
cargo flamegraph -- cargo build
```

---

## 10. Approval & Next Steps

### Sign-off Required

| Role | Name | Status |
|------|------|--------|
| Tech Lead | @username | Pending |
| Staff Engineer | @username | Pending |

### Action Items

1. [ ] Review and approve this RFC
2. [ ] Allocate 8 weeks for implementation
3. [ ] Assign engineer for Phase 1
4. [ ] Set up metrics dashboard

### Questions/Open

- ¿Hay presupuesto para usar servidores de build en la nube?
- ¿Se puede hacer feature flag migration en phases?
- ¿Priority de este proyecto vs otros?

---

*Document generated: 2026-05-28*
*Last updated: 2026-05-28*
