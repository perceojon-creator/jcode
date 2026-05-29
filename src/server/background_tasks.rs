use super::{
    FileAccess, SessionAgents, SessionInterruptQueues, SharedContext, SwarmEvent, SwarmMember,
    VersionedPlan, fanout_session_event, file_activity_scope_label,
    queue_soft_interrupt_for_session, session_event_fanout_sender,
};
use super::handles::MaintenanceServiceHandle;
use crate::message::{
    format_background_task_notification_markdown, format_background_task_progress_markdown,
};
use crate::protocol::{NotificationType, ServerEvent};
use jcode_agent_runtime::SoftInterruptSource;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

async fn run_background_task_message_in_live_session_if_idle(
    session_id: &str,
    message: &str,
    sessions: &SessionAgents,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
) -> bool {
    let agent = {
        let guard = sessions.read().await;
        guard.get(session_id).cloned()
    };
    let Some(agent) = agent else {
        return false;
    };

    let has_live_attachments = {
        let members = swarm_members.read().await;
        members
            .get(session_id)
            .map(|member| !member.event_txs.is_empty() || !member.event_tx.is_closed())
            .unwrap_or(false)
    };
    if !has_live_attachments {
        return false;
    }

    let is_idle = match agent.try_lock() {
        Ok(guard) => {
            drop(guard);
            true
        }
        Err(_) => false,
    };

    if !is_idle {
        return false;
    }

    let session_id = session_id.to_string();
    let message = message.to_string();
    let event_tx = session_event_fanout_sender(session_id.clone(), Arc::clone(swarm_members));
    tokio::spawn(async move {
        if let Err(err) = super::client_lifecycle::process_message_streaming_mpsc(
            agent,
            &message,
            vec![],
            Some(
                "A background task for this session just finished. Review the completion message and continue if useful."
                    .to_string(),
            ),
            event_tx,
        )
        .await
        {
            crate::logging::error(&format!(
                "Failed to run background task completion immediately for live session {}: {}",
                session_id, err
            ));
        }
    });

    true
}

// Ola 3 Agent 1 (Move6-MonitorBusExtractor): the background dispatch paths
// (previously free fns called from monitor_bus) are now behind MaintenanceServiceHandle
// as thin associated methods per SERVER_SERVICE_SPLIT_PLAN.md Move 6.
// Bodies moved verbatim into the impl (logic now "on" the handle type).
// Original pub(super) names remain as 1-line thin delegates (no behavior change;
// allows tests + other intra-module sites to continue compiling untouched).
impl MaintenanceServiceHandle {
    pub(super) async fn dispatch_background_task_completion(
        task: &crate::bus::BackgroundTaskCompleted,
        sessions: &SessionAgents,
        soft_interrupt_queues: &SessionInterruptQueues,
        swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
    ) {
        let notification = format_background_task_notification_markdown(task);

        if task.notify
            && fanout_session_event(
                swarm_members,
                &task.session_id,
                ServerEvent::Notification {
                    from_session: "background_task".to_string(),
                    from_name: Some("background task".to_string()),
                    notification_type: NotificationType::Message {
                        scope: Some("background_task".to_string()),
                        channel: None,
                    },
                    message: notification.clone(),
                },
            )
            .await
                == 0
        {
            crate::logging::warn(&format!(
                "Failed to notify attached clients for background task completion on session {}",
                task.session_id
            ));
        }

        if task.wake
            && !run_background_task_message_in_live_session_if_idle(
                &task.session_id,
                &notification,
                sessions,
                swarm_members,
            )
            .await
            && !queue_soft_interrupt_for_session(
                &task.session_id,
                notification.clone(),
                false,
                SoftInterruptSource::BackgroundTask,
                soft_interrupt_queues,
                sessions,
            )
            .await
        {
            crate::logging::warn(&format!(
                "Failed to deliver background task completion to session {}",
                task.session_id
            ));
        }
    }

    pub(super) async fn dispatch_background_task_progress(
        task: &crate::bus::BackgroundTaskProgressEvent,
        swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
    ) {
        let notification = format_background_task_progress_markdown(task);
        if fanout_session_event(
            swarm_members,
            &task.session_id,
            ServerEvent::Notification {
                from_session: "background_task".to_string(),
                from_name: Some("background task".to_string()),
                notification_type: NotificationType::Message {
                    scope: Some("background_task".to_string()),
                    channel: None,
                },
                message: notification,
            },
        )
        .await
            == 0
        {
            crate::logging::warn(&format!(
                "Failed to notify attached clients for background task progress on session {}",
                task.session_id
            ));
        }
    }

