# PLANTILLA DE FASE (Checklist de 25 Pasos Reutilizable)

> **INSTRUCCIÓN OBLIGATORIA**:  
> Para **cada una** de las 13 fases, copia este archivo completo a `fase-XX-nombre/CHECKLIST.md` y rellénalo paso a paso.  
> No marques un paso como completado hasta que la evidencia sea verificable (compilación limpia + tests + revisión del Revisor).  
> Este documento es la fuente de verdad operativa para los 5 agentes.

---

## Metadatos de la Fase (rellenar al inicio)

- **Fase N°**: ___
- **Nombre del Módulo Objetivo**: `jcode-tui-____________`
- **Dominio Principal**: _______________________________________________
- **Archivos Fuente Principales que se van a extraer**: (lista)
- **Fecha de inicio**: ___________
- **Agentes asignados**:
  - Arquitecto (minimax): ___________
  - Ejecutor A: ___________
  - Ejecutor B: ___________
  - Fixer: ___________
  - Revisor: ___________
- **Branch / Worktree**: `refactor/fase-XX-...`

---

## 0. Principios Inquebrantables de Esta Refactor (léelos cada vez)

1. **El TUI siempre debe poder arrancar** después de cualquier commit significativo. Si `cargo run` (release o selfdev) no abre el TUI funcional, la fase está bloqueada.
2. **Selfdev, swarm, overnight, improve y review commands son sagrados**. Cualquier fase que los toque requiere verificación manual explícita de minimax antes de cerrar.
3. **Reducción medible de acoplamiento**. El Revisor puede vetar aunque compile si el nuevo módulo todavía depende de 15 detalles internos de `App`.
4. **Compilación incremental primero**. Nunca hagas un cambio masivo que rompa `cargo check` por 2 horas. Avanza en rodajas que compilen.
5. **Los `#[path = "..."]` hacks deben morir**. Cada vez que veas uno, es una oportunidad de limpiar durante la extracción.
6. **Documenta las decisiones feas**. Si tuviste que dejar un glue code horrible "para que funcione ya", escríbelo en `RETROSPECTIVE.md` con fecha y plan de limpieza futura.

---

## FASE PRE-WORK (Pasos 1-5) — Lidera: Arquitecto (minimax)

### Paso 1: Análisis de Responsabilidades del Dominio Objetivo
**Objetivo**: Entender exactamente qué le pertenece a este módulo y qué NO.

**Acciones obligatorias**:
- [ ] Leer los 3-5 archivos más grandes relacionados con el dominio (usar `smart-explore` o grep estratégico si está disponible).
- [ ] Listar todas las **structs públicas**, **enums**, **traits** y **eventos** que claramente pertenecen a este dominio.
- [ ] Listar todo lo que **NO** pertenece (cosas que parecen relacionadas pero son de otro dominio).
- [ ] Identificar los 3-5 puntos de acoplamiento más peligrosos con el resto del `App` (campos de `App` que se tocan directamente, métodos `&mut self` gigantes, etc.).
- [ ] Crear el archivo `fase-XX/ANALISIS_RESPONSABILIDADES.md` con la tabla:
  | Responsabilidad | Pertenece aquí? | Justificación | Archivo(s) actual(es) |

**Agente responsable**: Arquitecto  
**Entregable**: `ANALISIS_RESPONSABILIDADES.md` (mínimo 1 página)  
**Gate**: Arquitecto marca "listo" solo cuando el Revisor ha leído y está de acuerdo con los límites del dominio.

---

### Paso 2: Diseño de la API Pública del Nuevo Módulo
**Objetivo**: Definir **antes de escribir código** qué va a exponer el nuevo crate a los demás.

**Acciones obligatorias**:
- [ ] Escribir `fase-XX/API_DESIGN.md` con:
  - Los tipos que serán `pub` (y por qué).
  - Los traits que se expondrán para que otros módulos puedan integrarse (ej: `AuthProvider`, `CommandHandler`, `SessionStore`).
  - Los eventos que emitirá (si usa el bus o un canal propio).
  - Lo que **NO** expondrá (reglas de encapsulamiento).
