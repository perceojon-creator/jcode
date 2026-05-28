# jcode Developer Experience & Contribution Analysis

**Date:** 2026-05-28  
**Perspective:** Developer Experience (DX) & Contribution Analysis

---

## 1. Curva de Aprendizaje

### Nivel de Entrada
| Aspecto | Evaluación |
|---------|------------|
| Lenguaje | Rust (requiere experiencia sólida) |
| Complejidad arquitectónica | Alta - 52 crates en workspace |
| Conceptos propios | Agent harness, swarm, self-dev, memory graphs |

### Obstáculos Iniciales Identificados

1. **Economía del Conocimiento:**
   - El proyecto usa terminología propia que requiere aprendizaje: *swarm*, *harness*, *selfdev*, *tui*, *ambient*
   - Los archivos grandes (server.rs ~83K, agent.rs ~35K) dificultan entender el flujo completo
   - 50+ archivos >1200 LOC en producción (del CODE_QUALITY_TODO audit)

2. **Entorno de Build Complex:**
   - 77 scripts de utility con múltiples variantes (debug, release, profile)
   - Configuración específica: `scripts/dev_cargo.sh` con profiles (`selfdev`, feature profiles)
   - Requiere entender `JCODE_REF_HOME`, `JCODE_SOCKET` para desarrollo
   - earlyoom + low-memory handling para máquinas con <24GiB RAM

3. **Conocimiento de Dominio Necesario:**
   - Protocolo de comunicación agent<->terminal
   - Sistema de plugins MCP (Model Context Protocol)
   - Provider abstraction (OpenAI, OpenRouter, Gemini, Azure, Claude, Copilot)
   - Session management y persistence
   - Telemetry y memory graph system

---

## 2. Documentación y DX

### Fortalezas

| Documento | Utilidad |
|-----------|----------|
| `AGENTS.md` | Workflow de desarrollo claro, commit-as-you-go, remote build guidance |
| `CONTRIBUTING.md` | Expectativas realistas - PRs como proposals, no guaranteed merge |
| `docs/CODE_QUALITY_10_10_PLAN.md` | Estándares claros ymedibles para contributors |
| `docs/REFACTORING.md` | Ratchet de seguridad para evitar regresiones |
| `docs/COMPILE_PERFORMANCE_PLAN.md` | Benchmarks documentados y scripts de medición |
| `docs/CRATE_OWNERSHIP_BOUNDARIES.md` | Checklist práctico para decisiones de split |
| `docs/CODE_QUALITY_TODO.md` | 430+ líneas de backlog detallado con status tracking |

### Debilidades

1. **Curva de Contribución:**
   - No hay "getting started" guía para nuevos contributors
   - No hay ejemplos de typical PR workflow
   - Documentación de arquitectura dispersa en múltiples RFCs

2. **Ausencias:**
   - No existe tutorial para setup local
   - No hay examples de cómo hacer un típico self-dev change
   - No hay decision tree para "dónde pertenece este código?"

3. **Inconsistencias:**
   - Los docs asumen conocimiento previo del codebase
   - Algunos scripts estánbien documentados (`refactor_shadow.sh`) vs otros sin help

---

## 3. Proceso de Build y Test

### Build Workflow

```bash
# Fast local dev loop
scripts/dev_cargo.sh check --quiet

# Self-dev build (optimizado)
scripts/dev_cargo.sh build --profile selfdev -p jcode --bin jcode --quiet

# Isolated refactor environment  
scripts/refactor_shadow.sh build [--release]
scripts/refactor_shadow.sh serve  # Runs in ~/.jcode-refactor
```

### Test Pipeline

```bash
# Fast test loop (lib + bins)
scripts/test_fast.sh

# Full e2e coverage
scripts/test_e2e.sh

# Verification before refactor merges
scripts/refactor_phase1_verify.sh
```

### Guardrails de Calidad

| Script | Función |
|--------|---------|
| `check_warning_budget.sh` | Previene warning drift |
| `check_code_size_budget.py` | Enforces ratchet en oversized files |
| `check_test_size_budget.py` | Enforces test file size limits |
| `check_panic_budget.py` | Ratchet de unwrap/expect/panic usage |
| `check_swallowed_error_budget.py` | Errores silenciados ratchet |
| `security_preflight.sh` | Scan de secrets + permissions + audit |
| `cargo audit` | Dependency advisory checks |

### CI/CD

- **45 min timeout** para quality guardrails
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Formato enforced: `cargo fmt --all -- --check`
- 4+ ratchet budgets activos en CI

---

## 4. Obstáculos para Contribución

### Obstáculos Altos

