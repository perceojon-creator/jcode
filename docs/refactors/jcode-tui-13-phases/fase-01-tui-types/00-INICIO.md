# Fase 01 — jcode-tui-types (Tipos Fundamentales y Errores de Dominio TUI)

**Estado**: Lista para empezar (Pre-work).

Esta es la fase más segura y con menor riesgo de todo el plan. Su objetivo es sentar las bases de tipos puros, errores y eventos que todos los demás módulos van a reutilizar.

## Por qué empieza el refactor aquí

- Casi no tiene lógica ejecutable.
- Muy bajo acoplamiento.
- Permite a los agentes practicar el proceso completo de extracción (incluyendo la plantilla de 25 pasos) sin riesgo real para selfdev.
- Crea el primer "ladrillo" estable sobre el que se construirán los crates de auth, session, remote, etc.

## Alcance probable (sujeto a análisis del Paso 1-2)

- Tipos de ID (`SessionId`, `MessageId`, `TurnId`, `AccountId`, etc.)
- Errores de dominio TUI (`TuiError`, variantes específicas por dominio)
- Eventos de dominio puros (no los de infraestructura)
- Value objects comunes (paths relativos validados, timestamps con semántica, etc.)
- Constantes y configuraciones de bajo nivel que hoy están desperdigadas

## Qué NO debe ir aquí

- Cualquier cosa que haga I/O, red, crypto o UI.
- Lógica de negocio (eso va a los crates de dominio).
- Estado mutable compartido.

## Primeros pasos recomendados

1. Abre `CHECKLIST.md` (es una copia de `PLANTILLA_FASE.md`).
2. Rellena la sección de metadatos.
3. Ejecuta el **Paso 1** (Análisis de Responsabilidades).
4. No toques código todavía.

---

*Cuando termines esta fase, habrás practicado el flujo completo de 5 agentes y tendrás un crate estable que las siguientes 12 fases podrán depender.*