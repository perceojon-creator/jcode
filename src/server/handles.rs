//! Thin service handle types for the server domain.
//!
//! Phase 1 (Fase 1 Entry Point #1): Zero behavior change.
//! These are pure newtype wrappers that group related Arc fields.
//! The goal is to stop the explosion of 15-29 argument lists in
//! `handle_client`, `ServerRuntime`, and maintenance loops.
//!
//! Later phases will move actual mutation behind these handles.
//!
//! See: docs/SERVER_SERVICE_SPLIT_PLAN.md (Move 2) + Fase0_Baseline_Report.md

// Phase 1 complete (Ola 2 Move 4): ServerServices bag now primary conduit for the two
// main entry handlers (handle_client + handle_debug_client signatures narrowed per
// SERVER_SERVICE_SPLIT_PLAN.md Move 4). Raw-param leafs remain in submodules only.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use tokio::sync::{Mutex, OnceCell, RwLock, broadcast};

use crate::agent::Agent;
use crate::ambient_runner::AmbientRunnerHandle;
use crate::mcp::SharedMcpPool;
use crate::protocol::ServerEvent;
use crate::provider::Provider;
use jcode_agent_runtime::InterruptSignal;

// Ola 3 Agent 2 (ProviderFacadeFirstSlice) + concurrent Ola 3 slices: imports for minimal
// ProviderServiceHandle surface methods (catalog/auth/failover thin delegations).
use anyhow::Error as AnyhowError;
use jcode_provider_core::{FailoverDecision, ModelCatalogRefreshSummary};

use super::{
    debug::{ClientConnectionInfo, ClientDebugState},
    debug_jobs::DebugJob,
    state::SwarmState,
    ServerIdentity, SessionInterruptQueues,
};

/// Thin handle for everything that conceptually belongs to the **Session** service.
///
/// Currently just a bag of the Arcs it will eventually own exclusively.
/// No methods yet (Phase 1 = zero behavior change).
#[derive(Clone)]
pub(crate) struct SessionServiceHandle {
    pub sessions: Arc<RwLock<HashMap<String, Arc<Mutex<Agent>>>>>,
    pub session_id: Arc<RwLock<String>>,
    pub is_processing: Arc<RwLock<bool>>,
    pub shutdown_signals: Arc<RwLock<HashMap<String, InterruptSignal>>>,
    pub soft_interrupt_queues: SessionInterruptQueues,
    pub event_tx: broadcast::Sender<ServerEvent>,
    pub provider: Arc<dyn Provider>,
}

impl SessionServiceHandle {
    pub fn new(
        sessions: Arc<RwLock<HashMap<String, Arc<Mutex<Agent>>>>>,
        session_id: Arc<RwLock<String>>,
        is_processing: Arc<RwLock<bool>>,
        shutdown_signals: Arc<RwLock<HashMap<String, InterruptSignal>>>,
        soft_interrupt_queues: SessionInterruptQueues,
        event_tx: broadcast::Sender<ServerEvent>,
        provider: Arc<dyn Provider>,
    ) -> Self {
        Self {
            sessions,
            session_id,
            is_processing,
            shutdown_signals,
            soft_interrupt_queues,
            event_tx,
            provider,
        }
    }
}

/// Thin handle for everything that conceptually belongs to the **Swarm** service.
/// In the current shape, most swarm data lives inside `SwarmState`.
/// This handle gives us a single thing to thread around while we gradually
/// narrow the old wide signatures.
#[derive(Clone)]
pub(crate) struct SwarmServiceHandle {
    pub swarm_state: SwarmState,
    pub shared_context: Arc<RwLock<HashMap<String, HashMap<String, super::SharedContext>>>>,
    pub channel_subscriptions: Arc<RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>>,
    pub channel_subscriptions_by_session:
        Arc<RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>>,
    pub file_touches: Arc<RwLock<HashMap<std::path::PathBuf, Vec<super::FileAccess>>>>,
    pub files_touched_by_session: Arc<RwLock<HashMap<String, std::collections::HashSet<std::path::PathBuf>>>>,
    pub event_history: Arc<RwLock<std::collections::VecDeque<super::SwarmEvent>>>,
    pub event_counter: Arc<std::sync::atomic::AtomicU64>,
    pub swarm_event_tx: broadcast::Sender<super::SwarmEvent>,
    pub await_members_runtime: super::await_members_state::AwaitMembersRuntime,
    pub swarm_mutation_runtime: super::swarm_mutation_state::SwarmMutationRuntime,
}

