use super::STREAM_KEEPALIVE_PONG_ID;
use crate::protocol::ServerEvent;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, MissedTickBehavior};

/// Emit Best Effort Hygiene (Agent D) — highest-volume swallow site identification
/// (from prior Error Auditor 2289 total + this targeted pass on agent/streaming + turn_* + provider paths).
///
/// The 3-5 highest-volume sites for ServerEvent fan-out swallows (and related provider streaming):
/// 1. `src/agent/turn_streaming_mpsc.rs` — highest volume (~39 let_ pre-pass): dense clusters of
///    ToolStart/ToolInput/ToolExec/ToolDone, TextDelta/TextReplace, Thinking*, Compaction recovery,
///    TokenUsage, graceful-shutdown + interrupt batch sends. Direct provider response path for mpsc TUI.
/// 2. `src/agent/turn_streaming_broadcast.rs` — symmetric high volume (~36 let_ pre-pass): identical
///    event fan-outs over broadcast for multi-client server sessions (ConnectionPhase/StatusDetail etc).
/// 3. `src/agent/streaming.rs` — the 2 canonical centralized `let _ =` inside the emit_* helpers
///    (the single source-of-truth swallows after quickwin rollout; all prior inline ones migrate here).
/// 4. Provider paths (e.g. `src/provider/gemini.rs:20+`, `src/provider/openrouter.rs`, `src/provider/copilot.rs:18`,
///    `src/provider/openai_stream_runtime.rs`, `src/provider/openrouter_sse_stream.rs`, `src/provider/anthropic.rs` etc):
///    High internal swallows for SSE framing, native_result channels, retry/phase sends, account failover.
///    (Not all use ServerEvent; many use separate mpsc/broadcast or .ok(); addressed via per-provider best-effort.)
/// 5. Interrupt + error ToolDone / recovery paths inside turn_streaming_* (high frequency under
///    soft-interrupt, reload, context-limit, provider stream error load — batches + per-tool swallows).
///
/// All hot paths remain panic-free by design (pure fire-and-forget). Use only for non-critical UI/progress events.
/// See Error Auditor + ORCHESTRATION_STATUS for full 2289 mapping.

fn stream_keepalive_interval() -> Duration {
    if cfg!(test) {
        Duration::from_millis(50)
    } else {
        Duration::from_secs(30)
    }
}

pub(super) fn stream_keepalive_ticker() -> time::Interval {
    let interval = stream_keepalive_interval();
    let mut ticker = time::interval_at(time::Instant::now() + interval, interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker
}

pub(super) fn send_stream_keepalive_broadcast(event_tx: &broadcast::Sender<ServerEvent>) {
    emit_best_effort_broadcast(
        event_tx,
        ServerEvent::Pong {
            id: STREAM_KEEPALIVE_PONG_ID,
        },
    );
}

pub(super) fn send_stream_keepalive_mpsc(event_tx: &mpsc::UnboundedSender<ServerEvent>) {
    emit_best_effort_mpsc(
        event_tx,
        ServerEvent::Pong {
            id: STREAM_KEEPALIVE_PONG_ID,
        },
    );
}

/// Best-effort emission of a `ServerEvent` via broadcast channel.
///
/// Use this for all non-fatal fan-out / progress / UI notification events
/// (e.g. `Compaction`, `MemoryInjected`, `TextDelta`, `Tool*`, `ConnectionPhase`,
/// `StatusDetail`, `TokenUsage`, `SidePanelState`, `GeneratedImage`, etc.).
///
/// This centralizes the previous ~75+ inline `let _ = event_tx.send(...)` swallows
/// that appear in `turn_streaming_broadcast.rs`, `turn_streaming_mpsc.rs`, and
/// related streaming paths.
///
/// # Semantics (unchanged from prior inline usage)
/// - Fire-and-forget: the send result is always ignored.
/// - A disconnected subscriber, full lag buffer, or zero receivers simply drops the event.
/// - Never propagates an error to the caller.
/// - Must never be used for critical control messages that require back-pressure or
///   guaranteed delivery (use the raw `send` + `?` or `is_err()` handling for those).
///
/// This is the recommended helper for the "provider response broadcast" swallows
/// identified by the Error Handling Auditor. It does not alter observable behavior.
///
/// See also: `emit_best_effort_mpsc` for the unbounded mpsc streaming variant.
pub(crate) fn emit_best_effort_broadcast(
    event_tx: &broadcast::Sender<ServerEvent>,
    event: ServerEvent,
) {
    // Best-effort / client-gone is expected and intentional for these events.
    // Subscriber disconnect or temporary lag must not stall a provider turn.
    let _ = event_tx.send(event);
}

/// Best-effort emission of a `ServerEvent` via unbounded mpsc channel.
///
/// Identical contract and purpose to `emit_best_effort_broadcast`.
/// Used by the mpsc-path streaming entrypoint (direct TUI sessions, certain tests,
/// and internal handoff paths).
///
/// Prefer this helper over raw `let _ = ...` for consistency and future
/// instrumentation (e.g. optional trace logging of dropped events can be added here
/// without touching dozens of call sites).
pub(crate) fn emit_best_effort_mpsc(
    event_tx: &mpsc::UnboundedSender<ServerEvent>,
    event: ServerEvent,
) {
    // Best-effort / client-gone is expected and intentional for these events.
    // Subscriber disconnect or temporary lag must not stall a provider turn.
    let _ = event_tx.send(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ServerEvent;

    /// Targeted test for emit_best_effort helpers under error (closed channels) + load (many emissions).
    /// Verifies panic-free behavior on hot path even when all subscribers are gone (the primary swallow case).
    #[test]
    fn emit_best_effort_helpers_silent_under_closed_channel_and_load() {
        // Broadcast variant: drop receiver -> send would error; helper must swallow silently.
        {
            let (tx, rx) = broadcast::channel::<ServerEvent>(8);
            drop(rx);
            // Under load: many sends after close (simulates subscriber disconnect during bursty provider stream)
            for i in 0..256u32 {
                emit_best_effort_broadcast(
                    &tx,
                    ServerEvent::Pong {
                        id: STREAM_KEEPALIVE_PONG_ID.wrapping_add(i),
                    },
                );
            }
            // No panic, all dropped. (If helper ever used .expect this would fail the test.)
        }

        // MPSC unbounded variant: same contract.
        {
            let (tx, rx) = mpsc::unbounded_channel::<ServerEvent>();
            drop(rx);
            for i in 0..256u32 {
                emit_best_effort_mpsc(
                    &tx,
                    ServerEvent::Pong {
                        id: STREAM_KEEPALIVE_PONG_ID.wrapping_add(i),
                    },
                );
            }
        }
    }
}
