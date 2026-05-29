# PLAN DE REFACTORIZACIÓN: jcode-tui → 13 Módulos por Dominios Funcionales (Bounded Contexts)

> **Estado**: Plan maestro listo para ejecución con 5 agentes IA coordinados.  
> **Fecha de creación**: 2026 (sesión actual)  
> **Objetivo principal**: Eliminar los 3 monolitos de ~100k+ LOC (auth.rs, commands.rs, key_handling.rs + inline_interactive.rs + state_ui_*.rs) dividiendo el dios `App` en 13 crates/módulos con límites claros, manteniendo la capacidad de compilar, correr el TUI y usar selfdev/swarm en **todo momento**.

---

## 1. El Problema (sin filtro)

El crate `jcode-tui` es el ejemplo clásico de la peor combinación posible de deuda técnica:

- **Sobre-fragmentación patológica a nivel de workspace**: 56 crates en total, muchos de 1 solo archivo de tipos.
- **3 monolitos gigantes dentro de un solo módulo `app`**:
  - `auth.rs` + sus `#[path]` → ~109k LOC
  - `commands.rs` + subcomandos → ~103k LOC
  - `remote/key_handling.rs` + `inline_interactive.rs` → ~106k LOC cada uno
  - Más de 20 archivos `state_ui_*.rs`, `ui_*.rs`, `tui_*.rs` que también son enormes.

Resultado: compilar debug puede dar stack overflow, los perfiles custom (selfdev/fastdev) tardan mucho, `cargo check` es doloroso, y **selfdev/swarm** (las features más valiosas del proyecto) corren sobre este castillo de naipes.

El objetivo de este refactor **NO** es "hacerlo más bonito". Es **hacer que selfdev y el desarrollo futuro sean sostenibles**.

---

## 2. Estrategia: 13 Módulos por Bounded Contexts (DDD)

No dividimos por capas técnicas (ui / state / network). Dividimos por **dominios funcionales** reales del producto:

| Fase | Nombre del Módulo (futuro crate)          | Dominio Principal                              | Riesgo | Orden | Dependencias clave de fases anteriores |
|------|-------------------------------------------|------------------------------------------------|--------|-------|---------------------------------------|
| 01   | `jcode-tui-types`                         | Tipos puros, errores, eventos de dominio       | Bajo   | 1     | Ninguna                               |
| 02   | `jcode-tui-auth`                          | Autenticación, cuentas, OAuth, suscripciones   | Medio  | 2     | 01                                    |
| 03   | `jcode-tui-input`                         | Parsing de input, comandos slash, dictation    | Medio  | 3     | 01                                    |
| 04   | `jcode-tui-session`                       | Sesiones, turns, mensajes, compaction, rewind  | Medio-Alto | 4  | 01, 02                                |
| 05   | `jcode-tui-remote`                        | Comunicación con servidor jcode, eventos SSE   | Alto   | 5     | 01, 04                                |
| 06   | `jcode-tui-tools`                         | Tool registry, MCP, tool calls, execution      | Alto   | 6     | 01, 04, 05                            |
| 07   | `jcode-tui-navigation`                    | Modos de UI, split view, navegación, pinned    | Medio  | 7     | 01, 04                                |
| 08   | `jcode-tui-state`                         | State machines de UI (state_ui_*)              | Alto   | 8     | 01-07                                 |
| 09   | `jcode-tui-render`                        | Rendering pipeline, layouts, widgets, themes   | Alto   | 9     | 01, 07, 08                            |
| 10   | `jcode-tui-stream`                        | Streaming de respuestas, async events, buffers | Medio  | 10    | 01, 04, 05                            |
| 11   | `jcode-tui-selfdev`                       | Comandos overnight/improve/review/selfdev/swarm| **Crítico** | 11 | 01-10                              |
| 12   | `jcode-tui-lifecycle`                     | Startup, shutdown, replay, test harness, diag  | Alto   | 12    | 01-11                                 |
| 13   | Reducción final del dios `App`            | Coordinador delgado + eliminación de glue      | **Crítico** | 13 | Todas                               |

**Principio de orden**: De menos acoplado y más estable → más riesgoso y con más dependencias. Las fases 11 y 13 son las que más pueden romper selfdev.

---

## 3. Los 5 Agentes y Reglas de Coordinación

### Roles

- **minimax (Arquitecto Principal / Usuario)**:
  - Dueño del plan, aprueba diseños de API pública de cada módulo.
  - Decide cuándo una fase está lista para cerrarse.
  - Puede ejecutar comandos de verificación en su máquina local (el TUI real).

- **Ejecutor A (Implementador principal)**:
  - Hace la mayor parte de la extracción de código.
  - Responsable de que compile en cada paso intermedio.

- **Ejecutor B (Implementador secundario + tests)**:
  - Mueve tests, crea harnesses de integración.
  - Se encarga de la documentación interna del nuevo crate.