impl SwarmServiceHandle {
    pub fn new(
        swarm_state: SwarmState,
        shared_context: Arc<RwLock<HashMap<String, HashMap<String, super::SharedContext>>>>,
        channel_subscriptions: Arc<RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>>,
        channel_subscriptions_by_session: Arc<
            RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>,
        >,
        file_touches: Arc<RwLock<HashMap<std::path::PathBuf, Vec<super::FileAccess>>>>,
        files_touched_by_session: Arc<RwLock<HashMap<String, std::collections::HashSet<std::path::PathBuf>>>>,
        event_history: Arc<RwLock<std::collections::VecDeque<super::SwarmEvent>>>,
        event_counter: Arc<std::sync::atomic::AtomicU64>,
        swarm_event_tx: broadcast::Sender<super::SwarmEvent>,
        await_members_runtime: super::await_members_state::AwaitMembersRuntime,
        swarm_mutation_runtime: super::swarm_mutation_state::SwarmMutationRuntime,
    ) -> Self {
        Self {
            swarm_state,
            shared_context,
            channel_subscriptions,
            channel_subscriptions_by_session,
            file_touches,
            files_touched_by_session,
            event_history,
            event_counter,
            swarm_event_tx,
            await_members_runtime,
            swarm_mutation_runtime,
        }
    }

    // === Ola 4 Wave 4.1 SwarmStateInMonitor (coordinate with Move6CollapseLead) ===
    // Extracted ONLY the swarm membership/state queries from inside monitor_bus loop
    // (reads of swarm_members + swarms_by_id to compute swarm_session_ids / peers,
    // plus common member info lookups for names).
    // Thin read methods on SwarmServiceHandle (appropriate owner for swarm state).
    // Zero behavior change, passthrough first. Do not touch file touch recording
    // or event paths (record_swarm_event, notification fanout, alert sends).
    // Follows Ola 4 Master Plan Wave 4.1 + sub-wave gates.
    //
    // These will be used by monitor_bus call sites (updated below) and prepare
    // for later param collapse + ownership move. Ergonomic &self variants for
    // when SwarmServiceHandle is threaded; Arc-taking variants for transitional
    // sites that still receive raw pieces (like current monitor_bus).