1. **Monumental Code Files:**
   - `src/server.rs` (83K) - El archivo de producción más grande
   - `src/agent.rs` (35K) - Turn loop + tool orchestration
   - `src/provider/mod.rs` (23K) - Provider trait + routing
   - 50 archivos >1200 LOC requieren split antes de tocar

2. **Rust Complexity:**
   - Async/await con Tokio runtime
   - Trait objects y dynamic dispatch para providers
   - Shared state con channels y Arc<Mutex<T>>
   - Generics para tool definitions

3. **Self-Dev Complexity:**
   - Build system que se auto-modifica durante reload
   - Isolated environment (`~/.jcode-refactor`) para proteger sesiones live
   - Version/channel management para source builds

### Obstáculos Medios

1. **Linker Configuration:**
   - Requiere `clang + lld` vs `mold` según plataforma
   - sccache configuration para builds repetidos
   - Low-memory handling con earlyoom detection

2. **Provider Ecosystem:**
   - 6+ providers con APIs diferentes
   - OpenAI compatible profile runtime
   - Model catalog + pricing tables complexos

3. **Memory System:**
   - Semantic recall con embedding layer
   - PDF extraction con jcode-pdf
   - Memory graph con graph traversal

### Obstáculos Bsajos (Mejorables)

1. **Ausencia de Contributor Onboarding:**
   - No hay `CONTRIBUTING_CHECKLIST.md`
   - No hay `FIRST_PR_GUIDE.md`

2. **Build Speed:**
   - Warm `cargo check`: ~7-14s según archivo modificado
   - Warm selfdev build: ~12-30s según archivo
   - Progreso documentado pero aún tiene room: target <5s check, <20s build

3. **Test Complexity:**
   - e2e tests requieren provider account setup
   - Auth tests requieren OAuth tokens
   - Mobile simulator tests requieren specific setup

---

## 5. Self-Dev Capabilities

### Auto-Modificación

El proyecto puede modificarse a sí mismo via:

1. **Reload System:**
   - `jcode serve --dev-reload` detecta cambios source
   - Rebuild + restart sin perder sesión
   - Scripts dedicated: `scripts/refactor_shadow.sh` para safe iteration

2. **Customization Records:**
   - Issue #32: User customization sin rebuild
   - Prompt overlays: `~/.jcode/prompt-overlay.md`, `./.jcode/prompt-overlay.md`
   - Extension points planificados para hooks/skills/config

3. **Build Support Crate:**
   - `crates/jcode-build-support` manage build commands
   - Source-state fingerprints
   - Binary channel paths/manifests

### Current State

| Capability | Status | Notes |
|------------|--------|-------|
| Hot reload | ✅ Available | Via `serve --dev-reload` |
| Prompt overlay | ✅ Available | Sin rebuild |
| Prompt customization | ✅ Available | Sin rebuild |
| Source modification | ✅ Available | Requiere rebuild |
| Skill extension | 🔜 Planned | Issue #32 roadmap |
| Hook extension | 🔜 Planned | Issue #32 roadmap |

---

## 6. Áreas Problemáticas para Nuevos Contributors

### Alta Prioridad (Bloquean onboarding)

#### A. Orientación inicial

**Problema:** No hay mapa del codebase. Un contributor nuevo no sabe:
- Por dónde empezar a leer
- Qué archivos son críticos vs utility
- Cómo se relacionan los módulos

**Recomendación:** Crear `docs/ARCHITECTURE_OVERVIEW.md` con:
```
src/
├── agent.rs          # Main agent orchestration (start here)
├── server.rs         # Server lifecycle (important for reload)
├── provider/         # Provider abstraction layer
├── tool/             # Tool definitions registry
├── tui/              # Terminal UI layer
└── memory.rs         # Memory system
```

#### B. Archivos monumentales

**Problema:** 50 archivos >1200 LOC hacen que cualquier change parezca riskante.

**Recomendación:** Priorizar splits de `CODE_QUALITY_TODO.md`:
- `src/server/comm_control.rs` (3228 LOC) - más urgent
- `src/agent.rs` (34956 LOC, + tests)
- `src/provider/mod.rs` (2365 LOC)

#### C. Ghost scripts

**Problema:** 77 scripts, pero no hay `scripts/list_by_purpose.sh`.

**Recomendación:** Organizar scripts en categorías:
- Build: `dev_cargo.sh`, `remote_build.sh`
- Test: `test_fast.sh`, `test_e2e.sh`, `test_swarm.sh`
- Refactor: `refactor_shadow.sh`, `refactor_phase1_verify.sh`
- Bench: `bench_compile.sh`, `bench_selfdev_checkpoints.sh`

