//! Core helpers and the cancellation port for agentic::coordination.

use super::coordinator::ConversationCoordinator;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::agentic::tools::ToolRuntimeRestrictions;
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::util::errors::NortHingError;
use northhing_runtime_ports::{AgentTurnCancellationPort, DelegationPolicy};
use std::time::Duration;

// Global coordinator singleton
pub static GLOBAL_COORDINATOR: std::sync::OnceLock<std::sync::Arc<ConversationCoordinator>> =
    std::sync::OnceLock::new();

/// Get global coordinator
///
/// Returns `None` if coordinator hasn't been initialized
pub fn global_coordinator() -> Option<std::sync::Arc<ConversationCoordinator>> {
    GLOBAL_COORDINATOR.get().cloned()
}

pub(crate) async fn is_ai_session_title_generation_enabled() -> bool {
    match crate::service::config::get_global_config_service().await {
        Ok(service) => service
            .config::<bool>(Some("app.ai_experience.enable_session_title_generation"))
            .await
            .unwrap_or(true),
        Err(_) => true,
    }
}

/// Build a `SessionRelationship` for a subagent.
pub(crate) fn build_subagent_session_relationship(
    parent_info: Option<&SubagentParentInfo>,
    agent_type: &str,
) -> SessionRelationship {
    SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: parent_info.map(|info| info.session_id.clone()),
        parent_request_id: None,
        parent_dialog_turn_id: parent_info.map(|info| info.dialog_turn_id.clone()),
        parent_turn_index: None,
        parent_tool_call_id: parent_info.map(|info| info.tool_call_id.clone()),
        subagent_type: Some(agent_type.to_string()),
    }
}

/// System reminder injected into forked subagent sessions.
pub(crate) fn fork_subagent_system_reminder() -> String {
    "<system_reminder>You are now running as a forked subagent. Messages before this reminder were inherited from the parent agent as context. Messages after this reminder are the request for you. Do not call the Task tool to launch another subagent. Use the tools available to complete the task directly.</system_reminder>".to_string()
}

/// Compute `ToolRuntimeRestrictions` for a given delegation policy.
pub(crate) fn runtime_tool_restrictions_for_delegation_policy(
    delegation_policy: DelegationPolicy,
) -> ToolRuntimeRestrictions {
    let mut restrictions = ToolRuntimeRestrictions::default();
    if !delegation_policy.allow_subagent_spawn {
        restrictions.denied_tool_names.insert("Task".to_string());
        restrictions.denied_tool_messages.insert(
            "Task".to_string(),
            "Recursive subagent delegation is blocked. Use direct tools instead.".to_string(),
        );
    }
    restrictions
}

#[async_trait::async_trait]
impl AgentTurnCancellationPort for ConversationCoordinator {
    async fn cancel_turn(
        &self,
        request: northhing_runtime_ports::AgentTurnCancellationRequest,
    ) -> northhing_runtime_ports::PortResult<northhing_runtime_ports::AgentTurnCancellationResult> {
        let session_id = request.session_id;
        if let Some(turn_id) = request.turn_id {
            self.cancel_dialog_turn(&session_id, &turn_id).await.map_err(|error| {
                northhing_runtime_ports::PortError::new(
                    northhing_runtime_ports::PortErrorKind::Backend,
                    error.to_string(),
                )
            })?;

            return Ok(northhing_runtime_ports::AgentTurnCancellationResult {
                session_id,
                turn_id: Some(turn_id),
                requested: true,
            });
        }

        let wait_timeout = Duration::from_millis(request.wait_timeout_ms.unwrap_or(1500));
        let cancelled_turn_id = self
            .cancel_active_turn_for_session(&session_id, wait_timeout)
            .await
            .map_err(|error| {
                northhing_runtime_ports::PortError::new(
                    northhing_runtime_ports::PortErrorKind::Backend,
                    error.to_string(),
                )
            })?;
        let requested = cancelled_turn_id.is_some();

        Ok(northhing_runtime_ports::AgentTurnCancellationResult {
            session_id,
            turn_id: cancelled_turn_id,
            requested,
        })
    }
}
