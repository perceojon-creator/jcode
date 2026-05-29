# jcode-tui Refactor — 13 Fases (Plan Detallado)

Este directorio contiene el plan completo y la documentación operativa para desfragmentar el monolito `jcode-tui` en **13 módulos por dominios funcionales** usando un equipo de 5 agentes IA coordinados.

## Contenido

| Archivo / Carpeta                    | Qué es |
|--------------------------------------|--------|
| `PLAN.md`                            | Plan maestro: las 13 fases, orden estratégico, roles de agentes y reglas de coordinación |
| `PLANTILLA_FASE.md`                  | **Checklist reutilizable de 25 pasos** que se debe copiar y seguir en **cada** fase |
| `00-README-AGENTES.md`               | Guía rápida para cualquier agente que participe |
| `metrics/before-after.md`            | Tabla para medir el impacto real (se actualiza al final de cada fase) |
| `fase-01-tui-types/`                 | Ejemplo de carpeta de fase (con `CHECKLIST.md` ya copiado de la plantilla) |
| `fase-XX-.../` (futuras)             | Cada fase tendrá su propia carpeta con su checklist rellenado + retrospectiva |

## Cómo empezar (para minimax o cualquier agente)

1. Lee primero `PLAN.md` (visión general + las 13 fases).
2. Lee `PLANTILLA_FASE.md` (el contrato operativo de 25 pasos).
3. Para la fase actual, copia `PLANTILLA_FASE.md` dentro de su carpeta como `CHECKLIST.md`.
4. Sigue la plantilla paso a paso. Nunca marques un paso como hecho sin evidencia verificable.

## Principios clave

- El TUI **siempre** debe poder compilar y arrancar después de cambios significativos.
- Selfdev, swarm, overnight, improve y review son sagrados.
- El Revisor tiene poder de veto por acoplamiento o riesgo.
- Se avanza en rodajas pequeñas que compilen.

---

**Objetivo final**: Reducir drásticamente el tamaño de los tres monolitos actuales (~100k+ LOC cada uno) y hacer que el desarrollo futuro con selfdev sea sostenible.

Este material fue generado para ser usado directamente por un flujo de 5 agentes (Arquitecto/minimax + 2 Ejecutores + Fixer + Revisor).