### Media Prioridad (Friction)

#### D. Build complexity sin guía

**Problema:** `COMPILE_PERFORMANCE_PLAN.md` tiene 630 líneas de contexto.

**Recomendación:** Crear `docs/DEVELOPER_QUICKSTART.md`:
```bash
# Setup
git clone ...
./scripts/install.sh

# Typical dev loop
cargo check  # or: scripts/dev_cargo.sh check
scripts/test_fast.sh
scripts/refactor_shadow.sh build && scripts/refactor_shadow.sh serve
```

#### E. CI Guardrails Opaque

**Problema:** 6+ budget scripts que deben pasar en CI pero no hay herramienta local para check.

**Recomendación:** Consolidar en `scripts/check_all_guardrails.sh`:
```bash
#!/bin/bash
scripts/check_warning_budget.sh
scripts/check_code_size_budget.py
scripts/check_test_size_budget.py
scripts/check_panic_budget.py
scripts/check_swallowed_error_budget.py
scripts/security_preflight.sh
```

#### F. Provider complexity

**Problema:** 6 providers con patrones diferentes - no hay Provider development guide.

**Recomendación:** Crear `docs/PROVIDER_DEV_GUIDE.md`:
- Cómo agregar un nuevo provider
- Provider trait boundaries
- Test patterns para providers

### Baja Prioridad (Nice-to-have)

- **Shell completion:** Scripts usan argparse simplificado, bash completion sería útil
- **Editor integration:** rust-analyzer config documentado
- **VSCode devcontainer:** Setup reproducible para contributors Windows

---

## 7. Scorecard de DX

| Dimensión | Score | /10 | Notas |
|-----------|-------|-----|-------|
| **Documentación Técnica** | 8 | 10 | Docs existentes son excelentes pero dispersos |
| **Onboarding** | 3 | 10 | No hay getting-started real |
| **Build Experience** | 6 | 10 | Scripts útiles pero complejos |
| **Code Clarity** | 4 | 10 | 50 archivos >1200 LOC, arquitectura hard |
| **Testability** | 7 | 10 | Tests buenos, pero requieren setup |
| **CI/CD Guardrails** | 9 | 10 | Ratchets robustos, automatizados |
| **Self-Dev Capability** | 8 | 10 | Hot reload + prompt overlay |
| **Contrib Acceptance** | 7 | 10 | Políticas claras, pero merge uncertain |
| **Overall DX** | **6.5** | 10 | Product maduro, onboarding weak |

---

## 8. Recomendaciones Prioritarias

### Must-Have (Antes de pub advertising)

1. **[HIGH]** Crear `docs/DEVELOPER_QUICKSTART.md` con 20-line setup + typical loop
2. **[HIGH]** Crear `docs/ARCHITECTURE_OVERVIEW.md` - 1-page mapa del codebase
3. **[HIGH]** Priorizar split de `src/agent.rs` y `src/server/comm_control.rs`

### Should-Have (Mejora DX)

4. **[MED]** Consolidar guardrails en `scripts/check_all_guardrails.sh`
5. **[MED]** Organizar scripts con metadata headers y `-h` help
6. **[MED]** Crear `docs/PROVIDER_DEV_GUIDE.md` para extensión de providers

### Nice-to-Have (Post-mVP)

7. **[LOW]** Agregar `scripts/list_by_purpose.sh`
8. **[LOW]** Devcontainer para reproducibilidad
9. **[LOW]** rust-analyzer settings documentation

---

## 9. Conclusión

**jcode es un proyecto técnicamente maduro con:**
- Documentación interna extensiva (630 líneas de compile plan, 430 líneas de quality todo)
- Ratchets de CI robustos que mantienen quality
- Self-dev capabilities que demuestran meta-circularidad
- Arquitectura modular en progreso (52 crates)

**Pero tiene barreras significativas de entrada:**
- 50+ archivos >1200 LOC requieren navegación cuidadosa
- Sin onboarding guide para nuevos contributors
- Build system requiere configuración no-documentada

**El mayor riesgo de contribución es el código generado por AI** - el maintainer explícitamente advierte que PRs generados son "deceptively plausible" y serán reimplementados. Esto sugiere que contribuciones manuales con tests narrow tienen más probabilidad de merge.

**Recomendación estratégica:** Para maximizar contribución exitos, contributors deben:
1. Empezar con un archivo pequeño y bien-encapsulado (ej: un helper en `jcode-core`)
2. Proporcionar tests narrow que demuestren behavior
3. Evitar cambios generation-large o highly automated
4. Seguir los ratchet scripts para verificar antes de PR
