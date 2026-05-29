# Guía Rápida para los 5 Agentes — Refactor jcode-tui (13 Fases)

Este documento es el punto de entrada para cualquier agente (incluido minimax) que participe en el refactor.

---

## Los 5 Roles (recordatorio ultra-corto)

| Rol          | Quién suele ser          | Responsabilidad principal                              | Puede vetar |
|--------------|--------------------------|---------------------------------------------------------|-------------|
| Arquitecto   | **minimax**              | Diseño de APIs, límites de dominio, orden de fases     | Sí (fuerte) |
| Ejecutor A   | Agente implementador     | Mover la mayor parte del código, hacer que compile     | No          |
| Ejecutor B   | Agente implementador     | Tests, harnesses, documentación interna                | No          |
| Fixer        | Agente especialista      | Resolver borrow checker, lifetimes, glue code horrible | No          |
| Revisor      | Agente de calidad        | Coupling, encapsulamiento, riesgo selfdev/swarm        | **Sí** (el más importante) |

---

## Flujo Obligatorio por Fase (resumen de 30 segundos)

1. **minimax** elige la siguiente fase según `PLAN.md`.
2. Se copia `PLANTILLA_FASE.md` → `fase-XX-nombre/CHECKLIST.md`.
3. Se rellena la sección de metadatos.
4. Se ejecutan **Pasos 1-5** (Pre-work). **NO se toca código de producción** hasta que el Revisor + minimax aprueben `API_DESIGN.md`.
5. Ejecutores + Fixer ejecutan Pasos 6-18 en iteraciones pequeñas que compilen.
6. Revisor hace Pasos 19-21 (puede haber múltiples rondas).
7. Se completa Paso 25 (Gate de Aceptación firmado por minimax).
8. Se escribe `RETROSPECTIVE.md`.
9. Solo entonces se pasa a la siguiente fase.

---

## Dónde Está la Información

- Visión general + las 13 fases → `PLAN.md`
- El checklist detallado de 25 pasos (tu guía día a día) → `PLANTILLA_FASE.md` (cópiala por fase)
- Lo que realmente pasó en cada fase → `fase-XX-nombre/RETROSPECTIVE.md` y `REVIEW_NOTES.md`
- Decisiones técnicas locales de la fase → `fase-XX-nombre/decisions/`
- Métricas de impacto → `metrics/before-after.md`

---

## Reglas de Comunicación entre Agentes

- Usa **archivos** dentro de la carpeta de la fase para handoffs (no confíes solo en el chat de la sesión).
- Cuando un Ejecutor termina su parte, **siempre** actualiza `HANDOVER_EJECUTORES.md`.
- El Revisor **nunca** aprueba en el chat sin dejar evidencia escrita en `REVIEW_NOTES.md`.
- Si algo te hace dudar sobre selfdev/swarm/overnight → **para todo** y llama a minimax inmediatamente.

---

## Comandos que Deberías Tener a Mano

```bash
# Verificación mínima antes de cualquier claim de "funciona"
cargo check -p jcode-tui --release 2>&1 | head -50

# Si usas los perfiles custom del proyecto
cargo check -p jcode-tui --profile selfdev

# Buscar referencias rápidas (ajusta el patrón)
rg "mod auth|use.*auth::" crates/jcode-tui/src --type rust | head -30
```

---

## Antipatrones que Este Plan Intenta Evitar

- "Ya lo arreglo en la siguiente fase" (eso nunca pasa).
- Mover 15.000 líneas de golpe y rezar para que compile.
- Dejar que el nuevo módulo importe `App` "temporalmente".
- Cerrar una fase sin que minimax haya probado el TUI real.
- Ignorar las notas del Revisor porque "compila".

---

**Recuerda**: El objetivo final no es tener 13 crates bonitos. El objetivo es que dentro de 3-6 meses puedas seguir haciendo selfdev y mejoras grandes en jcode **sin** que el miedo a tocar el código te paralice.

Bienvenido al refactor. Sé riguroso.