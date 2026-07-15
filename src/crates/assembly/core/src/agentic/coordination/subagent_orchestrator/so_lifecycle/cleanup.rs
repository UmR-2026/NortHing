//! Sub-domain: subagent cleanup — phase3 finalisation, persist, resource cleanup.
//! Spec step-3.7 — extracted from so_lifecycle.rs (R54a refactor).

use super::super::super::coordinator::*;
use crate::agentic::execution::ExecutionResult;
use crate::util::errors::{NortHingError, NortHingResult};
use std::io;
use tokio::time::Instant;
use tracing::{debug, warn};

impl ConversationCoordinator {
    /// Phase 3 — successful path: persist, cleanup, return.
    pub(crate) async fn execute_hidden_subagent_phase3(
        &self,
        phase2: SubagentPhase2Output,
    ) -> NortHingResult<SubagentResult> {
        let SubagentPhase2Output {
            result,
            session_id,
            dialog_turn_id,
            turn_index,
            user_input_text,
            agent_type,
            subagent_workspace_path,
            subagent_session_storage_path,
            parent_session_id,
            parent_dialog_turn_id,
            parent_tool_call_id,
            subagent_parent_info: _,
            subagent_cancel_token: _,
            execution_task: _,
            execution_scope,
            subagent_started_at: _,
        } = phase2;

        // Persist turn lifecycle before cleaning up the hidden subagent runtime.
        // Phase3 is only called when execution succeeded (Cancelled/TimedOut return Err from phase2).
        let (workspace_turn_status, response_text) = self
            .persist_subagent_result(&result, &session_id, &dialog_turn_id, &agent_type)
            .await?;

        Self::finalize_persisted_turn_in_workspace_if_needed(
            self.session_manager.as_ref(),
            &session_id,
            &dialog_turn_id,
            turn_index,
            &agent_type,
            &user_input_text,
            subagent_workspace_path.as_deref(),
            subagent_session_storage_path.as_deref(),
            Some(workspace_turn_status),
            None,
        )
        .await;

        self.cleanup_subagent_and_return(
            &session_id,
            &dialog_turn_id,
            &agent_type,
            &parent_session_id,
            &parent_dialog_turn_id,
            &parent_tool_call_id,
            response_text,
            execution_scope,
        )
        .await
    }

    /// Persist the completed dialog turn and extract the response text.
    pub(crate) async fn persist_subagent_result(
        &self,
        result: &NortHingResult<ExecutionResult>,
        session_id: &str,
        dialog_turn_id: &str,
        _agent_type: &str,
    ) -> NortHingResult<(crate::service::session::TurnStatus, String)> {
        let execution_result = match result.as_ref() {
            Ok(r) => r.clone(),
            Err(e) => {
                return Err(match e {
                    NortHingError::Service(s) => NortHingError::Service(s.clone()),
                    NortHingError::Agent(s) => NortHingError::Agent(s.clone()),
                    NortHingError::Tool(s) => NortHingError::Tool(s.clone()),
                    NortHingError::AIClient(s) => NortHingError::AIClient(s.clone()),
                    NortHingError::Session(s) => NortHingError::Session(s.clone()),
                    NortHingError::Workspace(s) => NortHingError::Workspace(s.clone()),
                    NortHingError::Validation(s) => NortHingError::Validation(s.clone()),
                    NortHingError::Http(s) => NortHingError::Http(s.clone()),
                    NortHingError::Semaphore(s) => NortHingError::Semaphore(s.clone()),
                    NortHingError::MCPError(s) => NortHingError::MCPError(s.clone()),
                    NortHingError::ProcessError(s) => NortHingError::ProcessError(s.clone()),
                    NortHingError::NotFound(s) => NortHingError::NotFound(s.clone()),
                    NortHingError::NotImplemented(s) => NortHingError::NotImplemented(s.clone()),
                    NortHingError::Timeout(s) => NortHingError::Timeout(s.clone()),
                    NortHingError::Configuration(s) => NortHingError::Configuration(s.clone()),
                    NortHingError::Deserialization(s) => NortHingError::Deserialization(s.clone()),
                    NortHingError::Cancelled(s) => NortHingError::Cancelled(s.clone()),
                    // Io and Serialization have From implementations
                    NortHingError::Io(_) => NortHingError::Io(io::Error::other(e.to_string())),
                    NortHingError::Serialization(_) => {
                        NortHingError::Serialization(serde_json::Error::io(io::Error::other(e.to_string())))
                    }
                    NortHingError::Other(_) => NortHingError::Other(anyhow::anyhow!(e.to_string())),
                });
            }
        };

        let (workspace_turn_status, response_text) = Self::persist_completed_dialog_turn(
            self.session_manager.as_ref(),
            None,
            session_id,
            dialog_turn_id,
            &execution_result,
        )
        .await;

        Ok((workspace_turn_status, response_text))
    }

    /// Clean up subagent resources and return the final SubagentResult.
    pub(crate) async fn cleanup_subagent_and_return(
        &self,
        session_id: &str,
        dialog_turn_id: &str,
        agent_type: &str,
        parent_session_id: &str,
        parent_dialog_turn_id: &str,
        parent_tool_call_id: &str,
        response_text: String,
        mut execution_scope: SubagentExecutionScope,
    ) -> NortHingResult<SubagentResult> {
        debug!(
            "Subagent successful execution produced final text: agent_type={}, session_id={}, dialog_turn_id={}, parent_session_id={}, parent_dialog_turn_id={}, parent_tool_call_id={}, text_len={}",
            agent_type,
            session_id,
            dialog_turn_id,
            parent_session_id,
            parent_dialog_turn_id,
            parent_tool_call_id,
            response_text.len(),
        );
        let cleanup_started_at = Instant::now();
        debug!(
            "Subagent cleanup starting after successful execution: agent_type={}, session_id={}, dialog_turn_id={}",
            agent_type, session_id, dialog_turn_id
        );
        if let Err(e) = self.cleanup_subagent_resources(session_id).await {
            warn!(
                "Failed to cleanup subagent resources: session={}, error={}",
                session_id, e
            );
        } else {
            debug!(
                "Subagent cleanup completed: agent_type={}, session_id={}, dialog_turn_id={}, cleanup_duration_ms={}",
                agent_type,
                session_id,
                dialog_turn_id,
                cleanup_started_at.elapsed().as_millis()
            );
        }
        let mut registry = self.subagent_timeout_registry.write().await;
        registry.remove(session_id);

        debug!(
            "Subagent result returning to caller: agent_type={}, session_id={}, dialog_turn_id={}, status=completed, text_len={}",
            agent_type,
            session_id,
            dialog_turn_id,
            response_text.len()
        );
        execution_scope.disarm();
        Ok(SubagentResult::completed(response_text))
    }
}