    pub(super) async fn dispatch_ui_activity(
        activity: &crate::bus::UiActivity,
        swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
    ) {
        let Some(session_id) = activity.session_id.as_deref() else {
            return;
        };

        if fanout_session_event(
            swarm_members,
            session_id,
            ServerEvent::Notification {
                from_session: "jcode".to_string(),
                from_name: Some("Jcode".to_string()),
                notification_type: NotificationType::Message {
                    scope: Some(activity.kind.scope().to_string()),
                    channel: None,
                },
                message: activity.message.clone(),
            },
        )
        .await
            == 0
        {
            crate::logging::warn(&format!(
                "Failed to notify attached clients for UI activity on session {}",
                session_id
            ));
        }
    }

    // === Ola 4 Wave 4.1.4 ParamCollapse (second high-impact method) ===
    // Extracts the *entire* remaining direct raw-Arc logic block from monitor_bus
    // FileTouch arm: the `if !previous_touches.is_empty()` alert construction,
    // members.read() for event_tx, two notification send loops, current/prev name
    // lookups (via existing SwarmServiceHandle thin helpers), scope labels,
    // + all queue_soft_interrupt_for_session calls for file conflicts.
    // This is the highest-leverage "complex query + side-effect" site left using
    // raw swarm_members + sessions + soft_interrupt_queues inside monitor_bus.
    // Placed in the MaintenanceServiceHandle impl extension (sibling to the other
    // dispatch_* for background/UI that already lived here post-Ola 3).
    // Follows exact Ola 3 dispatch extraction pattern + 4.1 thin method docs.
    // Zero behavior change: logs, sends, queues, alert msgs, Notification payloads
    // are verbatim from the inline block (now behind the seam).
    // Non-overlapping: does not touch FileTouch write (Maintenance record_file_touch),
    // does not touch event recording (Swarm record_file_activity_event), does not
    // touch the 3 prior query extractions. Only the alert fanout/queue part.
    // After this + call site update in monitor_bus: the only raw Arc usages left
    // inside monitor_bus body are the ones passed *to* thin handle methods
    // (record_*, get_*, previous_*, dispatch_* family) + the other BusEvent arms.
    // Directly advances "collapse the raw parameter list" + "move real ownership/logic
    // behind the service handles". Prepares for full monitor_bus taking ServerServices
    // or Maintenance+Swarm handles (Ola 4 #2 completion).
    // Refs: user mandate for ParamCollapse agent, OLA4_MASTER Wave 4.1.4,
    // ORCHESTRATION_STATUS 4.1.4, SPLIT_PLAN Move 6, background_tasks dispatch precedent.

