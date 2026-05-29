# Ola 4 + Master Completion Plan (Server Service Split)

**Status**: ACTIVE — Post-Ola 3 Stabilization (Ola 4 #1 fully delivered, both profiles green as of 2026-05-29/30)

**Orchestrator**: Grok (current session) + specialized sub-agents

**Master Goal**: Drive the 9 Fase 1 Entry Points to "complete" (or ≥90% with clear remaining slices) and finish the core of `SERVER_SERVICE_SPLIT_PLAN.md` (especially Move 6 100% + Provider facade expansion + Reload hardening). This is the "plan maestro completo" for the Server layer refactor.

**Baseline (post-Ola 3 + our stabilization)** — authoritative from `Fase0_Baseline_Report.md:334` + recent work:
- #1 Server Service Handles: 95%
- #2 Memory-Types Purity: 100%
- #3 TUI Hotspots: 52%
- #4 Compaction/Memory Core: 45%
- #5 Provider Runtime Facade: 12% (first safe slice only)
- #6 Swallowed Error Hygiene: 85%
- #7 Reload Hardening: 28%
- #8 Test Augmentation on Hot Paths: 27%
- #9 Build/Compile Hygiene: ~80% (selfdev + default both green after Ola 4 #1)

**Overall Philosophy** (from SPLIT_PLAN + all prior Olas):
- Zero new crates in this phase.
- Ruthlessly safe incremental slices (passthroughs first, then ownership moves).
- Every change must pass: `cargo check -p jcode --lib` (default + selfdev), fast test subset (2802 tests), boundary.py, relevant E2E (reload family, binary_integration).
- Commit small + focused after every verified slice.
- Update `ORCHESTRATION_STATUS.md` + `Fase0_Baseline_Report.md` entry point table after every wave.
- Use the Ola wave model (parallel non-overlapping agents + strong verifier/closure gate at end of each Ola).

---

## Ola 4 Charter (Ranked — from ORCHESTRATION_STATUS.md)

1. **Complete SPLIT_PLAN Move 6 100%** (monitor_bus full collapse) — highest leverage remaining.
2. **Provider Runtime Facade expansion** (slices #2/#3 — migrate real call sites + add catalog/failover/auth surface).
3. **Reload E2E full ratchet + collapse** (put every remaining high-blast path behind the documented MaintenanceServiceHandle methods + enforce gates).
4. Parallel hygiene acceleration (TUI, swallowed budget, tests on new seams, build hygiene).

**DoD for declaring "Master Plan Complete" (end of Ola 4 / start of Ola 5)**:
- Entry #1 ≥ 98% (monitor_bus no longer has raw 12+ param list; all mutations go through thin handle methods; state ownership migrated where safe).
- Entry #5 ≥ 50% (ProviderServiceHandle has ≥ 8-10 methods; ≥ 8-10 production call sites exclusively using `services.provider()`).
- Entry #7 ≥ 60% (remaining reload bodies collapsed; E2E gate is mechanically enforced; full family green on every touch).
- All other entries measurably improved (no regressions).
- Full verification suite (default + selfdev check, 2802 fast tests, boundary x2, reload E2E family + scripts/test_reload.py, Windows lifecycle) green with zero new panics/boundary violations.
- Clean Ola 4 closure synthesis in ORCHESTRATION_STATUS.md + updated Fase0 table with real % + agent traces.
- Swallowed error budget + warning budget re-measured and improved.

---

## Ola 4 Wave Structure (Parallel + Sequential)

### Wave 4.1 — Move 6 Completion (monitor_bus 100% behind MaintenanceServiceHandle)
**Lead Agent**: Move6CollapseLead  
**Parallel sub-agents** (non-overlapping scopes):
- FileTouchExtractor (record_file_touch + reverse index mutations)
- SwarmStateInMonitor (get_swarm_sessions, membership queries inside the loop)
- EventRecordingExtractor (record_swarm_event paths + notification fanout)
- MonitorBusParamCollapse (final signature narrowing + state ownership move into handle or dedicated MaintenanceRuntime)
- MonitorBusVerifier (global checks + targeted tests after each sub-slice)

**Key files**: `src/server.rs` (monitor_bus + call sites), `src/server/handles.rs` (new thin methods), `src/server/background_tasks.rs`, `src/server/client_session.rs` (any cross calls), tests.

**Sub-wave gates** (every sub-agent must deliver before next):
- cargo check (both profiles) green
- Relevant unit tests green
- No new swallowed errors introduced
- Doc comment + E2E gate updated on any new handle method

**Target**: Entry #1 to 98-99%, monitor_bus body dramatically smaller, zero direct map writes from the maintenance loop.

### Wave 4.2 — Provider Facade Expansion (Slice #2 + #3)
**Lead**: ProviderFacadeExpander

- Migrate real call sites in Session / client_lifecycle / provider_control / debug to `services.provider()`
- Add next methods on ProviderServiceHandle (catalog, failover targets, auth refresh coordination, model listing, etc.)
- Wire in more places (runtime, startup, etc.)
- Zero behavior change until ownership is ready to move.

**Target**: Entry #5 to 45-55%

### Wave 4.3 — Reload E2E Full Ratchet
**Lead**: ReloadRatcher

- Collapse remaining reload bodies (server_reload_starting, marker handoff, recovery, etc.) behind the existing 5+ thin methods + any new ones needed.
- Add the 1-2 unit tests required by the documented E2E gate.
- Mechanically enforce the gate (script or CI hook later).
- Full family (binary_integration + test_reload.py + Windows) green on every change.

**Target**: Entry #7 to 60%+

### Parallel Hygiene Waves (run alongside 4.1-4.3 where safe)
- TUIHotspotWave (continue ui_messages god-fn reduction, more seams in ui_diff / ui_messages)
- SwallowedBudgetPusher (convert remaining high-volume swallows using emit_best_effort_* helpers + budget JSON ratchet)
- TestAugmentor (add tests on every new thin seam created in 4.1-4.3)
- BuildHygieneAgent (warning budget, selfdev profile improvements, compile time tracking)

---

## Agent Roster (Specialized Sub-Agents to Spawn)

I (the orchestrator) will spawn these using the subagent system with:
- Clear non-overlapping mandate
- Reference to this plan + ORCHESTRATION_STATUS.md + Fase0 table + SPLIT_PLAN Move 6 section
- Strict "cargo check + fast tests + boundary must be green before you declare done"
- Instruction to commit small focused changes + update docs on landing

**Core Execution Agents**:
1. Move6MonitorBusLead (orchestrates 4.1 sub-waves, owns the overall monitor_bus collapse)
2. FileTouchExtractor
3. SwarmStateMonitorAgent
4. EventRecordingExtractor
5. ProviderFacadeExpander (Wave 4.2)
6. ReloadRatcher (Wave 4.3)
7. TUIHotspotAgent (parallel)
8. VerificationGateAgent (cross-cutting — runs the full gate after every major slice, updates Fase0 table)

**Support**:
- DocSyncAgent (keeps ORCHESTRATION_STATUS + Fase0 + this plan in sync after each wave closure)
- BudgetMeasurer (runs swallowed + warning + size budget scripts at wave boundaries)

**Closure Role** (like previous Olas):
- Ola 4 Verifier + Integrator + ClosureCoordinator (final gate — 25-30 min timebox, synthesizes all metrics, declares Ola 4 delivered or lists precise remaining slices, updates authoritative table).

---

## Execution Rules (Mandatory for all agents and me)

- Every code change → `cargo check -p jcode --lib` (default + selfdev) must pass before commit.
- After any handle method addition or monitor_bus edit → run the fast test subset + boundary.py.
- Small focused commits (one logical slice or one method + wiring + test).
- Update the live 9-entry table in Fase0_Baseline_Report.md with real % + evidence after every wave.
- Append progress blocks under the correct Ola 4 section in ORCHESTRATION_STATUS.md.
- Never regress an already-green E2E or boundary.
- When a wave/sub-wave is done: create a clear "Wave X.Y Closure" block with metrics, agent IDs (if applicable), verification output, and next ranked steps.

---

## Verification Cadence

After every major sub-wave and at Ola 4 closure:
- Full `cargo check` (both profiles)
- `cargo test --lib --bins -- --test-threads=1` (2802 tests target)
- `py -3 scripts/check_dependency_boundaries.py` x2
- `scripts/check_swallowed_error_budget.py`
- Reload E2E family (`cargo test -p jcode --test binary_integration` + `scripts/test_reload.py`)
- Windows lifecycle smoke (if on Windows or via remote)
- Update budgets + Fase0 table

---

## Success Criteria for "Plan Maestro Completo"

When Ola 4 ClosureCoordinator declares it delivered:
- All 9 entry points have clear "complete or near-complete with documented remaining low-risk slices".
- Move 6 is 100% (no more raw 12-param monitor_bus doing direct mutations).
- Provider facade is the dominant access path for provider concerns.
- Reload paths are behind the documented gate with mechanical enforcement.
- Swarm model has proven it can finish a major master plan slice safely and measurably.
- Clean handoff artifacts for Ola 5 (whatever comes next — possibly real crate splits, full provider extraction, TUI completion, etc.).

---

**Next Immediate Action (this session)**:
Orchestrator creates this plan → spawns first wave agents (starting with Move 6 sub-agents) → tracks via todo list + ORCHESTRATION updates → commits + verifies after each landing.

This document + the live ORCHESTRATION_STATUS.md + Fase0 table are the single source of truth for the remainder of the master plan.

**Created**: 2026-05-30 by Grok (post full Ola 4 #1 stabilization).
**To be updated** after every wave by the orchestrator + DocSyncAgent.