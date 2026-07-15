//! Remote-connect cancel-task sub-handler (Round 11b split).
//!
//! Owns the `RemoteCancel*` types and the `cancel_remote_task` orchestration
//! fn. Struct-owner mapping per QClaw R11b §5:
//! - `RemoteCancelDecision`
//! - `RemoteCancelTaskRequest`
//! - `RemoteCancelRuntimeHost`
//!
//! The cancel response helper `remote_task_cancel_response` lives here
//! because it is the only producer of `RemoteResponse::TaskCancelled`.

use northhing_runtime_ports::RemoteControlStateSnapshot;

use super::RemoteResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteCancelDecision {
    CancelCurrent(String),
    StaleRequestedTurn,
    AlreadyFinished,
    NoRunningTask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCancelTaskRequest {
    pub session_id: String,
    pub requested_turn_id: Option<String>,
}

#[async_trait::async_trait]
pub trait RemoteCancelRuntimeHost: Send + Sync {
    async fn resolve_restore_workspace(&self, session_id: &str) -> Option<String>;

    async fn remote_control_state(&self, session_id: &str) -> Result<Option<RemoteControlStateSnapshot>, String>;

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String>;

    async fn cancel_remote_turn(&self, session_id: &str, turn_id: &str) -> Result<(), String>;
}

pub fn resolve_remote_cancel_decision(
    running_turn_id: Option<&str>,
    requested_turn_id: Option<&str>,
) -> RemoteCancelDecision {
    match (running_turn_id, requested_turn_id) {
        (Some(current_turn_id), Some(req_id)) if req_id != current_turn_id => RemoteCancelDecision::StaleRequestedTurn,
        (Some(current_turn_id), _) => RemoteCancelDecision::CancelCurrent(current_turn_id.to_string()),
        (None, Some(_)) => RemoteCancelDecision::AlreadyFinished,
        (None, None) => RemoteCancelDecision::NoRunningTask,
    }
}

pub async fn cancel_remote_task<H>(host: &H, request: RemoteCancelTaskRequest) -> Result<(), String>
where
    H: RemoteCancelRuntimeHost + ?Sized,
{
    let RemoteCancelTaskRequest {
        session_id,
        requested_turn_id,
    } = request;

    let mut state = host.remote_control_state(&session_id).await?;
    if state.is_none() {
        let workspace_path = host
            .resolve_restore_workspace(&session_id)
            .await
            .ok_or_else(|| format!("Workspace path not available for session: {}", session_id))?;
        host.restore_remote_session(&session_id, &workspace_path)
            .await
            .map_err(|error| format!("Session not found: {error}"))?;
        state = host.remote_control_state(&session_id).await?;
    }

    let running_turn_id = state.and_then(|state| state.active_turn_id);
    match resolve_remote_cancel_decision(running_turn_id.as_deref(), requested_turn_id.as_deref()) {
        RemoteCancelDecision::StaleRequestedTurn => Err("This task is no longer running.".to_string()),
        RemoteCancelDecision::CancelCurrent(current_turn_id) => {
            host.cancel_remote_turn(&session_id, &current_turn_id).await
        }
        RemoteCancelDecision::AlreadyFinished => Err("This task is already finished.".to_string()),
        RemoteCancelDecision::NoRunningTask => Err(format!("No running task to cancel for session: {}", session_id)),
    }
}

pub fn remote_task_cancel_response(session_id: impl Into<String>, result: Result<(), String>) -> RemoteResponse {
    match result {
        Ok(()) => RemoteResponse::TaskCancelled {
            session_id: session_id.into(),
        },
        Err(message) => RemoteResponse::Error { message },
    }
}
