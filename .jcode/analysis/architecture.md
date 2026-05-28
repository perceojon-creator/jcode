# Análisis de Arquitectura y Modularidad - jcode

**Fecha:** 2026-05-28  
**Perspectiva:** Arquitectura y Modularidad  
**Versión jcode:** 0.14.4

---

## Resumen Ejecutivo

El proyecto jcode presenta una **arquitectura de monólito modular en evolución** hacia un workspace distribuidos. Con **50 crates** en el workspace, el proyecto ha logrado extracciones significativas, pero el **craze crate raíz** (`jcode`) sigue siendo el centro de gravedad dominante con ~230K líneas de código en `src/`.

### Estado General: 🟡 En Transición

- ✅ 50 crates extraídos del workspace
- ✅ Fronteras de módulos bien definidas en `src/server/`, `src/provider/`, `src/tui/`
- ⚠️ Raíz `jcode` aún contiene ~230K líneas de código principal
- ⚠️ Alta cohesión interna pero alto acoplamiento en ciertos módulos hotspot

---

## 1. Estructura de Crates - Panorama General

### 1.1 Crates por categoría

| Categoría | Crates | Propósito |
|-----------|--------|-----------|
| **Types/Contracts** | 18 | DTOs puros sin lógica de negocio (jcode-message-types, jcode-tool-types, etc.) |
| **Provider** | 5 | Implementaciones de providers (provider-core, openai, openrouter, gemini, metadata) |
| **TUI** | 11 | Widgets y componentes de UI (tui-core, tui-mermaid, tui-render, etc.) |
| **Heavy Leaves** | 4 | Integraciones pesadas (jcode-embedding, jcode-pdf, jcode-notify-email, jcode-azure-auth) |
| **Runtime** | 7 | Orquestación (agent-runtime, swarm-core, compaction-core, etc.) |
| **Platform** | 5 | Plataformas específicas (mobile-core, mobile-sim, desktop) |

### 1.2 Distribución de dependencias (Cargo.toml workspace)

```toml
[workspace]
members = [
    # Foundation
    "crates/jcode-core",           # Utilidades de bajo nivel
    "crates/jcode-storage",         # Helpers de storage
    
    # Types/Contracts (18 crates)
    "crates/jcode-*-types",        # 13 crates de tipos
    
    # Providers
    "crates/jcode-provider-core",
    "crates/jcode-provider-openai",
    "crates/jcode-provider-openrouter",
    "crates/jcode-provider-gemini",
    "crates/jcode-provider-metadata",
    
    # TUI (11 crates)
    "crates/jcode-tui-core",
    "crates/jcode-tui-mermaid",
    "crates/jcode-tui-render",
    "crates/jcode-tui-workspace",
    ... (7 más)
    
    # Heavy leaves
    "crates/jcode-embedding",      # ~163 dependencias de ONNX/tokenizer
    "crates/jcode-pdf",            # feature-gated
    "crates/jcode-notify-email",   # IMAP/SMTP
    "crates/jcode-azure-auth",     # Azure SDK
]
```

---

## 2. Crate Splits Realizados

### 2.1 Splits mayores completados (según COMPILE_PERFORMANCE_PLAN.md)

| Split | Fecha | Descripción |
|-------|-------|-------------|
| `jcode-embedding` | 2026-03-24 | ONNX/tokenizer (163 crates) - aislamiento de inferencia pesada |
| `jcode-pdf` | 2026-03-24 | Extracción de texto PDF |
| `jcode-azure-auth` | 2026-03-24 | Azure bearer token |
| `jcode-notify-email` | 2026-03-24 | SMTP/IMAP/mail |
| `jcode-provider-metadata` | 2026-03-25 | Catálogos de providers |
| `jcode-provider-core` | 2026-03-25 | HTTP client, route/cost/types |
| `jcode-provider-openrouter` | 2026-03-25 | Helpers de OpenRouter |
| `jcode-provider-gemini` | 2026-03-25 | Schema/model helpers de Gemini |
| `jcode-tui-workspace` | 2026-03-30 | Workspace map widget |