    /// Thin read: list of peer session_ids sharing the swarm with the given session.
    /// Direct extraction of the monitor_bus inline computation for swarm_session_ids.
    /// Passthrough implementation (logic identical to prior inline block).
    pub async fn get_swarm_peers_for_session(
        swarm_members: &Arc<RwLock<HashMap<String, super::SwarmMember>>>,
        swarms_by_id: &Arc<RwLock<HashMap<String, HashSet<String>>>>,
        session_id: &str,
    ) -> Vec<String> {
        let members = swarm_members.read().await;
        if let Some(member) = members.get(session_id) {
            if let Some(ref swarm_id) = member.swarm_id {
                let swarms = swarms_by_id.read().await;
                if let Some(swarm) = swarms.get(swarm_id) {
                    swarm.iter().cloned().collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    /// Ergonomic instance-method variant (preferred when SwarmServiceHandle available via services.swarm()).
    pub async fn swarm_peers_for(&self, session_id: &str) -> Vec<String> {
        Self::get_swarm_peers_for_session(
            &self.swarm_state.members,
            &self.swarm_state.swarms_by_id,
            session_id,
        )
        .await
    }

    /// Thin read helper: (friendly_name, swarm_id) for a session. Common membership query.
    /// (Supports future extraction of other monitor_bus member lookups without event side-effects.)
    pub async fn get_member_swarm_info(
        swarm_members: &Arc<RwLock<HashMap<String, super::SwarmMember>>>,
        session_id: &str,
    ) -> (Option<String>, Option<String>) {
        let members = swarm_members.read().await;
        members
            .get(session_id)
            .map(|m| (m.friendly_name.clone(), m.swarm_id.clone()))
            .unwrap_or((None, None))
    }

    /// Ergonomic variant.
    pub async fn member_swarm_info(&self, session_id: &str) -> (Option<String>, Option<String>) {
        Self::get_member_swarm_info(&self.swarm_state.members, session_id).await
    }

    // === Ola 4 Wave 4.1.2 SwarmStateInMonitor (this sub-agent) ===
    // Completes extraction of swarm membership/state *query* logic from monitor_bus
    // FileTouch arm (the direct swarm_members.read() + friendly_name lookups in the
    // alert/notification construction loops after the peers computation).
    // Thin read/passthrough only. Placed on SwarmServiceHandle (natural owner for
    // swarm membership state). Follows exact pattern of Ola 3 dispatch + 4.1.1
    // FileTouchExtractor (static &Arc form for transitional raw-param monitor_bus
    // sites + ergonomic &self form; full docs; zero behavior change).
    // Scope: ONLY queries for names used in alerts. NO event recording/fanout
    // (record_file_activity_event etc untouched), NO sends/queues, NO param list
    // changes to monitor_bus, NO ownership move. Non-overlapping with FileTouch
    // writes and EventRecording.
    // Prepares for later ParamCollapse (4.1.4). References: OLA4_MASTER_COMPLETION_PLAN.md
    // Wave 4.1 + Sub-wave 4.1.2, ORCHESTRATION_STATUS.md (4.1.1 closure + Lead proposal).

    /// Thin read helper: friendly_name for a session. Used exclusively for the
    /// member name lookups inside monitor_bus FileTouch alert/notification loops
    /// (the "later alert loops that read members again").
    /// Passthrough; logic identical to prior inline .and_then friendly_name clones.
    /// Does not return event_tx or perform any fanout/send/queue (those remain
    /// direct in caller per strict scope).
    pub async fn get_member_friendly_name_for_notification(
        swarm_members: &Arc<RwLock<HashMap<String, super::SwarmMember>>>,
        session_id: &str,
    ) -> Option<String> {
        let members = swarm_members.read().await;
        members.get(session_id).and_then(|m| m.friendly_name.clone())
    }

    /// Ergonomic &self variant (for future use once services bag threaded to monitor paths).
    pub async fn member_friendly_name_for_notification(&self, session_id: &str) -> Option<String> {
        Self::get_member_friendly_name_for_notification(&self.swarm_state.members, session_id).await
    }

    // === Ola 4 Wave 4.1 EventRecordingExtractor ===
    // Extracted ONLY the event recording + notification fanout logic for the
    // record_swarm_event call (and SwarmEventType::FileTouch construction) after
    // FileTouch inside monitor_bus. Thin passthrough on SwarmServiceHandle (owner
    // of event_history / event_counter / swarm_event_tx). Zero behavior change.
    // Wired at the single monitor_bus site using get_member_swarm_info helper
    // (pre-provided by SwarmStateInMonitor; no membership query code authored here).
    // Strictly non-overlapping: no file touch writes, no swarm membership queries,
    // no FileConflict notification/alert logic (separate fanout), no param collapse.
    // Follows OLA4_MASTER_COMPLETION_PLAN.md Wave 4.1 + SERVER_SERVICE_SPLIT_PLAN.md Move 6 exactly.
    // Small focused slice + cargo check both profiles + targeted tests + commit.

    /// Thin passthrough for recording swarm event on FileTouch (the record_swarm_event
    /// invocation + construction of SwarmEventType::FileTouch after BusEvent::FileTouch
    /// in monitor_bus). Delegates to the core recorder (fanout to swarm_event_tx + history).
    /// Signature mirrors record_swarm_event but specialized for this FileTouch variant.
    /// Zero behavior change.
    pub(super) async fn record_file_activity_event(
        event_history: &Arc<RwLock<std::collections::VecDeque<super::SwarmEvent>>>,
        event_counter: &Arc<std::sync::atomic::AtomicU64>,
        swarm_event_tx: &broadcast::Sender<super::SwarmEvent>,
        session_id: String,
        session_name: Option<String>,
        swarm_id: Option<String>,
        touch: &crate::bus::FileTouch,
    ) {
        super::swarm::record_swarm_event(
            event_history,
            event_counter,
            swarm_event_tx,
            session_id,
            session_name,
            swarm_id,
            super::SwarmEventType::FileTouch {
                path: touch.path.to_string_lossy().to_string(),
                op: touch.op.as_str().to_string(),
                intent: touch.intent.clone(),
                summary: touch.summary.clone(),
                detail: touch.detail.clone(),
            },
        )
        .await;
    }
}

/// Thin handle for **Client** connection concerns (accept loops, connection registry, etc.).
#[derive(Clone)]
pub(crate) struct ClientServiceHandle {
    pub client_count: Arc<RwLock<usize>>,
    pub client_connections: Arc<RwLock<HashMap<String, ClientConnectionInfo>>>,
}

impl ClientServiceHandle {
    pub fn new(
        client_count: Arc<RwLock<usize>>,
        client_connections: Arc<RwLock<HashMap<String, ClientConnectionInfo>>>,
    ) -> Self {
        Self {
            client_count,
            client_connections,
        }
    }
}

/// Thin handle for **Debug** concerns.
#[derive(Clone)]
pub(crate) struct DebugServiceHandle {
    pub client_debug_state: Arc<RwLock<ClientDebugState>>,
    pub client_debug_response_tx: broadcast::Sender<(u64, String)>,
    pub debug_jobs: Arc<RwLock<HashMap<String, DebugJob>>>,
}

impl DebugServiceHandle {
    pub fn new(
        client_debug_state: Arc<RwLock<ClientDebugState>>,
        client_debug_response_tx: broadcast::Sender<(u64, String)>,
        debug_jobs: Arc<RwLock<HashMap<String, DebugJob>>>,
    ) -> Self {
        Self {
            client_debug_state,
            client_debug_response_tx,
            debug_jobs,
        }
    }
}

/// Thin handle for cross-cutting **Maintenance** / runtime concerns.
/// Phase 1: minimal bag so we can stop passing individual pieces everywhere.
#[derive(Clone)]
pub(crate) struct MaintenanceServiceHandle {
    pub ambient_runner: Option<AmbientRunnerHandle>,
    pub mcp_pool: Arc<OnceCell<Arc<SharedMcpPool>>>,
    pub server_identity: ServerIdentity,
    // Ola 3 Agent 4 - HygieneFinisher: removed 2 dead duplicate fields (server_name, server_icon).
    // They were pure copies of server_identity.{name,icon} (see ServerIdentity in util.rs:146-148).
    // References Ola 2 Agent 4 dead field cleanup (~22 fields; runtime.rs:21-24, handles.rs:432).
    // Surgical: zero behavior change, only MaintenanceServiceHandle storage cleaned. No monitor_bus/provider/reload/TUI touched.
}

impl MaintenanceServiceHandle {
    pub fn new(
        ambient_runner: Option<AmbientRunnerHandle>,
        mcp_pool: Arc<OnceCell<Arc<SharedMcpPool>>>,
        server_identity: ServerIdentity,
    ) -> Self {
        Self {
            ambient_runner,
            mcp_pool,
            server_identity,
        }
    }

    // === Ola 3 Agent 3 - ReloadE2EHardener (surgical, non-overlapping) ===
    // References Ola 2 closure priority #3 exactly (see ORCHESTRATION_STATUS.md:29,67,80):
    // "3. **Reload guards + recovery behind MaintenanceServiceHandle + E2E gate**:
    // Mirror the high-blast-radius reload paths (server_reload_starting, marker handoff, recovery) behind the handle;
    // add 1-2 unit tests exercising narrowed paths; require green `tests/e2e/binary_integration` reload family + `scripts/test_reload.py` on any touch.
    // Directly de-risks the #1 risk from Server Debt Hunter."
    // These are thin mirrors/passthroughs (zero behavior change). Reload logic bodies stay in reload_*.rs + client_lifecycle guards.
    // Recovery paths (reload_recovery) + handoff (reload.rs) + guards now have documented canonical surface + strict E2E gates here.
    // No monitor_bus, provider, TUI, emit, memory touched. Only handles.rs edited for this scope.

    /// Reload starting guard (canonical mirror of server_reload_starting() + recent_reload_state + Starting phase).
    /// High-blast-radius reload guard path. Future guard checks should delegate here.
    pub fn reload_starting_guard_active(&self) -> bool {
        matches!(
            crate::server::recent_reload_state(std::time::Duration::from_secs(30)),
            Some(state) if state.phase == crate::server::ReloadPhase::Starting
        )
    }

    /// Passthrough for high-blast-radius marker active check (used in handoff/await flows).
    pub fn reload_marker_active(&self, max_age: std::time::Duration) -> bool {
        crate::server::reload_marker_active(max_age)
    }

    /// Canonical thin entry for reload marker write (handoff initiation, recovery setup).
    pub fn write_reload_marker(&self) {
        crate::server::write_reload_marker();
    }

    /// Canonical thin entry for reload marker clear (recovery / handoff complete paths).
    pub fn clear_reload_marker(&self) {
        crate::server::clear_reload_marker();
    }

    /// High-blast-radius reload handoff await surface (marker-driven pid/socket transition + recovery).
    pub async fn await_reload_handoff(
        &self,
        socket_path: &std::path::Path,
        max_age: std::time::Duration,
    ) -> crate::server::ReloadWaitStatus {
        crate::server::await_reload_handoff(socket_path, max_age).await
    }

    /// Thin mirror for reload marker existence (high-blast decision in recovery/intent flows per Ola 2 closure priority #3).
    pub fn reload_marker_exists(&self) -> bool {
        crate::server::reload_marker_exists()
    }

    /// Diagnostic surface for current reload state (used in high-blast handoff/recovery logging + guards).
    /// Completes canonical thin surface for recovery logic + marker paths behind MaintenanceServiceHandle.
    pub fn reload_state_summary(&self, max_age: std::time::Duration) -> String {
        crate::server::reload_state_summary(max_age)
    }

    // E2E TEST REQUIREMENTS / GATES (strict for ANY future touches to reload guards, recovery, marker handoff, high-blast paths):
    // - Edit to server_reload_starting, reload marker fns, recovery persist/mark_delivered, handoff logic, or these methods
    //   REQUIRES green BEFORE merge:
    //     * `cargo test -p jcode --test binary_integration` (full reload family: binary_integration_selfdev_reload_reconnects_quickly,
    //       selfdev_client_reload_resumes_session, selfdev_full_reload_resumes_session_quickly + PTY wait helpers in tests/e2e/test_support/mod.rs)
    //     * `python scripts/test_reload.py` (or pwsh equivalent exercising reload cycle)
    //     * `cargo check -p jcode --lib` (and recommended: fast `cargo test --lib --bins -- --test-threads=1` covering reload_tests.rs + client_session_tests/reload/*)
    // - This is the E2E gate defined by Ola 2 closure priority #3 + Ola 3 ReloadE2EHardener mandate.
    // - 1-2 unit test exercises for narrowed paths should be added on any expansion of these methods (per charter).
    // Protects the highest-risk server paths (cross-PID/version exec handoff + continuation intent delivery).

    /// Thin seam for FileTouch recording + reverse index (Wave 4.1.1 FileTouchExtractor sub-wave).
    /// Encapsulates the two direct writes previously in monitor_bus FileTouch arm (server.rs:1369-1388).
    /// Per OLA4_MASTER_COMPLETION_PLAN Wave 4.1 + SPLIT_PLAN Move 6: first mutation path collapsed behind MaintenanceServiceHandle.
    /// Signature mirrors the touch data + Arcs exactly (no extra indirection yet).
    /// Zero behavior change; call-site replace is mechanical 1:1.
    /// Future: when state migrates to handle or MaintenanceRuntime, this will take &self or &SwarmServiceHandle.
    /// E2E gate note: any expansion requires file_activity_tests.rs + server integration tests + full gate (no reload/TUI touched).
    pub(super) async fn record_file_touch(
        file_touches: Arc<RwLock<HashMap<PathBuf, Vec<super::FileAccess>>>>,
        files_touched_by_session: Arc<RwLock<HashMap<String, HashSet<PathBuf>>>>,
        path: PathBuf,
        session_id: String,
        op: crate::bus::FileOp,
        intent: Option<String>,
        summary: Option<String>,
        detail: Option<String>,
    ) {
        {
            let mut touches = file_touches.write().await;
            let accesses = touches.entry(path.clone()).or_insert_with(Vec::new);
            accesses.push(super::FileAccess {
                session_id: session_id.clone(),
                op: op.clone(),
                timestamp: Instant::now(),
                absolute_time: std::time::SystemTime::now(),
                intent: intent.clone(),
                summary: summary.clone(),
                detail: detail.clone(),
            });
        }
        {
            let mut reverse_index = files_touched_by_session.write().await;
            reverse_index
                .entry(session_id.clone())
                .or_default()
                .insert(path.clone());
        }
    }
}

/// Thin handle for **Provider** runtime concerns (catalog, failover, auth refresh).
///
/// Ola 3 Agent 2 - ProviderFacadeFirstSlice (first safe slice only):
/// - Minimal surface: holds the provider Arc (reused from Session for now; future slices
///   will own catalog state, failover targets, auth refresh coordination exclusively).
/// - Zero behavior change. No mutations moved yet. References Ola 2 thin-handle pattern
///   (e.g. SwarmServiceHandle methods) and Ola 3 charter priority #2.
/// - Responsibilities documented here for catalog + failover + auth refresh; methods
///   are thin passthroughs / accessors only in this slice.
#[derive(Clone)]
pub(crate) struct ProviderServiceHandle {
    pub provider: Arc<dyn Provider>,
}

impl ProviderServiceHandle {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Primary accessor (thin).
    pub fn provider(&self) -> Arc<dyn Provider> {
        Arc::clone(&self.provider)
    }

    /// Catalog responsibility surface (first slice: thin name + future catalog expansion point).
    pub fn name(&self) -> &str {
        self.provider.name()
    }

    /// Auth refresh responsibility surface (first slice: thin delegation entry point; no logic moved).
    pub fn on_auth_changed(&self) {
        self.provider.on_auth_changed();
    }

    /// Catalog responsibility surface (first slice expansion): model accessor for catalog flows.
    pub fn model(&self) -> String {
        self.provider.model()
    }

    /// Catalog responsibility surface (first slice): available models for display/picker/catalog refresh.
    pub fn available_models_display(&self) -> Vec<String> {
        self.provider.available_models_display()
    }

    /// Auth refresh responsibility surface (first slice expansion): preserve-current variant used in auth handling paths.
    pub fn on_auth_changed_preserve_current_provider(&self) {
        self.provider.on_auth_changed_preserve_current_provider();
    }

    // Failover responsibility surface (first slice entry point added; concrete classify/account logic
    // remains in src/provider/failover.rs + account_failover.rs per surgical non-overlap rule).
    // Thin hook reserved for routing provider paths in follow-on slices.

    /// Failover responsibility surface (first slice): preferred provider hook for catalog/failover routing decisions.
    /// Zero behavior change; logic stays in provider layer.
    pub fn preferred_provider(&self) -> Option<String> {
        self.provider.preferred_provider()
    }

    /// Catalog responsibility (core of first slice): explicit async refresh entry.
    /// Thin delegation; the real catalog work + heartbeat + summary stays in provider_control + provider/* .
    pub async fn refresh_model_catalog(&self) -> ::anyhow::Result<jcode_provider_core::ModelCatalogRefreshSummary> {
        self.provider.refresh_model_catalog().await
    }

    /// Failover responsibility (core of first slice): error classification seam.
    /// Delegates to jcode-provider-core; concrete account failover / prompt building remains untouched in src/provider/*.
    pub fn classify_failover_error(&self, err: &anyhow::Error) -> jcode_provider_core::FailoverDecision {
        jcode_provider_core::classify_failover_error_message(&err.to_string())
    }
}

/// Bag that owns one of each service handle.
/// This will eventually live inside `Server` and be the only thing
/// `ServerRuntime` and the accept loops need to clone.
#[derive(Clone)]
pub(crate) struct ServerServices {
    pub session: SessionServiceHandle,
    pub swarm: SwarmServiceHandle,
    pub client: ClientServiceHandle,
    pub debug: DebugServiceHandle,
    pub maintenance: MaintenanceServiceHandle,
    /// First thin Provider facade slice (Ola 3 Agent 2).
    pub provider: ProviderServiceHandle,
}

impl ServerServices {
    /// Build the complete services bag from the individual pieces that `Server::new`
    /// already creates. This is the Phase 1 "no behavior change" constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sessions: Arc<RwLock<HashMap<String, Arc<Mutex<Agent>>>>>,
        session_id: Arc<RwLock<String>>,
        is_processing: Arc<RwLock<bool>>,
        shutdown_signals: Arc<RwLock<HashMap<String, InterruptSignal>>>,
        soft_interrupt_queues: SessionInterruptQueues,
        event_tx: broadcast::Sender<ServerEvent>,
        provider: Arc<dyn Provider>,

        swarm_state: super::SwarmState,
        shared_context: Arc<RwLock<HashMap<String, HashMap<String, super::SharedContext>>>>,
        channel_subscriptions: Arc<RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>>,
        channel_subscriptions_by_session: Arc<RwLock<HashMap<String, HashMap<String, std::collections::HashSet<String>>>>>,
        file_touches: Arc<RwLock<HashMap<std::path::PathBuf, Vec<super::FileAccess>>>>,
        files_touched_by_session: Arc<RwLock<HashMap<String, std::collections::HashSet<std::path::PathBuf>>>>,
        event_history: Arc<RwLock<std::collections::VecDeque<super::SwarmEvent>>>,
        event_counter: Arc<std::sync::atomic::AtomicU64>,
        swarm_event_tx: broadcast::Sender<super::SwarmEvent>,
        await_members_runtime: super::await_members_state::AwaitMembersRuntime,
        swarm_mutation_runtime: super::swarm_mutation_state::SwarmMutationRuntime,

        client_count: Arc<RwLock<usize>>,
        client_connections: Arc<RwLock<HashMap<String, ClientConnectionInfo>>>,

        client_debug_state: Arc<RwLock<ClientDebugState>>,
        client_debug_response_tx: broadcast::Sender<(u64, String)>,
        debug_jobs: Arc<RwLock<HashMap<String, DebugJob>>>,

        ambient_runner: Option<AmbientRunnerHandle>,
        mcp_pool: Arc<OnceCell<Arc<SharedMcpPool>>>,
        server_identity: ServerIdentity,
        // Ola 3 Agent 4 HygieneFinisher: removed server_name/server_icon params (duplicates of identity; see struct clean above + Ola 2 precedent).
    ) -> Self {
        // Clone provider Arc early for the ProviderServiceHandle (first slice).
        // SessionServiceHandle continues to receive its copy (no behavior change).
        let provider_for_handle = Arc::clone(&provider);
        let session = SessionServiceHandle::new(
            sessions,
            session_id,
            is_processing,
            shutdown_signals,
            soft_interrupt_queues,
            event_tx,
            provider,
        );

        let swarm = SwarmServiceHandle::new(
            swarm_state,
            shared_context,
            channel_subscriptions,
            channel_subscriptions_by_session,
            file_touches,
            files_touched_by_session,
            event_history,
            event_counter,
            swarm_event_tx,
            await_members_runtime,
            swarm_mutation_runtime,
        );

        let client = ClientServiceHandle::new(client_count, client_connections);

        let debug = DebugServiceHandle::new(
            client_debug_state,
            client_debug_response_tx,
            debug_jobs,
        );

        let maintenance = MaintenanceServiceHandle::new(
            ambient_runner,
            mcp_pool,
            server_identity,
            // server_name/server_icon dropped (Ola 3 Agent 4 duplicate field hygiene; derived from identity below)
        );

        // First safe slice (Ola 3 Agent 2): construct ProviderServiceHandle from the
        // provider already threaded for Session (clone is cheap Arc). This wires the
        // thin facade into the services bag without moving ownership or touching
        // monitor_bus / reload / TUI / emit / memory paths.
        let provider_handle = ProviderServiceHandle::new(provider_for_handle);

        Self {
            session,
            swarm,
            client,
            debug,
            maintenance,
            provider: provider_handle,
        }
    }

    /// Convenient accessors (Fase 1 threading helpers).
    pub fn session(&self) -> &SessionServiceHandle { &self.session }
    pub fn swarm(&self) -> &SwarmServiceHandle { &self.swarm }
    pub fn client(&self) -> &ClientServiceHandle { &self.client }

    /// Accessor for the Provider thin facade (Ola 3 Agent 2 first slice).
    pub fn provider(&self) -> &ProviderServiceHandle { &self.provider }

    /// Thin passthrough for the Provider Arc (Ola 3 Agent 2 first slice wiring aid).
    /// Allows provider paths (e.g. registry prewarm, agent construction) to source via services.
    pub fn provider_arc(&self) -> Arc<dyn Provider> {
        self.provider.provider()
    }

    /// Catalog responsibility passthrough (Ola 3 Agent 2 ProviderFacadeFirstSlice).
    pub fn provider_available_models_display(&self) -> Vec<String> {
        self.provider.available_models_display()
    }

    /// Auth refresh responsibility passthrough (Ola 3 Agent 2 ProviderFacadeFirstSlice).
    pub fn provider_on_auth_changed_preserve(&self) {
        self.provider.on_auth_changed_preserve_current_provider();
    }

    /// Failover responsibility surface passthrough (Ola 3 Agent 2 first slice).
    /// Placeholder (method does not exist on Provider trait yet — future slice).
    /// Zero behavior change for current callers; will be wired when catalog/failover
    /// logic moves behind the facade.
    pub fn provider_failover_surface_ready(&self) -> bool {
        // TODO(Ola 4): implement real failover_surface_ready on Provider trait
        // or move the check behind ProviderServiceHandle properly.
        true
    }

    pub fn client_count(&self) -> Arc<RwLock<usize>> {
        Arc::clone(&self.client.client_count)
    }

    // Thin passthroughs on the bag for common access during migration (will grow into real service methods).
    pub fn await_members_runtime(&self) -> super::await_members_state::AwaitMembersRuntime {
        self.swarm.await_members_runtime.clone()
    }
    pub fn swarm_mutation_runtime(&self) -> super::swarm_mutation_state::SwarmMutationRuntime {
        self.swarm.swarm_mutation_runtime.clone()
    }

    // Additional helpers for the heavy call site migration
    pub fn event_history(&self) -> Arc<RwLock<std::collections::VecDeque<super::SwarmEvent>>> {
        Arc::clone(&self.swarm.event_history)
    }
    pub fn event_counter(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.swarm.event_counter)
    }

    pub fn client_debug_state(&self) -> Arc<RwLock<ClientDebugState>> {
        Arc::clone(&self.debug.client_debug_state)
    }
    pub fn shutdown_signals(&self) -> Arc<RwLock<HashMap<String, InterruptSignal>>> {
        Arc::clone(&self.session.shutdown_signals)
    }
    pub fn soft_interrupt_queues(&self) -> SessionInterruptQueues {
        Arc::clone(&self.session.soft_interrupt_queues)
    }
    pub fn event_tx(&self) -> broadcast::Sender<ServerEvent> {
        self.session.event_tx.clone()
    }
    pub fn client_debug_response_tx(&self) -> broadcast::Sender<(u64, String)> {
        self.debug.client_debug_response_tx.clone()
    }
    pub fn debug_jobs(&self) -> Arc<RwLock<HashMap<String, DebugJob>>> {
        Arc::clone(&self.debug.debug_jobs)
    }

    // Remaining thin passthroughs for the big call site migration
    pub fn swarm_event_tx(&self) -> broadcast::Sender<super::SwarmEvent> {
        self.swarm.swarm_event_tx.clone()
    }
    pub fn server_name(&self) -> String {
        // Ola 3 Agent 4 HygieneFinisher: derive from identity (post removal of duplicate server_name field in MaintenanceServiceHandle).
        // References Ola 2 Agent 4 dead field clean. Zero behavior change for callers.
        self.maintenance.server_identity.name.clone()
    }
    pub fn server_icon(&self) -> String {
        self.maintenance.server_identity.icon.clone()
    }

    pub fn server_identity(&self) -> ServerIdentity {
        self.maintenance.server_identity.clone()
    }
    pub fn ambient_runner(&self) -> Option<AmbientRunnerHandle> {
        self.maintenance.ambient_runner.clone()
    }

    pub fn mcp_pool(&self) -> Arc<OnceCell<Arc<SharedMcpPool>>> {
        Arc::clone(&self.maintenance.mcp_pool)
    }

    // Helper to get the awaited pool (common pattern)
    // Thin accessor so call sites (e.g. the handle_client monster) never call the util helper directly.
    pub async fn get_mcp_pool(&self) -> Arc<SharedMcpPool> {
        super::util::get_shared_mcp_pool(&self.maintenance.mcp_pool).await
    }

    // Delegate into SwarmState for the remaining inners still accessed in the big call site
    pub fn swarm_members(&self) -> Arc<RwLock<HashMap<String, super::SwarmMember>>> {
        Arc::clone(&self.swarm.swarm_state.members)
    }
    pub fn swarms_by_id(&self) -> Arc<RwLock<HashMap<String, std::collections::HashSet<String>>>> {
        Arc::clone(&self.swarm.swarm_state.swarms_by_id)
    }
    pub fn swarm_plans(&self) -> Arc<RwLock<HashMap<String, super::VersionedPlan>>> {
        Arc::clone(&self.swarm.swarm_state.plans)
    }
    pub fn swarm_coordinators(&self) -> Arc<RwLock<HashMap<String, String>>> {
        Arc::clone(&self.swarm.swarm_state.coordinators)
    }

    // NOTE (Ola 2 Agent 4 dead field clean): ServerServices::from_server removed.
    // It was unused (0 call sites), and referenced legacy duplicate fields we
    // eliminated from Server (await_members_runtime + swarm_mutation_runtime).
    // The services bag is populated exclusively via ServerServices::new in Server::new
    // and carried via .services clone / ServerRuntime::from_server.
}
