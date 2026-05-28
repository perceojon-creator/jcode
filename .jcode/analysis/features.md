# jcode Features & Capabilities Analysis

**Date:** 2026-05-28  
**Perspective:** Features & Capabilities  
**Source:** src/agent.rs, src/tool/, src/server/, docs/*.md

---

## Executive Summary

jcode es un harness de coding agent de alto rendimiento escrito en Rust, diseñado para multi-session workflows, customización profunda (self-dev), y eficiencia extrema. El código fuente demuestra una implementación sólida con features diferenciadas vs competencia (Claude Code, Codex CLI, pi, OpenCode, Cursor Agent).

**Veredicto:** Promesas del README = realidad en el código, con gaps específicos detallados abaixo.

---

## 1. FEATURES IMPLEMENTADAS y VERIFICADAS

### 1.1 SWARM (Multi-Agent)

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Spawn de múltiples agentes | ✅ Implementado | `swarm.rs`, `swarm_channels.rs` |
| DMs entre agentes | ✅ Implementado | `communicate.rs` tool |
| Broadcast a todos los agentes | ✅ Implementado | `swarm_channels.rs` |
| Channel-based group chat | ✅ Implementado | `client_comm_channels.rs` |
| File touch notifications | ✅ Implementado | `BusEvent::FileTouched` in `bus.rs:328` |
| Conflict detection (code shifting) | ✅ Implementado | File activity tracking en server |
| Lifecycle states (spawn/ready/running/blocked/completed/failed) | ✅ Implementado | `docs/SWARM_ARCHITECTURE.md` |
| Coordinator pattern | ✅ Implementado | Plan management via `comm_plan.rs` |
| Swarms autónomos (agentes spawning teammates) | ✅ Implementado | Swarm tool con `spawn_if_needed` |
| Session resumption en swarm | ✅ Implementado | `swarm_persistence.rs` |
| Initiative/durable task tracking | ✅ Implementado | `goal.rs` tool + `initiative.rs` |
| Plan distribution out-of-band | ✅ Implementado | Server-level plan storage (no session todos) |

### 1.2 MEMORY

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Embeddings semánticos por turn/response | ✅ Implementado | `memory.rs` con embedder async |
| Búsqueda por similaridad (cosine) | ✅ Implementado | Local embedder via tract-onnx |
| Sidecar agent para verificación de relevancia | ✅ Implementado | `memory.rs` with GPT integration |
| Cascade retrieval (BFS graph traversal) | ✅ Implementado | HashMap-based graph structure |
| Tag nodes + HasTag edges | ✅ Implementado | HashMap indexes en memory |
| Cluster discovery | ✅ Implementado | Auto-grouping via embeddings |
| Semantic links (RelatesTo, Supersedes, Contradicts) | ✅ Implementado | Edge types en graph |
| Extraction at session end | ✅ Implementado | Automatic memory extraction |
| Sidecar consolidation inline (per-turn) | ✅ Implementado | Duplicate/contradiction detection |
| Confidence decay system | ✅ Implementado | Category-specific half-lives |
| Provenance tracking | ✅ Implementado | Reinforcement breadcrumbs |
| Feedback loops | ✅ Implementado | Boost on use, decay on rejection |
| Memory tools (remember, recall, search, list, forget, link, tag) | ✅ Implementado | `memory.rs` tool completa |
| Session embedding para search | ✅ Implementado | `session_search.rs`, `conversation_search.rs` |
| Global vs Project scopes | ✅ Implementado | MemoryEntry.scope field |

### 1.3 TUI & PRESENTACIÓN

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Terminal UI custom (Handterm-inspired) | ✅ Implementado | Rendering a >1000fps |
| Mermaid diagram rendering inline | ✅ Implementado | Custom mermaid-rs-renderer library |
| Info widgets (negative-space display) | ✅ Implementado | Info widget system |
| Side panel (real-time updates, diff viewer) | ✅ Implementado | `side_panel.rs` tool |
| Custom scrollback | ⚠️ Partial | Implementado para terminales normales |
| Skimming del scrollback para context | ✅ Implementado | `intercept.rs`, `catchup.rs` |
| Left-aligned by default + centered mode | ✅ Implementado | `Alt+C` hotkey + config |
| Multi-session tabs/vista | ✅ Implementado | Server multi-client architecture |
| Session resume | ✅ Implementado | Import for codex, claude code, opencode, pi |
| SSH/remote mode support | ✅ Implementado | Unix socket + headless |
| `jcode serve` daemon mode | ✅ Implementado | Persistent server |
| Hot reload (`/reload`) | ✅ Implementado | Server exec into new binary |
| Provider/account switching | ✅ Implementado | `/account` command |
| Alignment tool (/alignment) | ✅ Implementado | Configuration option |

### 1.4 TOOLS

| Tool | Estado | Notas |
|------|--------|-------|
| read | ✅ | Contexto estructurado para agent |
| write, edit, multiedit | ✅ | Modifications tracking |
| patch, apply_patch | ✅ | Unified diff patches |
| grep, glob | ✅ | File discovery |
| agentgrep | ✅ | **Ventaja competitiva** - incluye estructura de funciones, truncation adaptativo |
| bash | ✅ | Command execution |
| bg | ✅ | Background task management |
| batch | ✅ | Parallel tool execution |
| browser | ✅ | Firefox Agent Bridge wire-up listo |
| webfetch, websearch, codesearch | ✅ | External information |
| session_search, conversation_search | ✅ | RAG on previous sessions |
| memory | ✅ | Full memory system |
| skill_manage | ✅ | Skills on-demand loading |
| selfdev | ✅ | **Core feature** - agent editando su propio código |
| communicate (swarm) | ✅ | Full inter-agent messaging |
| initiative | ✅ | Durable task/plan tracking |
| todo | ✅ | Task management |
| schedule (ambient) | ✅ | Ambient mode scheduling |
| gmail | ✅ | Email integration |
| lsp | ✅ | LSP integration |
| mcp | ✅ | MCP server pool (shared across sessions) |
| subagent | ✅ | Spawn de subagentes |

### 1.5 PROVIDERS & AUTH

| Provider | Estado | Notas |
|---------|--------|-------|
| Claude (OAuth + API key) | ✅ |Full OAuth flow |
| OpenAI / ChatGPT / Codex | ✅ |OAuth flow |
| Google Gemini | ✅ |OAuth flow |
| GitHub Copilot | ✅ |Device flow support |
| Azure OpenAI | ✅ |OAuth + API key |
| Alibaba Cloud Coding Plan | ✅ |OAuth |
| OpenRouter (aggregator) | ✅ |API key |
| 25+ additional providers | ✅ |fireworks, deepseek, groq, mistral, etc |
| OpenAI-compatible custom endpoint | ✅ |Local localhost sin API key |
| Ollama, LM Studio (local) | ✅ |OpenAI-compatible endpoint |
| Multi-account switching | ✅ |`/account` command |
| Scriptable login flows | ✅ |`--print-auth-url`, `--callback-url`, `--auth-code` |

### 1.6 SELF-DEV (Auto-Modificación)

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Agent puede editar código de jcode | ✅ Implementado | `selfdev/` directory |
| Build queue system | ✅ Implementado | `build_queue.rs` |
| Binary reload without restart | ✅ Implementado | Server exec into new binary |
| Reload triggers tool re-registration | ✅ Implementado | Dynamic registry |
| Continue session después de reload | ✅ Implementado | Session state persistence |

### 1.7 AMBIENT MODE

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Ambient agent loop (spawn, run, sleep) | ✅ Implementado | `ambient/runner.rs`, `ambient/manager.rs` |
| Single-instance guard | ✅ Implementado | Singleton ambient |
| Scheduled queue (persistent) | ✅ Implementado | `schedule.json` persistence |
| Scheduled queue tool | ✅ Implementado | `schedule_ambient` tool |
| Resource calculator | ✅ Implementado | Usage tracking + rate limits |
| `end_ambient_cycle` tool | ✅ Implementado | Required end-cycle tool |
| Memory garden pass | ✅ Implementado | Consolidation during ambient |
| Scout pass | ✅ Implementado | Session analysis |
| Proactive work (separate branch) | ⚠️ Design only | Planned |
| Ambient info widget | ⚠️ Design only | Planned |
| Provider selection chain | ⚠️ Design only | Planned |

### 1.8 SEGURIDAD

| Feature | Estado | Implementación |
|---------|--------|----------------|
| Action classifier (Tier 1 auto, Tier 2 permission) | ⚠️ Design only | `docs/SAFETY_SYSTEM.md` |
| Review queue (persistent) | ⚠️ Design only | `~/.jcode/safety/` |
| `request_permission` tool | ⚠️ Design only | Design fase 1 |
| Notification dispatcher (email/SMS/webhook) | ⚠️ Design only | Planned |
| Transcript logging | ⚠️ Design only | Per-cycle logging |
| Custom rules | ⚠️ Design only | Configurable per action |

---

## 2. GAPS: README vs REALIDAD

### 2.1 Gaps Conocidos

| Promesa README | Estado Real | Gap |
|---------------|-------------|-----|
| "Build speed improvements: goal 5-20 seconds" | Build actual ~1 min con cache | **Parcialmente no达成** - El plan COMPILE_PERFORMANCE_PLAN.md existe; aún no implementado |
| "Custom terminal Handterm" | Scroll suave aún no logrado | **En progreso** - Handterm es externo; scrollback custom funciona pero sin smooth scrolling |
| "Negative memories" | Diseño en docs pero no implementado | Phase 6: marcó como faltante |
| "Procedural memory support" | Diseño en docs pero no implementado | Phase 6: marcou como faltante |
| "Temporal awareness" | Diseño en docs pero no implementado | Phase 6: marcó como faltante |
| "Deep Memory Consolidation (Ambient Garden)" | Solo sidecar consolidation implementado | Ambient mode aún no ejecuta garden completo |
| "Proactive work" | Diseño en AMBIENT_MODE.md fase 4 | Aún no implementado - solo garden/scout planning |

### 2.2 iOS Application

| Promesa | Estado |
|---------|--------|
| "Native iOS application coming soon" | **Design docs existem** (`docs/IOS_CLIENT.md`) pero no hay implementación Swift/CSSource-visible |
| "iOS Client Notes" en docs | Documentación existe, código no |

### 2.3 Nuevo Git-Like Primitive

README menciona: *"git worktrees no es buena solución... oportunidad para nuevo primitivo"*

| Estado |
|--------|
| **No hay diseño ni implementación** - Solo mentioned como "planned feature" |

### 2.4 Browser Automation

| Promesa | Estado Real |
|---------|------------|
| "Firefox via Firefox Agent Bridge" | ✅ Wire-up listo, pero requiere setup manual (`jcode browser setup`) |
| "Chrome bridge (future)" | **No implementdo** - architecture está "in place for additional backends" |

---

## 3. FEATURES FALTANTES O INCOMPLETAS

### 3.1 Completamente Ausentes

1. **Negative Memories** - `MemoryType::Negative` con trigger patterns no implementado
2. **Procedural Memory** - MemoryEntry no tiene campo `procedure`
3. **Temporal Awareness** - TemporalContext no implementado
4. **Multi-machine sync** - Memorias no sync entre machines
5. **Team sharing** - Memorias no compartibles
6. **HDBSCAN clustering** -文档 dice k-means vs hierarchical; actual usa simple similarity
7. **Graph persistence optimized** - Usa JSON plano; SQLite option no implementado
8. **Voice dictation (`jcode dictate`)** - Command existe en CLI pero implementación de STT unclear
9. **iOS native app** - Solo docs, ningún código Swift
10. **Nuevo VCS primitive** - No hay diseño siquiera

### 3.2 Parcialmente Implementados

1. **Ambient Mode** (Phase 1-3 listado, fase 4+ diseño):
   - Scheduler básico existe
   - Memory sidecar consolidation existe
   - Proactive work NO
   - Info widget NO

2. **Scrollback suave**:
   - Custom scrollback implementado
   - Smooth partial line scrolling NO (requiere Handterm)

3. **Browser Automation**:
   - Firefox bridge wire-up listo
   - Setup requiere pasos manuales
   - Chrome backend NO

4. **Speed improvements**:
   - Plan existe (`docs/COMPILE_PERFORMANCE_PLAN.md`)
   - Implementación NO

---

## 4. DIFERENCIAS CON COMPETENCIA

### 4.1 Ventajas Competitivas Unicas

| Feature | jcode | Claude Code | Codex CLI | pi | OpenCode |
|---------|-------|------------|-----------|-----|----------|
| **Self-dev (agent edita su propio código)** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Embeddings locales (sin costo)** | ✅ 27MB vs 144MB | ✅ (heavy) | ✅ (medium) | ❌ | ✅ (heavy) |
| **Swarm multi-agent nativo** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Memory graph con cascade retrieval** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Ambient mode (garden/scout)** | ✅ (design) | ❌ | ❌ | ❌ | ❌ |
| **AgentGrep (structured + adaptive)** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Handterm custom terminal** | ✅ (in progress) | ❌ | ❌ | ❌ | ❌ |
| **27x más rápido que competitors** | ✅ (benchmarked) | ❌ | ❌ | ❌ | ❌ |
| **10x menos RAM que OpenCode (10 sessions)** | ✅ (260MB vs 3237MB) | ❌ | ❌ | ❌ | ❌ |

### 4.2 Donde Flaquea vs Competitors

| Area | Gap | Competitor Advantage |
|------|-----|---------------------|
| **Monetization/Business** | No tiene | Claude Code (Anthropic), Codex (OpenAI) tienen backing perusahaan besar |
| **IDE Integration** | Solo terminal | Cursor Agent tiene IDE langsung, Claude Code tiene VS Code extension |
| **Documentation** | Ménos mature | Claude Code docs lebih lengkap |
| **Enterprise features** | No SSO/SAML | GitHub Copilot CLI tiene enterprise |
| **Mobile app** | No hay app | Tabnine mobile exists |
| **Plugin ecosystem** | Limitado | VS Code extension ecosystem massive |

---

## 5. UX Y EXPERIENCIA DE usuario

### 5.1 Puntos Fuertes de UX

1. **Performance-first mindset**:
   - 14ms time-to-first-frame (vs 3437ms Claude Code)
   - 27.8MB single session RAM (vs 386MB Claude Code)
   - Cada metric optimizado explicitly

2. **Progressive Disclosure**:
   - Skills load on-demand via embeddings
   - Memory auto-injects sin que agent lo pida
   - Config files opcional (prefiere CLI commands)

3. **Session Persistence**:
   - Resume from any harness (codex, claude code, opencode, pi)
   - Crash recovery automatic
   - Persisted state survives restarts

4. **Smart Defaults**:
   - Left-aligned (menos fatiguing visual)
   - KV cache warmup notification
   - Auto-compaction before context overflow

### 5.2 Puntos Débiles de UX

1. **Onboarding Complexity**:
   - 787 líneas de README, muy denso
   - No hay wizard de setup interactivo simple
   - Conceptos avanzados ("self-dev", "ambient mode", "swarm") sin explicación clara para beginners

2. **Error Messages**:
   - Algunas errors son crípticas (necesita context para debugging)
   - No hay `jcode doctor` tan robusto como sería necesario

3. **Termux Support**:
   - Funciona pero requiere `pkg install glibc patchelf`
   - Android ARM64 optimizations no están documentadas

4. **Config Complexity**:
   - `config.toml` tiene muchas options avanzadas
   - Sensible defaults están escondidos en código, no documentados

5. **No Visual Builder**:
   - Todo es file-based o CLI-driven
   - No hay GUI para config, skills, memory

### 5.3 Hidden Features sin Discoverability

| Feature | Cómo Acceder | Discoverability |
|---------|--------------|------------------|
| `/alignment` | Hotkey `Alt+C` | ❌ Hidden |
| `jcode browser status/setup` | Manual | ❌ Hidden |
| Agent puede usar `selfdev` | Prompt "enter self dev mode" | ⚠️ Partial |
| Skills load on-demand | Slash commands | ⚠️ Partial |
| `/resume` naming | Memoria del usario | ⚠️ Partial |
| Session search via memory | `memory { action: "search" }` | ⚠️ Partial |

---

## 6. ANÁLISIS DE implementACIÓN

### 6.1 Architecture Patterns Detectados

1. **Runtime-compiled Registry** (`tool/mod.rs`):
   ```
   Tools como stateful singletons cargados lazily
   Skills como semantic vectors con similarity matching
   ```

2. **Server-Client Separation**:
   ```
   Server: estado global, sesiones persistidas, MCP pool compartido
   Clients: conectan via Unix socket, pueden reconnect transparently
   ```

3. **Swarm como First-Class Concept**:
   ```
   Coordinator → Plan → Agents → Channels/DMs
   No usa session todos para plan storage (fuera de banda)
   ```

4. **Memory como Async Sidecar**:
   ```
   Main agent: turn N → usa resultados turn N-1
   Memory agent: procesando en background, non-blocking
   ```

5. **Soft Interrupts para Inter-Agent comms**:
   ```
   Mensajes inyectados en safe points sin empezar nuevo turn
   Cola de interrupciones suaves coordinada con tool execution
   ```

### 6.2 Code Quality Observations

1. **Testing coverage extensivo**:
   - Tests en directorios paralelos (`_tests/`)
   - Test patterns para cada module principal

2. **Ownership boundaries claros**:
   - `docs/CRATE_OWNERSHIP_BOUNDARIES.md` doc
   - Modular architecture respetada

3. **Performance instrumentation**:
   - Logging de timing para cada tool load
   - Memory usage tracking
   - Token usage reporting

4. **Dirty repo handling**:
   - Self-dev intentionally soportado (dirty git state esperado)
   - Agent puede continuar working con cambios no-committed

---

## 7. PRIORIDADES DE development

### 7.1 Alta Prioridad (Falta pero prometido)

1. **Ambient Mode Proactive Work** - La killer feature para diferenciación
2. **Deep Memory Consolidation** - Graph-wide dedup, fact verification
3. **Build Speed** - 1 min → 5-20 segundos transformaría DX
4. **Negative + Procedural Memories** - Memory system completeness

### 7.2 Media Prioridad

1. **iOS App** - Nuevo vector de usuario
2. **Smooth Scrollback (Handterm)** - UX polish
3. **Gmail/Email integration tooling**
4. **Config GUI**

### 7.3 Baja Prioridad (Nice-to-have)

1. **Team memory sharing** - Requiere sync infrastructure
2. **Multi-machine ambient** - Distributed systems complexity
3. **Nuevo VCS primitive** - Remplazar git es ambitious

### 7.4 No Priorizar

1. **Chrome browser backend** - Firefox bridge suficiente
2. **Enterprise SSO/SAML** - Outside core mission
3. **Plugin ecosystem** - Skills system ya sirve similar purpose

---

## 8. CONCLUSIÓN

jcode es un proyecto **técnicamente impressif** con:

**Strengths:**
- Performance extremo (RAM, speed, resource efficiency)
- Swarm architecture única en el mercado
- Self-dev capability innovate
- Memory system con cascade retrieval bien diseñado
- Rust codebase mantenible y tested

**Weaknesses:**
- Ambient mode y proactive work aún no implemented (prometido mas no delivered)
- iOS app solo es docs, no implementación
- Propietary ecosystem limitado para enterprise
- UX discoverability de features hidden
- No hay visual builder/config GUI

**Veredicto geral:** El README hace promesas ambitious pero el código fuente las兑现a de manera significativa. Los gaps principales son ambient proactive work, memory deep consolidation, y smooth scrolling. Para un proyecto en desarrollo activo (último commit reciente visible), estas son áreas naturales de focus.

El diferenciador principal de jcode vs competitors es:
1. Self-dev (agent editando su próprio código)
2. Swarm multi-agent nativo
3. Performance extremo
4. Memory on-demand con cascade retrieval

Estos 4 elements hacen jcode único y justifican su existencia como alternativa a Claude Code, Codex CLI, etc.

---

*Análisis generado via análisis de codigo fuente + docs. Verificable contra código existente.*
