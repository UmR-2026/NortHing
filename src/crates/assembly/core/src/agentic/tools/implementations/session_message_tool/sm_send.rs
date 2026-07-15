//! `SessionMessage` tool — target-session preparation and dialog dispatch.
//!
//! Owns the two target-resolution branches of `call_impl`:
//!  * `prepare_existing_target` — locate the target session inside an
//!    already-known workspace, resolve its persisted agent type, and
//!    return the resolved `PreparedTarget`.
//!  * `prepare_new_target` — create a fresh target session through the
//!    service-agent runtime and surface the new `session_id` so the
//!    caller can echo it back to the assistant UI.
//!
//! The shared "wrap + submit + format success" phase lives in
//! `submit_and_format`, which builds the cross-session reminder, calls
//! `submit_dialog_turn`, and renders the final `ToolResult`.

use northhing_agent_runtime::runtime::AgentRuntime;
use northhing_runtime_ports::{
    AgentDialogTurnRequest, AgentSessionCreateRequest, AgentSessionListRequest, AgentSessionReplyRoute,
    AgentSessionWorkspaceRequest,
};
use serde_json::json;

use crate::agentic::coordination::{DialogSubmissionPolicy, DialogTriggerSource};
use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};

use super::sm_types::SessionMessageInput;

/// Resolved details for the target session of a `call_impl` invocation.
///
/// `created_session_id` is `Some(_)` only when the caller asked the tool to
/// create a brand new session (i.e. omitted `session_id` in the input).
#[derive(Debug)]
pub(super) struct PreparedTarget {
    pub session_id: String,
    pub agent_type: String,
    pub created_session_id: Option<String>,
    pub workspace: String,
}

impl super::tool::SessionMessageTool {
    /// Existing-session branch: the caller specified a `session_id`. We
    /// resolve the workspace (either explicit or via the runtime), list
    /// sessions in that workspace, look up the target entry, and surface
    /// its persisted `agent_type` (defaulting to `"agentic"` if empty).
    pub(super) async fn prepare_existing_target(
        &self,
        params: &SessionMessageInput,
        target_session_id: &str,
        source_session_id: &str,
        context: &ToolUseContext,
        runtime: &AgentRuntime,
    ) -> NortHingResult<PreparedTarget> {
        if source_session_id == target_session_id {
            return Err(NortHingError::tool(
                "SessionMessage cannot send a message to the same session".to_string(),
            ));
        }

        let workspace = if let Some(workspace) = params.workspace.as_deref() {
            self.resolve_workspace(workspace, context)?
        } else {
            runtime
                .resolve_session_workspace_path(AgentSessionWorkspaceRequest {
                    session_id: target_session_id.to_string(),
                })
                .await
                .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?
                .ok_or_else(|| {
                    NortHingError::NotFound(format!(
                        "Workspace for session '{}' could not be resolved",
                        target_session_id
                    ))
                })?
        };
        let existing_sessions = runtime
            .list_sessions(AgentSessionListRequest {
                workspace_path: workspace.clone(),
            })
            .await
            .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?;
        let target_session = existing_sessions
            .iter()
            .find(|session| session.session_id == target_session_id)
            .ok_or_else(|| {
                NortHingError::NotFound(format!(
                    "Session '{}' not found in workspace '{}'",
                    target_session_id, workspace
                ))
            })?;

        let persisted_agent_type = target_session.agent_type.trim();
        let target_agent_type = if persisted_agent_type.is_empty() {
            "agentic".to_string()
        } else {
            persisted_agent_type.to_string()
        };

        Ok(PreparedTarget {
            session_id: target_session_id.to_string(),
            agent_type: target_agent_type,
            created_session_id: None,
            workspace,
        })
    }

    /// New-session branch: the caller omitted `session_id`. We resolve the
    /// workspace, pull the supplied `session_name` + `agent_type`, mint a
    /// `createdBy` marker, and ask the runtime to create the session.
    pub(super) async fn prepare_new_target(
        &self,
        params: &SessionMessageInput,
        context: &ToolUseContext,
        runtime: &AgentRuntime,
    ) -> NortHingResult<PreparedTarget> {
        let workspace = self.resolve_workspace(
            params
                .workspace
                .as_deref()
                .ok_or_else(|| NortHingError::tool("workspace is required when session_id is omitted".to_string()))?,
            context,
        )?;
        let session_name = params
            .session_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| NortHingError::tool("session_name is required when session_id is omitted".to_string()))?;
        let agent_type = params
            .agent_type
            .as_ref()
            .ok_or_else(|| NortHingError::tool("agent_type is required when session_id is omitted".to_string()))?
            .as_str()
            .to_string();
        let created_by = self.creator_session_marker(context)?;
        let mut metadata = serde_json::Map::new();
        metadata.insert("createdBy".to_string(), json!(created_by));
        let session = runtime
            .create_session(AgentSessionCreateRequest {
                session_name,
                agent_type: agent_type.clone(),
                workspace_path: Some(workspace.clone()),
                metadata,
            })
            .await
            .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?;

        Ok(PreparedTarget {
            session_id: session.session_id.clone(),
            agent_type: session.agent_type.clone(),
            created_session_id: Some(session.session_id),
            workspace,
        })
    }

    /// Submit a `AgentDialogTurnRequest` through the runtime using the
    /// pre-resolved `PreparedTarget`. Wraps the original message with the
    /// cross-session reminder built in `sm_resolve::format_forwarded_message`
    /// and returns the final `ToolResult` echoing the disposition.
    pub(super) async fn submit_and_format(
        &self,
        params: &SessionMessageInput,
        source_session_id: &str,
        source_workspace: &str,
        target: PreparedTarget,
        runtime: &AgentRuntime,
    ) -> NortHingResult<Vec<ToolResult>> {
        let PreparedTarget {
            session_id: target_session_id,
            agent_type: target_agent_type,
            created_session_id,
            workspace,
        } = target;

        let (forwarded_message, prepended_messages) = self.format_forwarded_message(&params.message);

        runtime
            .submit_dialog_turn(AgentDialogTurnRequest {
                session_id: target_session_id.clone(),
                message: forwarded_message,
                original_message: Some(params.message.clone()),
                turn_id: None,
                agent_type: target_agent_type.clone(),
                workspace_path: Some(workspace.clone()),
                policy: DialogSubmissionPolicy::for_source(DialogTriggerSource::AgentSession),
                reply_route: Some(AgentSessionReplyRoute {
                    source_session_id: source_session_id.to_string(),
                    source_workspace_path: source_workspace.to_string(),
                }),
                prepended_reminders: prepended_messages,
                attachments: Vec::new(),
                metadata: serde_json::Map::new(),
            })
            .await
            .map_err(|error| NortHingError::tool(CoreServiceAgentRuntime::runtime_error_message(error)))?;

        Ok(vec![ToolResult::Result {
            data: json!({
                "success": true,
                "target_workspace": workspace.clone(),
                "target_session_id": target_session_id.clone(),
                "target_agent_type": target_agent_type.clone(),
                "created_session_id": created_session_id.clone(),
            }),
            result_for_assistant: Some(if let Some(created_session_id) = created_session_id {
                format!(
                    "Created session '{}' and accepted the message in workspace '{}' using agent type '{}'.",
                    created_session_id, workspace, target_agent_type
                )
            } else {
                format!(
                    "Message accepted for session '{}' in workspace '{}' using agent type '{}'.",
                    target_session_id, workspace, target_agent_type
                )
            }),
            image_attachments: None,
        }])
    }
}