- **Fixer**:
  - Resuelve TODOs, errores de borrow checker, problemas de visibilidad y lifetimes que aparecen durante la extracción.
  - Especialista en "hacer que las cosas feas sigan funcionando mientras limpiamos".

- **Revisor (Calidad + Coupling + Seguridad)**:
  - **NO aprueba** si queda acoplamiento innecesario al viejo `App`.
  - Revisa que el nuevo módulo sea usable **sin** conocer detalles internos de otros módulos.
  - Busca riesgos de selfdev/swarm rotos.
  - Revisa performance y uso de memoria (crítico en este proyecto).

### Reglas de Oro de Coordinación (obligatorias)

1. **Nunca se trabaja en Fase N+1 si la Fase N no tiene gate de aceptación firmado por minimax + Revisor**.
2. Cada fase **debe** poder compilar y arrancar el TUI completo (`cargo run -p jcode --release` o el perfil selfdev).
3. Después de cada 3 fases, se hace una "sync de integración" donde todos los agentes revisan que selfdev y los comandos overnight sigan funcionando.
4. Cualquier cambio que toque `App` (el dios) debe pasar por Revisor antes de merge.
5. Se usa **worktree por fase** (o rama dedicada) para que los Ejecutores puedan trabajar en paralelo sin pisarse cuando sea posible.
6. El Revisor **puede vetar** una fase aunque compile. El criterio es: "¿Este cambio hace más fácil o más difícil el futuro desarrollo con selfdev?".

---

## 4. Cómo Usar Este Plan (Flujo de 5 Agentes)

1. **minimax** abre una nueva sesión y carga este `PLAN.md` + la `PLANTILLA_FASE.md`.
2. Para la fase actual, copia la plantilla a `fase-XX-nombre/CHECKLIST.md` y la llena paso a paso.
3. Los agentes se comunican mediante archivos en `fase-XX-nombre/` (handover documents, API proposals, review notes).
4. Al final de cada fase se genera un pequeño `RETROSPECTIVE.md` con lecciones (esto es oro para las fases siguientes).
5. Al terminar las 13 fases se hace un documento de "Estado post-refactor" con métricas reales (LOC por crate, tiempo de compilación, etc.).

---

## 5. Estructura de Archivos Recomendada en `docs/refactors/tui-13-modules/`

```
tui-13-modules/
├── PLAN.md                          ← Este archivo (visión general)
├── PLANTILLA_FASE.md                ← El checklist reutilizable de 25 pasos (OBLIGATORIO por fase)
├── 00-README-AGENTES.md             ← Instrucciones cortas para los 5 agentes
├── fase-01-tui-types/
│   ├── CHECKLIST.md                 ← Copia de la plantilla rellenada
│   ├── API_DESIGN.md
│   ├── RETROSPECTIVE.md
│   └── ...
├── fase-02-tui-auth/
├── ...
├── fase-13-final-reduction/
└── metrics/
    └── before-after.md              ← Se actualiza al final de cada fase
```

---

## 6. Consideraciones Técnicas Específicas de Este Código Base

- Los archivos gigantes usan `#[path = "xxx.rs"] mod xxx;` extensivamente. Esto es un anti-patrón que hay que eliminar durante la extracción.
- Hay perfiles de compilación custom (`selfdev`, `fastdev`). El template debe recordarle a los agentes usarlos.
- `cargo check` en debug puede fallar por stack overflow en algunos entornos → siempre tener release o los perfiles custom como fallback.
- Selfdev, swarm, overnight, improve y review commands son **sagrados**. Cualquier fase que los toque necesita pruebas manuales de minimax.
- El bus de eventos (`crate::bus`) y el `McpManager` son cross-cutting concerns que probablemente vivirán en crates ya existentes o en `jcode-tui-core`.

---

## 7. Criterios de "Éxito Total" del Proyecto de Refactor

Al terminar la Fase 13:

- Ningún archivo `.rs` dentro de `jcode-tui` supera los 8.000 LOC (ideal < 5.000).
- El `App` struct es un coordinador delgado (< 1.500 LOC) que solo orquesta.
- Se pueden compilar y testear módulos individuales de forma razonablemente rápida.
- `cargo run` (con perfil selfdev o release) sigue funcionando idéntico desde el punto de vista del usuario.
- El tiempo de compilación incremental para cambios en auth o commands ha bajado significativamente.
- Los 5 agentes + minimax tienen un proceso probado que pueden repetir en el futuro para otras partes del sistema.

---

**Siguiente paso inmediato**: Generar la `PLANTILLA_FASE.md` ultra-detallada (el checklist de 20-25 pasos que se repite en cada extracción). Ese archivo es el que usarán los agentes como guía operativa en las 13 fases.

---

*Este plan fue creado siguiendo exactamente la solicitud del usuario de tener documentación "bn comentada" y accionable por agentes IA dentro del proyecto.*