### 2.2 Splits de types/contracts (2026-05-05)

| Crate | Contenido |
|-------|-----------|
| `jcode-message-types` | Message, ContentBlock, ToolDefinition, StreamEvent |
| `jcode-tool-types` | ToolOutput, ToolImage |
| `jcode-tool-core` | Tool trait, ToolContext, ToolExecutionMode |
| `jcode-provider-core::Provider` | Trait Provider + EventStream alias |

---

## 3. Diseño de Fronteras entre Módulos

### 3.1 Frontera Provider (más madura)

```
jcode-root (src/provider/mod.rs)
    ├── jcode-provider-core (Provider trait, value types)
    │       ├── jcode-provider-openrouter
    │       ├── jcode-provider-gemini
    │       └── jcode-provider-metadata
    └── jcode-root (implementaciones concretas)
```

**Estado:** ✅ Avanzado - El trait `Provider` vive en `jcode-provider-core`

### 3.2 Frontera TUI (en progreso)

```
jcode-root (src/tui/*)
    ├── jcode-tui-core (keybind parsing, stream buffers)
    ├── jcode-tui-mermaid
    ├── jcode-tui-workspace (widget)
    └── jcode-root (app state, reducers)
```

**Estado:** 🟡 Parcial - Widgets extraídos, app state aún en raíz

### 3.3 Frontera Embedding/PDF

```
jcode-root (src/embedding.rs)
    ├── jcode-embedding (ONNX + tokenizers) [feature=embeddings]
    └── jcode-root (facade + cache/stats)

jcode-root (src/import.rs)
    ├── jcode-pdf (pdf-extract) [feature=pdf]
    └── jcode-root (facade)
```

**Estado:** ✅ Completado - Feature-gated correctamente

---

## 4. Análisis de Acoplamiento y Cohesión

### 4.1 Módulos hotspot (mayor fan-out)

| Archivo | Líneas | Descripción |
|---------|--------|-------------|
| `src/server.rs` | 1731 | Orquestación del servidor, ciclo de vida |
| `src/provider/mod.rs` | 2283 | Trait + implementaciones de providers |
| `src/session.rs` | 2730 | Estado de sesión, transiciones |
| `src/agent.rs` | ~900 | Loop del agente |
| `src/compaction.rs` | ~1500 | Compactación de contexto |

### 4.2 Acoplamiento detectado

**Problemas de acoplamiento:**

1. **Provider ↔ Message:** El trait `Provider` aún depende de `jcode-message-types` que vive parcialmente en la raíz
2. **TUI ↔ Server:** La TUI mantiene referencias a `App` que tiene dependencias de servidor
3. **Tool Registry:** El registry de tools (`src/tool/mod.rs`) arrastra muchas dependencias

**Cohesión interna:**
- ✅ `src/server/` tiene buena separación (client_lifecycle, comm_control, etc.)
- ✅ `src/provider/` tiene submodule tree maduro
- ⚠️ `src/tui/` tiene muchos archivos grandes (ui_messages.rs: 1848 líneas)

---

## 5. Áreas que Necesitan Más Refactoring

### 5.1 Prioridad ALTA

| Área | Problema | Recomendación |
|------|----------|---------------|
| `src/session.rs` (2730 líneas) | Monolito de sesión | Extraer `jcode-session` con estado y transiciones |
| `src/provider/mod.rs` (2283 líneas) | Trait + impls mezclados | Extraer `jcode-provider` runtime crate |
| `src/server.rs` (1731 líneas) | Orquestación pesada | Continuar shrinking hacia `src/server/` submodules |
| `src/tui/ui_messages.rs` (1848 líneas) | Widget demasiado grande | Dividir en módulos más pequeños |

### 5.2 Prioridad MEDIA

