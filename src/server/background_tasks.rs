use super::{
    FileAccess, SessionAgents, SessionInterruptQueues, SharedContext, SwarmEvent, SwarmMember,
    VersionedPlan, fanout_session_event, queue_soft_interrupt_for_session,
    session_event_fanout_sender,
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
