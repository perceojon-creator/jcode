use super::client_lifecycle::handle_client;
use super::debug::{ClientDebugState, handle_debug_client};
use super::debug_jobs::DebugJob;

use crate::gateway::GatewayClient;
use crate::transport::{Listener, Stream};
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Clone)]
pub(super) struct ServerRuntime {
    /// Sole source of truth for all server services (post Ola 1 Agent A + Agent 4 dead field clean).
    /// All legacy duplicate fields removed; runtime methods use self.services.* exclusively.
    /// See handles.rs and SERVER_SERVICE_SPLIT_PLAN.
    services: super::handles::ServerServices,
}

impl ServerRuntime {
    pub(super) fn from_server(server: &super::Server) -> Self {
        Self {
            // Fase 1 complete + dead field cleanup (Ola 2 Agent 4): services bag is now the only field.
            // Legacy duplicates (sessions, event_tx, ... await_members_runtime etc) fully eliminated
            // as they were unused in all ServerRuntime methods (reads went exclusively via services post Agent A).
            services: server.services.clone(),
        }
    }

    pub(super) fn spawn_main_accept_loop(&self, listener: Listener) -> tokio::task::JoinHandle<()> {
        let runtime = self.clone();
        tokio::spawn(async move {
            #[cfg(windows)]
            let mut listener = listener;

            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        runtime.increment_client_count().await;
                        runtime.spawn_client_task(stream, "Client error", true);
                    }
                    Err(e) => {
                        crate::logging::error(&format!("Main accept error: {}", e));
                    }
                }
            }
        })
    }

    pub(super) fn spawn_debug_accept_loop(
        &self,
        listener: Listener,
        server_start_time: Instant,
    ) -> tokio::task::JoinHandle<()> {
        let runtime = self.clone();
        tokio::spawn(async move {
            #[cfg(windows)]
            let mut listener = listener;

            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        // Debug clients do not participate in idle-timeout accounting.
                        runtime.spawn_debug_client_task(stream, server_start_time);
                    }
                    Err(e) => {
                        crate::logging::error(&format!("Debug accept error: {}", e));
                    }
                }
            }
        })
    }

    pub(super) fn spawn_gateway_accept_loop(
        &self,
        mut client_rx: mpsc::UnboundedReceiver<GatewayClient>,
    ) -> tokio::task::JoinHandle<()> {
        let runtime = self.clone();
        tokio::spawn(async move {
            while let Some(gw_client) = client_rx.recv().await {
                runtime.increment_client_count().await;
                crate::logging::info(&format!(
                    "Gateway client connected: {} ({})",
                    gw_client.device_name, gw_client.device_id
                ));
                // Preserve prior behavior: gateway sessions do not nudge the
                // ambient runner on disconnect.
                runtime.spawn_gateway_client_task(gw_client);
            }
        })
    }

    fn spawn_client_task(&self, stream: Stream, error_prefix: &'static str, nudge_ambient: bool) {
        let runtime = self.clone();
        tokio::spawn(async move {
            runtime
                .run_client_stream(stream, error_prefix, nudge_ambient)
                .await;
        });
    }

    fn spawn_gateway_client_task(&self, gw_client: GatewayClient) {
        let runtime = self.clone();
        tokio::spawn(async move {
            runtime
                .run_client_stream(gw_client.stream, "Gateway client error", false)
                .await;
        });
    }

    fn spawn_debug_client_task(&self, stream: Stream, server_start_time: Instant) {
        let runtime = self.clone();
        tokio::spawn(async move {
            runtime.run_debug_stream(stream, server_start_time).await;
        });
    }

    async fn increment_client_count(&self) {
        *self.services.client_count().write().await += 1;
        crate::runtime_memory_log::emit_event(
            crate::runtime_memory_log::RuntimeMemoryLogEvent::new(
                "client_connected",
                "client_count_incremented",
            ),
        );
    }

    async fn decrement_client_count(&self) {
        *self.services.client_count().write().await -= 1;
        crate::runtime_memory_log::emit_event(
            crate::runtime_memory_log::RuntimeMemoryLogEvent::new(
                "client_disconnected",
                "client_count_decremented",
            ),
        );
    }

    async fn run_client_stream(
        self,
        stream: Stream,
        error_prefix: &'static str,
        nudge_ambient: bool,
    ) {
        // Ola 2 Move4 (Agent 1): narrowed to services bag + stream (SPLIT_PLAN Move 4).
        // All 29 exploded args eliminated; mcp fetch moved inside handler.
        let result = handle_client(stream, self.services.clone()).await;

        self.decrement_client_count().await;

        if nudge_ambient && let Some(ref runner) = self.services.ambient_runner() {
            runner.nudge();
        }

        if let Err(e) = result {
            crate::logging::error(&format!("{}: {}", error_prefix, e));
        }
    }

    async fn run_debug_stream(self, stream: Stream, server_start_time: Instant) {
        // Ola 2 Move4 (Agent 1): narrowed to services bag + stream + server_start_time context.
        if let Err(e) = handle_debug_client(stream, self.services, server_start_time).await {
            crate::logging::error(&format!("Debug client error: {}", e));
        }
    }
}