    /// Thin dispatch for FileConflict alerts + soft interrupts (ParamCollapse 4.1.4).
    /// Takes previous peer touches (already computed via SwarmServiceHandle::previous_peer_touches)
    /// + the raw pieces needed for tx fanout and queuing. Internal read of swarm_members
    /// for event_tx is now here (no longer in monitor_bus).
    /// Name lookups delegate to SwarmServiceHandle statics (no dupe logic).
    /// Signature mirrors the data that was inlined; pub(super) for intra-server use.
    pub(super) async fn dispatch_file_conflict_alerts(
        swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
        sessions: &SessionAgents,
        soft_interrupt_queues: &SessionInterruptQueues,
        previous_touches: &[FileAccess],
        path: &PathBuf,
        touch: &crate::bus::FileTouch,
        session_id: &str,
    ) {
        crate::logging::info(&format!(
            "[file-activity] {} touched by peers before modification — sending alerts",
            path.display()
        ));
        // Swarm membership query for names routed via thin SwarmServiceHandle
        // (Wave 4.1.2 SwarmStateInMonitor precedent). Direct .read() here is *only*
        // for .event_tx access (fanout/send/queue logic now inside this method).
        let members = swarm_members.read().await;
        let current_name =
            super::handles::SwarmServiceHandle::get_member_friendly_name_for_notification(
                swarm_members,
                session_id,
            )
            .await;

        // Alert the current agent about previous peer touches (one per agent).
        if let Some(member) = members.get(session_id) {
            for prev in previous_touches {
                let prev_name =
                    super::handles::SwarmServiceHandle::get_member_friendly_name_for_notification(
                        swarm_members,
                        &prev.session_id,
                    )
                    .await;
                let scope = file_activity_scope_label(prev, touch);
                let intent_suffix = prev
                    .intent
                    .as_ref()
                    .map(|intent| format!(" — intent: {}", intent))
                    .unwrap_or_default();
                let alert_msg = format!(
                    "⚠ File activity: {} — {} — {} previously {} this file{}{}",
                    path.display(),
                    scope,
                    prev_name.as_deref().unwrap_or(&prev.session_id[..8]),
                    prev.op.as_str(),
                    prev.summary
                        .as_ref()
                        .map(|s| format!(": {}", s))
                        .unwrap_or_default(),
                    intent_suffix
                );
                let notification = ServerEvent::Notification {
                    from_session: prev.session_id.clone(),
                    from_name: prev_name,
                    notification_type: NotificationType::FileConflict {
                        path: path.display().to_string(),
                        operation: prev.op.as_str().to_string(),
                        intent: prev.intent.clone(),
                        summary: prev.summary.clone(),
                        detail: prev.detail.clone(),
                    },
                    message: alert_msg.clone(),
                };
                let _ = member.event_tx.send(notification);

                if !queue_soft_interrupt_for_session(
                    session_id,
                    alert_msg.clone(),
                    false,
                    SoftInterruptSource::System,
                    soft_interrupt_queues,
                    sessions,
                )
                .await
                {
                    crate::logging::warn(&format!(
                        "Failed to queue file-activity soft interrupt for session {}",
                        session_id
                    ));
                }
            }
        }

        // Alert previous agents about the current modification.
        for prev in previous_touches {
            if let Some(prev_member) = members.get(&prev.session_id) {
                let scope = file_activity_scope_label(prev, touch);
                let intent_suffix = touch
                    .intent
                    .as_ref()
                    .map(|intent| format!(" — intent: {}", intent))
                    .unwrap_or_default();
                let alert_msg = format!(
                    "⚠ File activity: {} — {} — {} just {} this file you previously worked with{}{}",
                    path.display(),
                    scope,
                    current_name
                        .as_deref()
                        .unwrap_or(&session_id[..8.min(session_id.len())]),
                    touch.op.as_str(),
                    touch
                        .summary
                        .as_ref()
                        .map(|s| format!(": {}", s))
                        .unwrap_or_default(),
                    intent_suffix
                );
                let notification = ServerEvent::Notification {
                    from_session: session_id.to_string(),
                    from_name: current_name.clone(),
                    notification_type: NotificationType::FileConflict {
                        path: path.display().to_string(),
                        operation: touch.op.as_str().to_string(),
                        intent: touch.intent.clone(),
                        summary: touch.summary.clone(),
                        detail: touch.detail.clone(),
                    },
                    message: alert_msg.clone(),
                };
                let _ = prev_member.event_tx.send(notification);

                if !queue_soft_interrupt_for_session(
                    &prev.session_id,
                    alert_msg.clone(),
                    false,
                    SoftInterruptSource::System,
                    soft_interrupt_queues,
                    sessions,
                )
                .await
                {
                    crate::logging::warn(&format!(
                        "Failed to queue file-activity soft interrupt for session {}",
                        prev.session_id
                    ));
                }
            }
        }
    }

    // Thin method on MaintenanceServiceHandle for the monitor_bus responsibility
    // (Move 6) — attempted during Ola 3 but created duplicate + broken delegation.
    // The working thin entry lives in server.rs for now.
    // Full ownership moves behind the handle in Ola 4 #2 (Move 6 completion).
    // run_monitor_bus thin seam was partially extracted here in Ola 3 but created
    // duplicate definition + broken delegation to a non-existent Server method.
    // The working thin entry point lives in server.rs (Server::run_monitor_bus
    // which forwards to the real heavy Server::monitor_bus).
    //
    // Full migration of monitor_bus ownership + this entry behind
    // MaintenanceServiceHandle is Ola 4 priority #2 (Move 6 completion).
    // Removed the duplicate here to restore selfdev profile compilation.
}

pub(super) async fn dispatch_background_task_completion(
    task: &crate::bus::BackgroundTaskCompleted,
    sessions: &SessionAgents,
    soft_interrupt_queues: &SessionInterruptQueues,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
) {
    MaintenanceServiceHandle::dispatch_background_task_completion(
        task,
        sessions,
        soft_interrupt_queues,
        swarm_members,
    )
    .await;
}

pub(super) async fn dispatch_background_task_progress(
    task: &crate::bus::BackgroundTaskProgressEvent,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
) {
    MaintenanceServiceHandle::dispatch_background_task_progress(task, swarm_members).await;
}

pub(super) async fn dispatch_ui_activity(
    activity: &crate::bus::UiActivity,
    swarm_members: &Arc<RwLock<HashMap<String, SwarmMember>>>,
) {
    MaintenanceServiceHandle::dispatch_ui_activity(activity, swarm_members).await;
}
