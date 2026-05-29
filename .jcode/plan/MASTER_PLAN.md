# MASTER PLAN - Correccion de Debilidades Jcode

**Generado:** 2026-05-28 por 5-agent swarm
**Proyecto:** perceojon-creator/jcode
**Actualizado:** 2026-05-28

---

## Resumen Ejecutivo

| # | Debilidad | Estado | Timeline | Riesgo |
|---|-----------|--------|----------|--------|
| 1 | MiniMax Provider Fix | COMPLETADO | Hecho | Bajo |
| 2 | Build Speed | Pendiente parcial | 2-4 semanas | Medio |
| 3 | Android Support | Pendiente | 1-2 semanas | Medio |
| 4 | Memory Phase 6 | Pendiente | 4-8 semanas | Alto |
| 5 | Handterm Smooth Scroll | Pendiente | 2-3 semanas | Medio |

---

## Completado: MiniMax Provider Fix

MiniMax ya fue corregido. La premisa anterior de forzar `sk-cp-*` a `api.minimaxi.com` era insegura y fue eliminada.

Estado actual:
- Endpoint oficial: `https://api.minimax.io/v1`
- Env var dedicada: `MINIMAX_API_KEY`
- Compatibilidad legacy: `OPENAI_API_KEY` solo dentro de `minimax.env`
- Validado con `jcode -p minimax`

Impacto: jcode puede usar MiniMax desde el provider `minimax`.

---

## Prioridad 2: Build Speed Optimization

Problema:
- Warm cargo check aun necesita medicion actual
- Warm selfdev build aun necesita medicion actual

Solucion:
- Ver `build_speed_plan.md`
- Ver `build_speed_phase2.md`
- Validar con comandos repetibles antes de prometer mejora real

Impacto: ciclo de desarrollo mas rapido.

---

## Prioridad 3: Android Support

Problema:
- Binary desktop requiere glibc
- Termux usa bionic
- `arboard` sigue siendo punto de riesgo

Solucion:
- Ver `android_support_plan.md`
- Validar en CI ARM64/Android

Impacto: desarrollo desde Termux en Android.

---

## Prioridad 4: Memory Phase 6

Problema:
- Negative/procedural/temporal memories siguen pendientes.

Solucion:
- Ver `memory_phase6_plan.md`

Impacto: agentes mas utiles a largo plazo, pero con alto riesgo de complejidad.

---

## Prioridad 5: Handterm Smooth Scroll

Problema:
- Custom scrollback existe, pero smooth scrolling no esta cerrado.

Solucion:
- Ver `handterm_plan.md`

Impacto: mejora UX.

---

## Proximos Pasos

1. Completado: MiniMax.
2. Limpiar/decidir cambios ajenos del working tree.
3. Validar Android en CI.
4. Medir build speed actual.
5. Empezar una extraccion pequena de arquitectura/server/TUI.

---

*MiniMax eliminado de pendientes porque ya esta terminado.*