- [ ] Identificar dependencias cross-cutting que el nuevo módulo necesitará:
  - `jcode-tui-types` (errores, ids, etc.)
  - `crate::config`
  - `crate::bus`
  - `crate::telemetry`
  - Logging
  - Cualquier otro
- [ ] Proponer la estructura de carpetas interna del nuevo crate (`src/domain/`, `src/infra/`, `src/api/`, etc.).
- [ ] Definir la estrategia de **feature flags** si el módulo tendrá partes opcionales (replay, test-harness, etc.).

**Agente responsable**: Arquitecto (con input de Ejecutor A)  
**Entregable**: `API_DESIGN.md` aprobado  
**Gate**: **Revisor + minimax deben aprobar explícitamente** este documento **antes** de que cualquier Ejecutor escriba código de producción. Si no hay aprobación, se itera.

---

### Paso 3: Mapeo de Dependencias y Puntos de Dolor
**Objetivo**: No llevarnos sorpresas a la mitad de la extracción.

**Acciones obligatorias**:
- [ ] Correr `cargo tree -p jcode-tui -i <algún tipo del dominio>` para ver quién depende de qué.
- [ ] Buscar todas las referencias a los archivos que vamos a tocar (`grep -r "mod auth"`, `grep -r "use super::auth"`, etc.).
- [ ] Identificar usos de `super::*` o `use crate::tui::app::*` que van a romperse.
- [ ] Listar tests que ejercitan este dominio (unitarios + integración).
- [ ] Identificar si hay código generado o macros que referencian estas partes.
- [ ] Documentar en `DEPENDENCIAS_Y_PUNTOS_DE_DOLOR.md`.

**Agente responsable**: Ejecutor B (tests) + Fixer  
**Entregable**: `DEPENDENCIAS_Y_PUNTOS_DE_DOLOR.md`

---

### Paso 4: Estrategia de Transición (cómo no romper todo)
**Objetivo**: Tener un plan de cómo vamos a mover código sin dejar el TUI roto por días.

**Decisiones que se deben tomar aquí**:
- [ ] ¿Vamos a crear el nuevo crate desde el día 1 y dejar el código viejo como "deprecated facade" durante la transición?
- [ ] ¿O vamos a extraer dentro del mismo crate primero (nuevo módulo `mod foo { ... }`) y convertirlo en crate después?
- [ ] Estrategia de re-exports temporales para no romper 200 imports de golpe.
- [ ] ¿Cómo vamos a manejar el estado compartido que hoy vive dentro de `App` (campos que se mutan desde muchos lados)?
- [ ] Plan para los `#[path = "..."]` hacks que existen en los archivos gigantes.

**Agente responsable**: Arquitecto + Fixer  
**Entregable**: Sección "Estrategia de Transición" dentro de `API_DESIGN.md` o documento separado.

---

### Paso 5: Setup del Entorno de la Fase
**Acciones obligatorias**:
- [ ] Crear el directorio `fase-XX-nombre/` con la estructura estándar:
  ```
  fase-XX-nombre/
  ├── CHECKLIST.md                 (esta plantilla copiada y renombrada)
  ├── ANALISIS_RESPONSABILIDADES.md
  ├── API_DESIGN.md
  ├── DEPENDENCIAS_Y_PUNTOS_DE_DOLOR.md
  ├── HANDOVER_EJECUTORES.md       (se actualiza cada vez que un ejecutor termina su parte)
  ├── REVIEW_NOTES.md              (el Revisor escribe aquí)
  ├── RETROSPECTIVE.md             (al final de la fase)
  └── decisions/                   (ADRs pequeños de la fase)
  ```
