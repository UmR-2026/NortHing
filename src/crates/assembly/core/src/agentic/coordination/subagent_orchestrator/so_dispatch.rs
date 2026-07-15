//! Sub-domain: subagent dispatch entry points + request resolution.
//! Spec step-3.7 — extracted from subagent_orchestrator.rs (R50b refactor).

use super::super::coordinator::*;
use super::super::handoff::{CoordinatorHiddenSubagentHandoff, SubAgentHandoff};
use super::super::ports::*;
use super::so_types::*;
use crate::agentic::core::{InternalReminderKind, Message, SessionConfig};
use crate::agentic::fork_agent::ForkAgentContextSnapshot;
use crate::agentic::goal_mode::{
    effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
    should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
    user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
};
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::agentic::tools::{
    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_dispatch::{ActorRuntime, USE_LIGHTWEIGHT_ACTOR};
use northhing_runtime_ports::{AgentBackgroundResultRequest, DelegationPolicy, SubagentContextMode};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

impl ConversationCoordinator {
    async fn resolve_hidden_subagent_execution_request(
        &self,
        request: SubagentExecutionRequest,
    ) -> NortHingResult<HiddenSubagentExecutionRequest> {
        let task_description = request.task_description.trim().to_string();
        if task_description.is_empty() {
            return Err(NortHingError::Validation(
                "task_description is required when creating a subagent session".to_string(),
            ));
        }

        let model_id = request
            .model_id
            .as_deref()
            .map(str::trim)
            .filter(|model_id| !model_id.is_empty())
            .map(str::to_string);
        let created_by = Some(format!("session-{}", request.subagent_parent_info.session_id));

        match request.context_mode {
            SubagentContextMode::Fresh => {
                let agent_type = request.subagent_type.ok_or_else(|| {
                    NortHingError::Validation("subagent_type is required when context_mode is 'fresh'".to_string())
                })?;
                let workspace_path = request.workspace_path.ok_or_else(|| {
                    NortHingError::Validation(
                        "workspace_path is required when creating a fresh subagent session".to_string(),
                    )
                })?;

                Ok(HiddenSubagentExecutionRequest {
                    session_name: format!("Subagent: {}", task_description),
                    agent_type,
                    session_config: Self::build_session_config_for_workspace(workspace_path, model_id).await,
                    initial_messages: vec![Message::user(task_description.clone())],
                    user_input_text: task_description,
                    created_by,
                    subagent_parent_info: Some(request.subagent_parent_info),
                    context: request.context,
                    delegation_policy: request.delegation_policy,
                    runtime_tool_restrictions: super::so_types::runtime_tool_restrictions_for_delegation_policy(
                        request.delegation_policy,
                    ),
                    prompt_cache_source_session_id: None,
                })
            }
            SubagentContextMode::Fork => {
                if request.subagent_type.is_some() {
                    return Err(NortHingError::Validation(
                        "subagent_type is not allowed when context_mode is 'fork'".to_string(),
                    ));
                }
                if request.workspace_path.is_some() {
                    return Err(NortHingError::Validation(
                        "workspace_path is not allowed when context_mode is 'fork'".to_string(),
                    ));
                }
                if model_id.is_some() {
                    return Err(NortHingError::Validation(
                        "model_id is not allowed when context_mode is 'fork'".to_string(),
                    ));
                }

                let snapshot = self
                    .capture_fork_agent_context_snapshot(&request.subagent_parent_info.session_id)
                    .await?;
                let mut initial_messages = snapshot.messages.clone();
                initial_messages.push(Message::internal_reminder(
                    InternalReminderKind::ForkSubagent,
                    super::so_types::fork_subagent_system_reminder(),
                ));
                initial_messages.push(Message::user(task_description.clone()));

                Ok(HiddenSubagentExecutionRequest {
                    session_name: format!("Fork: {}", task_description),
                    agent_type: snapshot.parent_agent_type.clone(),
                    session_config: snapshot.build_child_session_config(None),
                    initial_messages,
                    user_input_text: task_description,
                    created_by,
                    subagent_parent_info: Some(request.subagent_parent_info),
                    context: request.context,
                    delegation_policy: request.delegation_policy,
                    runtime_tool_restrictions: super::so_types::runtime_tool_restrictions_for_delegation_policy(
                        request.delegation_policy,
                    ),
                    prompt_cache_source_session_id: Some(snapshot.parent_session_id),
                })
            }
        }
    }

    /// Execute subagent task directly
    /// DialogTurnStarted event not needed for now
    ///
    /// Returns SubagentResult with the final text response
    ///
    /// B-2: routes through `CoordinatorHiddenSubagentHandoff::handoff`
    /// (the canonical sub-agent handoff entry point) instead of calling
    /// the legacy `execute_hidden_subagent_internal` directly. Per-turn
    /// enforcement now happens at the trait boundary.
    pub(crate) async fn execute_subagent(
        &self,
        request: SubagentExecutionRequest,
        _cancel_token: Option<&CancellationToken>,
        _timeout_seconds: Option<u64>,
        _actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> NortHingResult<SubagentResult> {
        let hidden_request = self.resolve_hidden_subagent_execution_request(request).await?;
        let turn_id = subagent_turn_id(&hidden_request);
        let handoff = CoordinatorHiddenSubagentHandoff::new();
        handoff.handoff(&turn_id, hidden_request).await
    }

    pub(crate) async fn start_background_subagent(
        &self,
        request: SubagentExecutionRequest,
        _timeout_seconds: Option<u64>,
        _actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> NortHingResult<BackgroundSubagentStartResult> {
        let request = self.resolve_hidden_subagent_execution_request(request).await?;
        let agent_type = request.agent_type.clone();
        let subagent_parent_info = request.subagent_parent_info.clone().ok_or_else(|| {
            NortHingError::Validation(
                "subagent_parent_info is required when creating a background subagent session".to_string(),
            )
        })?;
        let parent_session = self
            .session_manager
            .get_session(&subagent_parent_info.session_id)
            .ok_or_else(|| {
                NortHingError::NotFound(format!("Parent session not found: {}", subagent_parent_info.session_id))
            })?;
        let parent_agent_type = parent_session.agent_type.clone();
        let parent_workspace_path = parent_session.config.workspace_path.clone();
        let background_task_id = format!("bg-subagent-{}", uuid::Uuid::new_v4());
        let background_task_id_for_delivery = background_task_id.clone();
        let task_description = request.user_input_text.clone();
        // B-2: cancel token wiring is currently dropped at the handoff
        // boundary (the canonical `CoordinatorHiddenSubagentHandoff` does
        // not yet plumb per-call cancel through to the global coordinator's
        // phase2 loop). It is preserved as a derived `parent_cancel_token`
        // step above so a follow-up R73+ handoff enhancement can thread it
        // through without re-introducing the lookup. The child's per-turn
        // enforcement (via `TurnHandoffCounter`) remains in force.
        let _parent_cancel_token = self
            .execution_engine
            .cancel_token_for_dialog_turn(&subagent_parent_info.dialog_turn_id)
            .map(|token| token.child_token());
        let turn_id = subagent_turn_id(&request);

        // B-2: route the spawned future through `CoordinatorHiddenSubagentHandoff`
        // (the canonical sub-agent handoff entry point) instead of calling
        // the legacy `execute_hidden_subagent_internal` directly. The handoff
        // is owned by the spawn future so its per-turn counter moves with it.
        let handoff = CoordinatorHiddenSubagentHandoff::new();

        tokio::spawn(async move {
            let handoff_result = handoff.handoff(&turn_id, request).await;
            let (delivery_text, display_text) = match handoff_result {
                Ok(result) => (
                    super::so_types::format_background_subagent_delivery_text(
                        &background_task_id_for_delivery,
                        &agent_type,
                        Ok(&result),
                    ),
                    super::so_types::format_background_subagent_display_text(Ok(&result)),
                ),
                Err(error) => (
                    super::so_types::format_background_subagent_delivery_text(
                        &background_task_id_for_delivery,
                        &agent_type,
                        Err(&error),
                    ),
                    super::so_types::format_background_subagent_display_text(Err(&error)),
                ),
            };

            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "kind".to_string(),
                serde_json::Value::String("background_result".to_string()),
            );
            metadata.insert(
                "sourceKind".to_string(),
                serde_json::Value::String("subagent".to_string()),
            );
            metadata.insert(
                "backgroundTaskId".to_string(),
                serde_json::Value::String(background_task_id_for_delivery.clone()),
            );
            metadata.insert("subagentType".to_string(), serde_json::Value::String(agent_type));
            metadata.insert(
                "taskDescription".to_string(),
                serde_json::Value::String(task_description),
            );

            let runtime = match CoreServiceAgentRuntime::global_agent_runtime_with_lifecycle_delivery() {
                Ok(runtime) => runtime,
                Err(error) => {
                    warn!(
                        "Agent runtime lifecycle delivery is not available; background subagent result dropped: background_task_id={}, parent_session_id={}, error={}",
                        background_task_id_for_delivery,
                        subagent_parent_info.session_id,
                        error
                    );
                    return;
                }
            };

            if let Err(error) = runtime
                .deliver_background_result(AgentBackgroundResultRequest {
                    session_id: subagent_parent_info.session_id.clone(),
                    agent_type: parent_agent_type,
                    workspace_path: parent_workspace_path,
                    content: delivery_text,
                    display_content: Some(display_text),
                    metadata,
                })
                .await
            {
                warn!(
                    "Failed to deliver background subagent result: background_task_id={}, parent_session_id={}, error={}",
                    background_task_id_for_delivery,
                    subagent_parent_info.session_id,
                    CoreServiceAgentRuntime::runtime_error_message(error)
                );
            }
        });

        Ok(BackgroundSubagentStartResult { background_task_id })
    }
}

/// B-2: derive the per-turn counter key for a sub-agent handoff.
///
/// Prefers the parent dialog turn id (the main agent's turn that
/// triggered the sub-agent). Falls back to a stable orphan key when
/// no parent info is attached — this happens for legacy
/// `SubagentExecutionRequest` variants that omit the parent (e.g.
/// internal reconciliation tasks).
fn subagent_turn_id(request: &HiddenSubagentExecutionRequest) -> String {
    request
        .subagent_parent_info
        .as_ref()
        .map(|p| p.dialog_turn_id.clone())
        .unwrap_or_else(|| format!("orphan-{}", request.session_name))
}

#[cfg(test)]
mod subagent_turn_id_tests {
    //! QClaw review observation 2: `subagent_turn_id` had no unit test
    //! coverage after the B-2 follow-up. Cover both branches:
    //! (a) parent info present → return parent's dialog_turn_id
    //! (b) parent info absent  → return `orphan-<session_name>` fallback

    use super::*;
    use std::collections::HashMap;

    fn make_request(parent: Option<SubagentParentInfo>, session_name: &str) -> HiddenSubagentExecutionRequest {
        HiddenSubagentExecutionRequest {
            session_name: session_name.to_string(),
            agent_type: "general".to_string(),
            session_config: SessionConfig::default(),
            initial_messages: vec![Message::user("hello".to_string())],
            user_input_text: "hello".to_string(),
            created_by: Some("session-test".to_string()),
            subagent_parent_info: parent,
            context: HashMap::new(),
            delegation_policy: DelegationPolicy::default(),
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            prompt_cache_source_session_id: None,
        }
    }

    #[test]
    fn uses_parent_dialog_turn_id_when_parent_info_present() {
        let parent = SubagentParentInfo {
            tool_call_id: "tc_001".to_string(),
            session_id: "sess_parent".to_string(),
            dialog_turn_id: "turn_uuid_abc".to_string(),
        };
        let req = make_request(Some(parent), "Subagent: anything");
        assert_eq!(subagent_turn_id(&req), "turn_uuid_abc");
    }

    #[test]
    fn falls_back_to_orphan_session_name_when_parent_info_absent() {
        let req = make_request(None, "Subagent: reconciliation task");
        assert_eq!(subagent_turn_id(&req), "orphan-Subagent: reconciliation task");
    }

    #[test]
    fn orphan_fallback_preserves_session_name_verbatim() {
        // Sanity: special characters in session_name are not sanitized
        // (callers control session_name and the key is opaque to consumers).
        let req = make_request(None, "session/with/slashes");
        assert_eq!(subagent_turn_id(&req), "orphan-session/with/slashes");
    }
}
