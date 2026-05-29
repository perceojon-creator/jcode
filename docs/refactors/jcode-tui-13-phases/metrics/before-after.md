# Métricas Antes / Después — Refactor jcode-tui a 13 Módulos

Este archivo se actualiza **al final de cada fase** (Paso 17 de la plantilla).

---

## Estado Inicial (Baseline) — Antes de empezar cualquier fase

| Métrica                              | Valor (aprox)          | Notas |
|--------------------------------------|------------------------|-------|
| LOC total de `jcode-tui`             | ~450k-500k+            | Incluye todos los sub-módulos de `app/` |
| Archivo más grande                   | auth.rs ~109k LOC      | + sus `#[path]` companions |
| 2do más grande                       | inline_interactive.rs ~106k |
| 3er más grande                       | remote/key_handling.rs ~106k |
| commands.rs                          | ~103k LOC              | |
| input.rs                             | ~93k LOC               | |
| Número de archivos > 40k LOC         | 10+                    | |
| Tiempo `cargo check` (debug, típico) | Muy variable / a veces falla por stack overflow | |
| Tiempo `cargo check --release`       | 4-8 minutos (estimado usuario) | Depende de la máquina |
| ¿Se puede compilar solo un dominio?  | No                     | Todo está acoplado dentro de `app` |

---

## Por Fase (ir rellenando)

### Fase 01: jcode-tui-types

| Métrica                              | Antes     | Después   | Delta | Fecha |
|--------------------------------------|-----------|-----------|-------|-------|
| LOC movidos                          | -         | ~X k      | +X k  |       |
| Tamaño de auth.rs (si aplica)        | 109k      | 109k      | 0     |       |
| ¿Nuevo crate compila aislado?        | -         | Sí        | -     |       |
| Tiempo check incremental (cambio en types) | -    | Rápido    | Mejor |       |

**Observaciones / Lecciones**:
- (se llena al cerrar la fase)

---

### Fase 02: jcode-tui-auth

(plantilla lista para rellenar)

---

## Estado Final Esperado (después de Fase 13)

| Métrica                              | Objetivo ambicioso     | Objetivo realista     |
|--------------------------------------|------------------------|-----------------------|
| Ningún archivo dentro de jcode-tui   | < 5.000 LOC            | < 8.000 LOC           |
| `App` struct (el dios)               | < 1.000 LOC coordinador| < 1.500 LOC           |
| Crates extraídos                     | 13                     | 11-13                 |
| Tiempo `cargo check` típico (cambio pequeño en auth/commands) | < 30s | < 90s |
| ¿Se puede trabajar en un dominio sin recompilar todo? | Sí (en la mayoría de casos) | Parcialmente |

---

*Este archivo es evidencia objetiva de si el refactor está cumpliendo su propósito o solo estamos moviendo código de un lado a otro.*