- [ ] Crear la rama o worktree dedicada.
- [ ] Actualizar el `Cargo.toml` del workspace (solo si ya se decidió crear el crate en esta fase; muchas veces se crea vacío primero).
- [ ] Verificar que `cargo check -p jcode-tui` (o el perfil correspondiente) pase limpio **antes** de tocar nada.

**Gate**: Todo lo anterior completado y verificado. Arquitecto da luz verde para empezar a mover código.

---

## FASE DE EJECUCIÓN (Pasos 6-18) — Lideran: Ejecutor A + Ejecutor B + Fixer

### Paso 6: Crear el Esqueleto del Nuevo Crate/Módulo
- [ ] Crear `crates/jcode-tui-xxx/Cargo.toml` con las dependencias mínimas identificadas en el Paso 2.
- [ ] Crear `lib.rs` con la estructura de módulos interna acordada.
- [ ] Publicar los primeros tipos puros (value objects, errores, ids) aunque todavía no hagan nada.
- [ ] Hacer que el crate compile solo (`cargo check -p jcode-tui-xxx`).
- [ ] Añadirlo como dependencia del workspace y de `jcode-tui` (con `path`).

**Consejo específico de este codebase**: Empieza siempre con los tipos que **no tienen lógica**. Los tipos puros casi nunca rompen nada.

---

### Paso 7: Mover Tipos y Datos Puros (sin comportamiento)
- [ ] Mover structs/enums que solo contienen datos.
- [ ] Mover traits que son puramente descriptivos.
- [ ] Crear re-exports temporales en el viejo lado si es necesario (`pub use jcode_tui_xxx::Foo as Foo;`).
- [ ] Verificar que `cargo check` sigue pasando después de cada batch de 3-5 tipos movidos.

**Regla**: Si un tipo tiene más de 2-3 métodos con lógica compleja, déjalo para más adelante. Los datos primero.

---

### Paso 8: Extraer Lógica de Dominio Pura (sin efectos secundarios)
- [ ] Mover funciones puras y lógica de negocio que no toquen I/O, red, UI ni el `App`.
- [ ] Si la lógica necesita leer config, inyectar la config como parámetro (no hardcodear `crate::config` dentro del nuevo módulo todavía).
- [ ] Escribir (o mover) tests unitarios para esta lógica.

---

### Paso 9: Definir y Extraer Traits de Abstracción
Este es uno de los pasos **más importantes** para reducir acoplamiento.

- [ ] Identificar qué es lo que el dominio necesita del resto del sistema (ej: "necesito poder guardar una cuenta", "necesito emitir un evento de login completado").
- [ ] Definir traits pequeños y bien nombrados en el nuevo módulo (o en `jcode-tui-types` si son cross-cutting).
- [ ] Implementar esos traits en el lado viejo (glue code) usando los detalles sucios de `App`.
- [ ] El nuevo módulo ahora depende solo de los traits, no de `App`.

**El Revisor prestará atención extrema a este paso.**

---

### Paso 10: Mover Lógica con Efectos Secundarios Controlados
- [ ] Mover código que hace I/O, red, crypto, etc., **solo después** de tener los traits de abstracción.
- [ ] Para el caso específico de `remote/key_handling.rs`: la criptografía y el manejo de claves es muy delicado. Moverlo requiere revisión extra del Fixer + Revisor.

---

### Paso 11: Manejar el Estado Compartido (el problema más duro)
La mayoría de las dificultades vendrán de aquí.

Opciones (elegir conscientemente y documentar):
- Opción A: Mover el estado al nuevo módulo y que `App` tenga solo un `Arc<RwLock<NuevoEstado>>` o similar.
- Opción B: Dejar el estado en `App` por ahora y exponer métodos de "actualización" a través de un trait.
- Opción C: Usar message passing (el bus) para reducir acoplamiento directo.

**Nunca elijas Opción A sin que el Revisor esté de acuerdo.** Es la que más riesgo de romper selfdev tiene.

---

