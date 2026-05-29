use super::STREAM_KEEPALIVE_PONG_ID;
use crate::protocol::ServerEvent;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, MissedTickBehavior};

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