| Área | Problema | Recomendación |
|------|----------|---------------|
| `src/compaction.rs` | Lógica de compactación pesada | Extraer a `jcode-compaction-core` o nuevo crate |
| `src/tui/` | App state acoplado | Continuar TUI state/reducer split |
| `src/tool/` | Registry demasiado acoplado | Separar tool definitions de impls |

### 5.3 Métricas de compilación (desde COMPILE_PERFORMANCE_PLAN.md)

| Touched file | Warm `cargo check` | Warm `selfdev-jcode` build |
|---|---|---|
| `src/agent.rs` | 7.3s | **30.9s** (hotspot) |
| `src/tool/browser.rs` | **13.7s** (hotspot) | 18.9s |
| `src/provider/mod.rs` | 9.8s | 17.9s |
| `src/server.rs` | 8.7s | 19.0s |

---

## 6. Estado Actual de la Modularización

### 6.1 Progreso vs. Meta (MODULAR_ARCHITECTURE_RFC.md)

| Capa objetivo | Estado | Notas |
|--------------|--------|-------|
| **L0: Foundation** | ✅ 70% | types, core, storage extraídos |
| **L1: Domain/Runtime** | 🟡 40% | Provider avanzado, server/agent en progreso |
| **L2: Interfaces** | 🟡 30% | TUI parcialmente extraído |
| **L3: Composition** | ⏳ Pendiente | Raíz aún muy grande |

### 6.2 Reglas de dependencia implementadas

✅ **Regla 1:** Dependencias fluyen hacia abajo  
✅ **Regla 5:** Async/network no en `jcode-core`  
✅ **Regla 6:** Contratos estables vs. orquestación  
⚠️ **Regla 2:** TUI types cerca de interface layer (parcial)  
⚠️ **Regla 4:** Provider leaf crates no dependen de root (en progreso)

### 6.3 Cumplimiento de criterios de extracción

| Criterio | Provider | TUI | Server | Session |
|----------|-----------|-----|--------|---------|
| API describible | ✅ | ✅ | ✅ | ⚠️ |
| Sin callbacks a raíz | ✅ | ⚠️ | ⚠️ | ❌ |
| Deps solo downward | ✅ | ⚠️ | ⚠️ | ❌ |
| Tests a nivel crate | ✅ | ✅ | ⚠️ | ❌ |
| Benchmark muestra mejora | ✅ | ✅ | 🟡 | ❌ |

---

## 7. Recomendaciones

### 7.1 Próximos splits (ROI más alto)

1. **Extraer `jcode-session`** - 2730 líneas en session.rs, alta cohesión
2. **Extraer `jcode-server`** - Usar `src/server/` submodule existente
3. **Extraer `jcode-agent`** - Unificar loop del agente con agent-runtime
4. **Dividir `ui_messages.rs`** - Widget de 1848 líneas necesita subdivisión

### 7.2 Reducir invalidación

- Evitar cambios en `jcode-message-types` y `jcode-provider-core` (alto fan-out)
- Mover herramientas de alta rotación fuera del registry principal
- Mantener `src/agent.rs` contenido (30.9s warm build es significativo)

### 7.3 Métricas a seguir

- Warm `cargo check` target: **< 5s** para edits comunes
- Warm self-dev build target: **< 20-30s**
- Crate count actual: 50 → objetivo: 60-70 (no más, evitar fragmentation)

---

## 8. Conclusión

El proyecto jcode ha logrado un **progreso significativo** en modularización con 50 crates extraídos y fronteras bien diseñadas para providers y features pesadas. El trabajo pendiente se concentra en:

1. **Session domain** - siguiente extracción más obvia
2. **Server/Agent separation** - continuando la descomposición interna
3. **TUI app state** - estado/reducer split aún pendiente

La arquitectura objetivo de workspace en capas está bien definida en los documentos y el progreso es consistente con los planes documentados. El riesgo principal es la tendencia natural a耦合 cruzada cuando los módulos crecen sin revisión de dependencias.