### Paso 12: Actualizar Todos los Call Sites (el trabajo tedioso pero crítico)
- [ ] Cambiar imports en los archivos que siguen dentro de `jcode-tui`.
- [ ] Usar los re-exports temporales para no tener que tocar 80 archivos el mismo día.
- [ ] Cada vez que compiles y algo falle, **arregla inmediatamente** antes de seguir moviendo más código. La deuda de "ya lo arreglo después" explota en este tipo de refactor.

---

### Paso 13: Mover Tests (unitarios primero)
- [ ] Mover tests que solo prueban lógica del nuevo dominio.
- [ ] Si los tests usan mocks o fixtures del viejo `App`, crear versiones locales en el nuevo crate.
- [ ] Ejecutor B es el dueño de este paso.

---

### Paso 14: Mover / Crear Tests de Integración y Harness
- [ ] Los tests que necesitan el TUI completo o el servidor de replay suelen estar en `tests/`.
- [ ] Asegurarse de que el harness de test sigue pudiendo instanciar el nuevo comportamiento.
- [ ] Si el dominio tiene un modo "replay" o "test-harness", asegurarse de que sigue funcionando.

---

### Paso 15: Compilación con Todos los Perfiles
**Acciones obligatorias** (no las saltes):

```bash
# Perfil normal
cargo check -p jcode-tui

# Release (el que más usa el usuario final)
cargo check -p jcode-tui --release

# Si existen los perfiles custom (selfdev / fastdev)
cargo check -p jcode-tui --profile selfdev
cargo check -p jcode-tui --profile fastdev
```

Si alguno falla, **no sigas** hasta que esté verde.

---

### Paso 16: Pruebas Manuales de Funcionalidad Crítica (minimax)
- [ ] minimax debe correr el TUI real con el perfil que usa normalmente.
- [ ] Probar específicamente los flujos que tocan el dominio extraído (login, comandos, etc.).
- [ ] Si la fase toca selfdev/swarm/overnight → prueba explícita de esos comandos.
- [ ] Documentar cualquier anomalía en `REVIEW_NOTES.md`.

---

### Paso 17: Medición de Impacto de la Fase
Actualizar (o crear) en `metrics/before-after.md`:

- Líneas movidas en esta fase (aprox).
- Tamaño actual de los archivos monstruo originales (antes vs después de la fase).
- Tiempo aproximado de `cargo check` relevante.
- Cualquier reducción de complejidad ciclomática o acoplamiento que se pueda medir fácilmente.

---

### Paso 18: Preparación para Revisión Formal
- [ ] Limpiar TODOs y comentarios temporales que ya no aplican.
- [ ] Asegurarse de que todo el código nuevo tiene al menos la misma cantidad de comentarios que el código viejo (idealmente más, porque ahora hay contexto de dominio).
- [ ] Ejecutor A escribe un `HANDOVER_EJECUTORES.md` corto explicando qué decisiones se tomaron y qué trampas quedan para el Fixer/Revisor.

---

## FASE DE REVISIÓN Y CIERRE (Pasos 19-25) — Lidera: Revisor + Fixer

### Paso 19: Revisión de Coupling y Encapsulamiento (Revisor)
El Revisor hace una pasada **exclusivamente** buscando:

- [ ] ¿El nuevo módulo expone detalles de implementación que no debería?
- [ ] ¿Hay imports circulares o acoplamiento temporal creado solo para que compile?
- [ ] ¿Se puede usar el nuevo módulo desde otro contexto sin conocer `App`?
- [ ] ¿Los traits de abstracción son pequeños y con nombres que revelan intención?
- [ ] Escribir hallazgos (con severidad) en `REVIEW_NOTES.md`.

---

### Paso 20: Revisión de Riesgo para Selfdev / Swarm (Revisor + minimax)
- [ ] Si la fase tocó auth, commands, session, remote o selfdev → revisión extra.
- [ ] minimax corre al menos un escenario real de selfdev o swarm si es posible.
- [ ] Cualquier regresión en estos flujos **bloquea** el cierre de la fase.

