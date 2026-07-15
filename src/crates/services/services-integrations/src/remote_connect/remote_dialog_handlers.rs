//! Remote-connect dialog sub-handler (Round 11b split).
//!
//! Owns the dialog-submission DTOs and the `submit_remote_dialog` orchestration
//! fn. Struct-owner mapping per QClaw R11b §5:
//! - `RemoteDialogQueuePriority`
//! - `RemoteDialogSubmissionPolicy`
//! - `RemoteDialogSubmissionRequest`
//! - `RemoteDialogResolvedSubmission`
//! - `RemoteDialogSubmitOutcome`
//! - `RemoteDialogSchedulerOutcomeFact`
//! - `RemoteTerminalPrewarmRequest` (related to dialog submit)
//! - `RemoteDialogRuntimeHost`
//!
//! The dialog response helper `remote_dialog_submit_response` lives here
//! because it is the only producer of `RemoteResponse::MessageSent` from the
//! dialog submit path.

use super::RemoteConnectSubmissionSource;
use super::RemoteResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDialogQueuePriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteDialogSubmissionPolicy {
    pub source: RemoteConnectSubmissionSource,
    pub queue_priority: RemoteDialogQueuePriority,
    pub skip_tool_confirmation: bool,
}

impl RemoteDialogSubmissionPolicy {
    pub const fn for_source(source: RemoteConnectSubmissionSource) -> Self {
        Self {
            source,
            queue_priority: RemoteDialogQueuePriority::Normal,
            skip_tool_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteDialogSubmissionRequest<ImageContext> {
    pub session_id: String,
    pub content: String,
    pub agent_type: Option<String>,
    pub image_contexts: Vec<ImageContext>,
    pub policy: RemoteDialogSubmissionPolicy,
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTerminalPrewarmRequest {
    pub session_id: String,
    pub binding_workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteDialogResolvedSubmission<ImageContext> {
    pub session_id: String,
    pub content: String,
    pub resolved_agent_type: String,
    pub binding_workspace: Option<String>,
    pub image_contexts: Vec<ImageContext>,
    pub policy: RemoteDialogSubmissionPolicy,
    pub turn_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteDialogSubmitOutcome {
    Started { session_id: String, turn_id: String },
    Queued { session_id: String, turn_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteDialogSchedulerOutcomeFact {
    Started { session_id: String, turn_id: String },
    Queued { session_id: String, turn_id: String },
}

pub fn remote_dialog_submit_outcome_from_scheduler(
    fact: RemoteDialogSchedulerOutcomeFact,
) -> RemoteDialogSubmitOutcome {
    match fact {
        RemoteDialogSchedulerOutcomeFact::Started { session_id, turn_id } => {
            RemoteDialogSubmitOutcome::Started { session_id, turn_id }
        }
        RemoteDialogSchedulerOutcomeFact::Queued { session_id, turn_id } => {
            RemoteDialogSubmitOutcome::Queued { session_id, turn_id }
        }
    }
}

/// Host callbacks required by remote-connect dialog execution.
#[async_trait::async_trait]
pub trait RemoteDialogRuntimeHost: Send + Sync {
    type ImageContext: Send + Sync + 'static;

    fn ensure_tracker(&self, session_id: &str);

    async fn resolve_binding_workspace(&self, session_id: &str) -> Option<String>;

    async fn remote_session_exists(&self, session_id: &str) -> Result<bool, String>;

    async fn restore_remote_session(&self, session_id: &str, workspace_path: &str) -> Result<(), String>;

    fn prewarm_remote_terminal(&self, request: RemoteTerminalPrewarmRequest);

    fn generate_turn_id(&self) -> String;

    async fn submit_dialog(
        &self,
        submission: RemoteDialogResolvedSubmission<Self::ImageContext>,
    ) -> Result<RemoteDialogSubmitOutcome, String>;
}

pub async fn submit_remote_dialog<H>(
    host: &H,
    request: RemoteDialogSubmissionRequest<H::ImageContext>,
) -> Result<RemoteDialogSubmitOutcome, String>
where
    H: RemoteDialogRuntimeHost + ?Sized,
{
    let RemoteDialogSubmissionRequest {
        session_id,
        content,
        agent_type,
        image_contexts,
        policy,
        turn_id,
    } = request;

    host.ensure_tracker(&session_id);

    let binding_workspace = host.resolve_binding_workspace(&session_id).await;
    let session_exists = host.remote_session_exists(&session_id).await?;

    if let Some(workspace_path) =
        super::remote_request_builders::remote_session_restore_target(session_exists, binding_workspace.as_deref())
    {
        let _ = host.restore_remote_session(&session_id, workspace_path).await;
    }

    host.prewarm_remote_terminal(RemoteTerminalPrewarmRequest {
        session_id: session_id.clone(),
        binding_workspace: binding_workspace.clone(),
    });

    let resolved_agent_type =
        super::remote_workspace_resolver::resolve_remote_agent_type(agent_type.as_deref()).to_string();
    let turn_id = turn_id.unwrap_or_else(|| host.generate_turn_id());

    host.submit_dialog(RemoteDialogResolvedSubmission {
        session_id,
        content,
        resolved_agent_type,
        binding_workspace,
        image_contexts,
        policy,
        turn_id,
    })
    .await
}

pub fn remote_dialog_submit_response(result: Result<RemoteDialogSubmitOutcome, String>) -> RemoteResponse {
    match result {
        Ok(RemoteDialogSubmitOutcome::Started { session_id, turn_id })
        | Ok(RemoteDialogSubmitOutcome::Queued { session_id, turn_id }) => {
            RemoteResponse::MessageSent { session_id, turn_id }
        }
        Err(message) => RemoteResponse::Error { message },
    }
}
