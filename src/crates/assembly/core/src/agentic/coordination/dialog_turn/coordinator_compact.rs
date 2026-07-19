//! Sub-domain: compact.
//! Spec §2.1 — facade method extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains the public `compact_session_manually` method that persists a
//! maintenance turn running the context compression tool.

use super::super::coordinator::*;

use crate::agentic::core::SessionState;
use crate::agentic::events::AgenticEvent;
use crate::agentic::execution::ExecutionContext;
use crate::agentic::tools::ToolRuntimeRestrictions;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::DelegationPolicy;
use std::collections::HashMap;
use std::path::Path;

const MANUAL_COMPACTION_COMMAND: &str = "/compact";

impl ConversationCoordinator {
    /// Compact the active session context as a persisted maintenance turn.
    pub async fn compact_session_manually(&self, session_id: String) -> NortHingResult<()> {
        let session = self
            .session_manager
            .get_session(&session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;

        match &session.state {
            SessionState::Idle => {}
            SessionState::Processing { current_turn_id, phase } => {
                return Err(NortHingError::Validation(format!(
                    "Session is still processing: current_turn_id={}, phase={:?}",
                    current_turn_id, phase
                )));
            }
            SessionState::Error { error, .. } => {
                return Err(NortHingError::Validation(format!(
                    "Session must be idle before manual compaction: {}",
                    error
                )));
            }
        }

        let context_messages = self.session_manager.get_context_messages(&session_id).await?;
        let needs_restore = if context_messages.is_empty() {
            true
        } else {
            context_messages.len() == 1 && !session.dialog_turn_ids.is_empty()
        };

        if needs_restore {
            let workspace_path = session.config.workspace_path.as_deref().ok_or_else(|| {
                NortHingError::Validation(format!(
                    "workspace_path is required when restoring session: {}",
                    session_id
                ))
            })?;
            self.session_manager
                .restore_session(Path::new(workspace_path), &session_id)
                .await?;
        }

        let context_messages = self.session_manager.get_context_messages(&session_id).await?;
        let turn_index = self.session_manager.get_turn_count(&session_id);
        let user_message_metadata = Some(Self::manual_compaction_metadata());
        let turn_id = self
            .session_manager
            .start_maintenance_turn(
                &session_id,
                MANUAL_COMPACTION_COMMAND.to_string(),
                None,
                user_message_metadata.clone(),
            )
            .await?;

        self.emit_event(AgenticEvent::DialogTurnStarted {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            turn_index,
            user_input: MANUAL_COMPACTION_COMMAND.to_string(),
            original_user_input: None,
            user_message_metadata: user_message_metadata.clone(),
        })
        .await;

        let current_tokens = Self::estimate_context_tokens(&context_messages);
        let manual_workspace = Self::build_workspace_binding(&session.config).await;
        let manual_workspace_services = Self::build_workspace_services(&manual_workspace).await;
        let manual_execution_context = ExecutionContext {
            session_id: session_id.clone(),
            dialog_turn_id: turn_id.clone(),
            turn_index,
            agent_type: session.agent_type.clone(),
            workspace: manual_workspace,
            context: HashMap::new(),
            subagent_parent_info: None,
            delegation_policy: DelegationPolicy::top_level(),
            skip_tool_confirmation: true,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            workspace_services: manual_workspace_services,
            round_injection: None,
            recover_partial_on_cancel: false,
        };
        let session_max_tokens = session.config.max_context_tokens;

        // Unify context_window: min(model capability, session config)
        let model_context_window = match crate::infrastructure::ai::get_global_ai_client_factory().await {
            Ok(factory) => {
                let model_id = session.config.model_id.as_deref().unwrap_or("default");
                match factory.get_client_resolved(model_id).await {
                    Ok(client) => Some(client.config.context_window as usize),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };
        let context_window = match model_context_window {
            Some(mcw) => mcw.min(session_max_tokens),
            None => session_max_tokens,
        };
        let compression_threshold = session.config.compression_threshold;

        match self
            .execution_engine
            .compact_session_context(
                session_id.clone(),
                turn_id.clone(),
                manual_execution_context,
                context_messages,
                current_tokens,
                "manual",
            )
            .await
        {
            Ok(outcome) => {
                let model_round = Self::build_manual_compaction_round_completed(
                    &turn_id,
                    &outcome,
                    context_window,
                    compression_threshold,
                );
                self.session_manager
                    .complete_maintenance_turn(&session_id, &turn_id, vec![model_round], outcome.duration_ms)
                    .await?;
                self.session_manager
                    .update_session_state(&session_id, SessionState::Idle)
                    .await?;

                // Compact-path emission: unlike the main dialog turn path (which
                // emits via sub_handle_out after persistence), compact turns bypass
                // sub_handle_out, so this self-emit is the only DialogTurnCompleted event.
                self.emit_event(AgenticEvent::DialogTurnCompleted {
                    session_id,
                    turn_id,
                    total_rounds: 1,
                    total_tools: 1,
                    duration_ms: outcome.duration_ms,
                    partial_recovery_reason: None,
                    success: Some(true),
                    finish_reason: Some("complete".to_string()),
                })
                .await;

                Ok(())
            }
            Err(err) => {
                let error_text = err.to_string();
                let compression_id = format!("compression_{}", uuid::Uuid::new_v4());
                let model_round = Self::build_manual_compaction_round_failed(
                    &turn_id,
                    compression_id,
                    &error_text,
                    context_window,
                    compression_threshold,
                );
                let _ = self
                    .session_manager
                    .fail_maintenance_turn(&session_id, &turn_id, error_text.clone(), vec![model_round])
                    .await;
                let _ = self
                    .session_manager
                    .update_session_state(&session_id, SessionState::Idle)
                    .await;
                self.emit_event(AgenticEvent::DialogTurnFailed {
                    session_id,
                    turn_id,
                    error: error_text.clone(),
                    error_category: Some(err.error_category()),
                    error_detail: Some(err.error_detail()),
                })
                .await;
                Err(err)
            }
        }
    }
}