---

### Paso 21: Fixer Resuelve los Issues del Revisor
- [ ] El Fixer no marca nada como resuelto hasta que el Revisor confirme que ya no es un problema.
- [ ] Si el Revisor vetó por acoplamiento, el Fixer debe proponer una alternativa (puede requerir volver a pasos anteriores).

---

### Paso 22: Segunda Ronda de Compilación + Tests Completos
- [ ] `cargo test -p jcode-tui-xxx` (el nuevo crate).
- [ ] `cargo test` en las partes relevantes del viejo crate.
- [ ] Compilación con todos los perfiles otra vez.
- [ ] Si hay tests de UI o E2E, ejecutarlos.

---

### Paso 23: Documentación Interna y Externa
- [ ] Actualizar el `README.md` del nuevo crate (aunque sea corto).
- [ ] Si el módulo tiene decisiones de diseño no obvias, crear un pequeño ADR en `decisions/`.
- [ ] Actualizar cualquier comentario de "esto vive en X.rs gigante" que ya no sea verdad.

---

### Paso 24: Cierre de Fase y Retrospectiva
**Entregables obligatorios antes de marcar la fase como terminada**:

- [ ] `RETROSPECTIVE.md` con:
  - Qué salió bien
  - Qué fue más doloroso de lo esperado
  - Decisiones técnicas que tomamos y que pueden afectar fases futuras
  - Lecciones para el Revisor y los Ejecutores en las próximas fases
- [ ] Actualización del `PLAN.md` (si algo del orden o alcance de fases siguientes cambió).
- [ ] Métricas actualizadas.
- [ ] Commit limpio con mensaje convencional que mencione la fase.

---

### Paso 25: Gate de Aceptación de la Fase (firma de minimax)

**minimax debe confirmar explícitamente** (puede ser en un comentario en el PR o en el archivo de la fase):

1. El TUI arranca y se puede usar normalmente con el perfil habitual.
2. Los flujos críticos del dominio extraído funcionan (incluyendo selfdev/swarm si aplica).
3. El Revisor dio su aprobación final (sin vetos abiertos).
4. Las métricas de impacto se actualizaron.
5. La retrospectiva está escrita y es útil.

**Solo cuando los 5 puntos anteriores están marcados, la fase se considera cerrada y se puede empezar la siguiente.**

---

## Apéndice A: Comandos de Verificación Frecuentes (cópialos)

```bash
# Compilación rápida de solo este crate
cargo check -p jcode-tui-xxx

# Todo el tui (el que más te va a doler)
cargo check -p jcode-tui

# Con release (recomendado antes de pruebas manuales)
cargo check -p jcode-tui --release

# Si tienes los perfiles custom
cargo check -p jcode-tui --profile selfdev

# Tests del nuevo módulo
cargo test -p jcode-tui-xxx

# Buscar usos de un tipo que estás moviendo
rg "use crate::tui::app::auth" --type rust
```

---

## Apéndice B: Antipatrones Comunes en Este Codebase (evítalos)

- Dejar que el nuevo módulo importe `super::App` o `crate::tui::app::App`.
- Usar `#[path = "..."]` dentro del nuevo crate (estamos huyendo de eso).
- Crear un trait gigante que replica todo lo que hacía `App` (derrota el propósito).
- Mover código con `unwrap()` o `.expect("comentario que explica por qué no puede fallar")` sin pensar si el contexto de error cambió.
- Olvidar que algunos flujos de error y telemetría tienen que seguir reportando exactamente igual.

---

**Esta plantilla es viva.** Después de cada fase, agrega en este mismo archivo (o en un `NOTAS_ADICIONALES.md`) cualquier paso, verificación o lección que haya demostrado ser valiosa y que no estaba contemplada.

---

*Fin de la PLANTILLA DE FASE. Úsala religiosamente. Es tu mejor defensa contra el caos en una refactor de este tamaño.*