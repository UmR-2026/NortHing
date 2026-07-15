//! CronService – free helper functions and internal types.

use super::schedule::{compute_initial_next_run_at_ms, validate_schedule};
use super::types::{CronJob, CronJobPayload, CronJobTarget, CronLaunchSpec, CronSchedule, CronWorkspaceRef};
use crate::agentic::coordination::{DialogQueuePriority, DialogSubmissionPolicy, DialogTriggerSource};
use crate::util::errors::{NortHingError, NortHingResult};
use chrono::{Local, SecondsFormat, TimeZone, Utc};
use northhing_runtime_ports::AgentDialogPrependedReminder;

pub(super) fn reconcile_loaded_job(job: &mut CronJob, now_ms: i64) -> NortHingResult<bool> {
    let original = job.clone();

    job.target = materialize_target(job.target.clone());
    validate_request_fields(&job.name, &job.payload, &job.target)?;
    validate_schedule(&job.schedule, job.created_at_ms)?;

    if job.updated_at_ms < job.created_at_ms {
        job.updated_at_ms = job.created_at_ms;
    }

    if let CronSchedule::Every { anchor_ms, .. } = &mut job.schedule {
        if anchor_ms.is_none() {
            *anchor_ms = Some(job.created_at_ms);
        }
    }

    if job.state.recover_interrupted_turn_after_restart(
        now_ms,
        "Application restarted before the scheduled job finished".to_string(),
    ) {
        job.updated_at_ms = now_ms;
    }

    if !job.enabled {
        job.state.mark_disabled();
    } else if job.state.pending_trigger_at_ms.is_some() {
        job.state.ensure_pending_retry_at(now_ms);
    } else if job.state.next_run_at_ms.is_none() {
        job.state.next_run_at_ms = compute_initial_next_run_at_ms(job, now_ms)?;
    }

    Ok(job != &original)
}

pub(super) fn validate_request_fields(
    name: &str,
    payload: &CronJobPayload,
    target: &CronJobTarget,
) -> NortHingResult<()> {
    if name.trim().is_empty() {
        return Err(NortHingError::validation("Scheduled job name must not be empty"));
    }
    if payload.text.trim().is_empty() {
        return Err(NortHingError::validation(
            "Scheduled job payload.text must not be empty",
        ));
    }

    validate_target(target)?;

    Ok(())
}

pub(super) fn materialize_schedule(schedule: CronSchedule, anchor_ms: i64) -> CronSchedule {
    match schedule {
        CronSchedule::Every {
            every_ms,
            anchor_ms: None,
        } => CronSchedule::Every {
            every_ms,
            anchor_ms: Some(anchor_ms),
        },
        other => other,
    }
}

pub(super) fn materialize_target(target: CronJobTarget) -> CronJobTarget {
    match target {
        CronJobTarget::Session { session_id, workspace } => CronJobTarget::Session {
            session_id: session_id.trim().to_string(),
            workspace: materialize_workspace_ref(workspace),
        },
        CronJobTarget::Workspace { workspace, launch } => CronJobTarget::Workspace {
            workspace: materialize_workspace_ref(workspace),
            launch: materialize_launch_spec(launch),
        },
    }
}

pub(super) fn materialize_workspace_ref(workspace: CronWorkspaceRef) -> CronWorkspaceRef {
    CronWorkspaceRef {
        workspace_id: workspace
            .workspace_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        workspace_path: workspace.workspace_path.trim().to_string(),
        remote_connection_id: workspace
            .remote_connection_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        remote_ssh_host: workspace
            .remote_ssh_host
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    }
}

pub(super) fn materialize_launch_spec(launch: CronLaunchSpec) -> CronLaunchSpec {
    CronLaunchSpec {
        agent_type: normalize_agent_type(&launch.agent_type),
        model_id: launch
            .model_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    }
}

pub(super) fn normalize_agent_type(agent_type: &str) -> String {
    if agent_type.trim().is_empty() {
        "agentic".to_string()
    } else {
        agent_type.trim().to_string()
    }
}

pub(super) fn validate_target(target: &CronJobTarget) -> NortHingResult<()> {
    validate_workspace_ref(target.workspace())?;

    match target {
        CronJobTarget::Session { session_id, .. } => {
            if session_id.trim().is_empty() {
                return Err(NortHingError::validation("Scheduled job sessionId must not be empty"));
            }
        }
        CronJobTarget::Workspace { launch, .. } => {
            if launch.agent_type.trim().is_empty() {
                return Err(NortHingError::validation(
                    "Scheduled job launch.agentType must not be empty",
                ));
            }
        }
    }

    Ok(())
}

pub(super) fn validate_workspace_ref(workspace: &CronWorkspaceRef) -> NortHingResult<()> {
    if workspace.workspace_path.trim().is_empty() {
        return Err(NortHingError::validation(
            "Scheduled job workspacePath must not be empty",
        ));
    }
    Ok(())
}

pub(super) fn matches_workspace_filter(
    workspace: &CronWorkspaceRef,
    workspace_path: Option<&str>,
    workspace_id: Option<&str>,
    remote_connection_id: Option<&str>,
) -> bool {
    let workspace_path_matches = workspace_path
        .map(|value| workspace.workspace_path == value)
        .unwrap_or(true);
    let workspace_id_matches = workspace_id
        .map(|value| workspace.workspace_id.as_deref() == Some(value) || workspace.workspace_id.is_none())
        .unwrap_or(true);
    let remote_connection_matches = remote_connection_id
        .map(|value| workspace.remote_connection_id.as_deref() == Some(value))
        .unwrap_or(true);

    workspace_path_matches && workspace_id_matches && remote_connection_matches
}

pub(super) fn next_wakeup_for_job(job: &CronJob) -> Option<i64> {
    job.state.next_wakeup_at_ms()
}

pub(super) fn format_scheduled_job_user_input(
    payload: &str,
    current_ms: i64,
) -> (String, Vec<AgentDialogPrependedReminder>) {
    let current_time = Local
        .timestamp_millis_opt(current_ms)
        .single()
        .map(|datetime| datetime.to_rfc3339_opts(SecondsFormat::Secs, false))
        .unwrap_or_else(|| current_ms.to_string());

    (
        payload.to_string(),
        vec![AgentDialogPrependedReminder {
            kind: "scheduled_job".to_string(),
            text: format!(
                "This message was triggered by a scheduled job.\nCurrent time: {}",
                current_time
            ),
        }],
    )
}

pub(super) fn scheduled_job_policy() -> DialogSubmissionPolicy {
    DialogSubmissionPolicy::new(DialogTriggerSource::ScheduledJob, DialogQueuePriority::Low, true)
}

pub(super) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(super) struct EnqueueInput {
    pub job_id: String,
    pub job_name: String,
    pub turn_id: String,
    pub target: CronJobTarget,
    pub user_input: String,
    pub prepended_messages: Vec<AgentDialogPrependedReminder>,
}

pub(super) struct ResolvedEnqueueSubmission {
    pub session_id: String,
    pub workspace_path: String,
    pub agent_type: String,
}

pub(super) fn submit_target_session_id(enqueue_input: &EnqueueInput) -> &str {
    match &enqueue_input.target {
        CronJobTarget::Session { session_id, .. } => session_id.as_str(),
        CronJobTarget::Workspace { .. } => "<new-session>",
    }
}

/// Permanent failure: coordinator cannot load session metadata (session deleted from disk).
pub(super) fn cron_enqueue_error_is_missing_session(error: &str) -> bool {
    error.contains("Session metadata not found")
}
