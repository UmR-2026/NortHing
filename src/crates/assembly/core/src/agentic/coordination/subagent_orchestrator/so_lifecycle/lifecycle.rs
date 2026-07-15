//! Sub-domain: subagent lifecycle — phase2 execution loop, outcome monitoring.
//! Spec step-3.7 — extracted from so_lifecycle.rs (R54a refactor).

use super::super::super::coordinator::*;
use super::monitor::SubagentExecutionOutcome;
use crate::agentic::core::{ProcessingPhase, SessionState};
use crate::agentic::events::AgenticEvent;
use crate::agentic::execution::{ExecutionContext, ExecutionResult};
use crate::util::errors::{NortHingError, NortHingResult};
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

impl ConversationCoordinator {
    /// Phase 2: start dialog turn, spawn execution task, run deadline/cancel/join loop.
    /// Returns SubagentPhase2Output on success (where result is the execution result).
    /// On cancelled/timed_out returns Err (phase3_* helpers already cleaned up).
    pub(crate) async fn execute_hidden_subagent_phase2(
        &self,
        phase1: &SubagentPhase1Output,
        _cancel_token: Option<&CancellationToken>,
    ) -> NortHingResult<SubagentPhase2Output> {
        // Extract all needed values from phase1 upfront to avoid borrow issues.
        let phase1_owned = SubagentPhase1Output {
            agent_type: phase1.agent_type.clone(),
            session_id: phase1.session_id.clone(),
            initial_messages: phase1.initial_messages.clone(),
            user_input_text: phase1.user_input_text.clone(),
            subagent_parent_info: phase1.subagent_parent_info.clone(),
            context: phase1.context.clone(),
            delegation_policy: phase1.delegation_policy,
            runtime_tool_restrictions: phase1.runtime_tool_restrictions.clone(),
            turn_index: phase1.turn_index,
            dialog_turn_id: phase1.dialog_turn_id.clone(),
            subagent_cancel_token: phase1.subagent_cancel_token.clone(),
            deadline_rx: phase1.deadline_rx.clone(),
            requested_timeout_seconds: phase1.requested_timeout_seconds,
            timeout_seconds: phase1.timeout_seconds,
            timeout_error_message: phase1.timeout_error_message.clone(),
            parent_session_id: phase1.parent_session_id.clone(),
            parent_dialog_turn_id: phase1.parent_dialog_turn_id.clone(),
            parent_tool_call_id: phase1.parent_tool_call_id.clone(),
            subagent_workspace: phase1.subagent_workspace.clone(),
            subagent_started_at: phase1.subagent_started_at,
        };

        let SubagentPhase1Output {
            agent_type,
            session_id,
            initial_messages,
            user_input_text,
            subagent_parent_info,
            context,
            delegation_policy,
            runtime_tool_restrictions,
            turn_index,
            dialog_turn_id,
            subagent_cancel_token,
            mut deadline_rx,
            requested_timeout_seconds: _,
            timeout_seconds,
            timeout_error_message,
            parent_session_id,
            parent_dialog_turn_id,
            parent_tool_call_id,
            subagent_workspace,
            subagent_started_at,
        } = phase1_owned;

        let dialog_turn_id = self
            .session_manager
            .start_dialog_turn_with_existing_context(
                &session_id,
                agent_type.clone(),
                user_input_text.clone(),
                Some(dialog_turn_id),
                None,
            )
            .await?;
        debug!("Generated unique dialog_turn_id for subagent: {}", dialog_turn_id);

        self.execution_engine
            .register_cancel_token(&dialog_turn_id, subagent_cancel_token.clone());
        debug!(
            "Registered cancel token to RoundExecutor: dialog_turn_id={}",
            dialog_turn_id
        );

        let _cleanup_guard = CancelTokenGuard {
            execution_engine: self.execution_engine.clone(),
            dialog_turn_id: dialog_turn_id.clone(),
        };

        self.session_manager
            .update_session_state_for_turn_if_processing(
                &session_id,
                &dialog_turn_id,
                SessionState::Processing {
                    current_turn_id: dialog_turn_id.clone(),
                    phase: ProcessingPhase::Thinking,
                },
            )
            .await?;

        self.emit_event(AgenticEvent::DialogTurnStarted {
            session_id: session_id.clone(),
            turn_id: dialog_turn_id.clone(),
            turn_index,
            user_input: user_input_text.clone(),
            original_user_input: None,
            user_message_metadata: None,
        })
        .await;

        let subagent_workspace_path = subagent_workspace
            .as_ref()
            .map(|workspace| workspace.root_path_string());
        let subagent_session_storage_path = subagent_workspace
            .as_ref()
            .map(|workspace| workspace.session_storage_path().to_path_buf());
        let subagent_services = Self::build_workspace_services(&subagent_workspace).await;
        let execution_context = ExecutionContext {
            session_id: session_id.clone(),
            dialog_turn_id: dialog_turn_id.clone(),
            turn_index,
            agent_type: agent_type.clone(),
            workspace: subagent_workspace,
            context,
            subagent_parent_info: subagent_parent_info.clone(),
            delegation_policy,
            skip_tool_confirmation: true,
            runtime_tool_restrictions,
            workspace_services: subagent_services,
            round_injection: None,
            recover_partial_on_cancel: true,
        };

        let execution_engine = self.execution_engine.clone();
        let agent_type_for_execution = agent_type.clone();
        debug!(
            "Subagent execution task starting: agent_type={}, session_id={}, dialog_turn_id={}, parent_session_id={}, parent_dialog_turn_id={}, parent_tool_call_id={}, timeout_seconds={:?}",
            agent_type,
            session_id,
            dialog_turn_id,
            parent_session_id,
            parent_dialog_turn_id,
            parent_tool_call_id,
            timeout_seconds,
        );
        let mut execution_task = tokio::spawn(async move {
            execution_engine
                .execute_dialog_turn(agent_type_for_execution, initial_messages, execution_context)
                .await
        });
        let abort_handle = execution_task.abort_handle();

        if subagent_parent_info.is_some() {
            self.active_subagent_executions.insert(
                session_id.clone(),
                ActiveSubagentExecution {
                    parent_session_id: parent_session_id.to_string(),
                    parent_dialog_turn_id: parent_dialog_turn_id.to_string(),
                    subagent_session_id: session_id.clone(),
                    subagent_dialog_turn_id: dialog_turn_id.clone(),
                    cancel_token: subagent_cancel_token.clone(),
                    abort_handle: abort_handle.clone(),
                },
            );
        }

        let mut execution_scope = SubagentExecutionScope {
            execution_engine: self.execution_engine.clone(),
            tool_pipeline: self.tool_pipeline.clone(),
            session_manager: self.session_manager.clone(),
            active_subagent_executions: self.active_subagent_executions.clone(),
            subagent_session_id: session_id.clone(),
            subagent_dialog_turn_id: dialog_turn_id.clone(),
            subagent_cancel_token: subagent_cancel_token.clone(),
            abort_handle,
            disarmed: false,
        };

        let execution_outcome = loop {
            let current_deadline = *deadline_rx.borrow_and_update();
            match current_deadline {
                Some(expires_at) if Instant::now() >= expires_at => {
                    break SubagentExecutionOutcome::TimedOut;
                }
                Some(expires_at) => {
                    let sleep = tokio::time::sleep_until(expires_at);
                    tokio::pin!(sleep);
                    tokio::select! {
                        join_result = &mut execution_task => {
                            break SubagentExecutionOutcome::Completed(join_result);
                        }
                        _ = subagent_cancel_token.cancelled() => {
                            break SubagentExecutionOutcome::Cancelled;
                        }
                        _ = &mut sleep => {
                            continue;
                        }
                        _ = deadline_rx.changed() => {
                            continue;
                        }
                    }
                }
                None => {
                    tokio::select! {
                        join_result = &mut execution_task => {
                            break SubagentExecutionOutcome::Completed(join_result);
                        }
                        _ = subagent_cancel_token.cancelled() => {
                            break SubagentExecutionOutcome::Cancelled;
                        }
                        _ = deadline_rx.changed() => {
                            continue;
                        }
                    }
                }
            }
        };

        let execution_outcome_label = match &execution_outcome {
            SubagentExecutionOutcome::Completed(_) => "completed",
            SubagentExecutionOutcome::Cancelled => "cancelled",
            SubagentExecutionOutcome::TimedOut => "timed_out",
        };
        debug!(
            "Subagent execution outcome resolved: agent_type={}, session_id={}, dialog_turn_id={}, parent_session_id={}, parent_dialog_turn_id={}, parent_tool_call_id={}, outcome={}, duration_ms={}",
            agent_type,
            session_id,
            dialog_turn_id,
            parent_session_id,
            parent_dialog_turn_id,
            parent_tool_call_id,
            execution_outcome_label,
            subagent_started_at.elapsed().as_millis()
        );

        match execution_outcome {
            SubagentExecutionOutcome::Completed(join_result) => {
                let result = match join_result {
                    Ok(result) => result,
                    Err(error) => {
                        let join_error =
                            NortHingError::tool(format!("Subagent '{}' failed to join: {}", agent_type, error));
                        Self::persist_failed_dialog_turn(
                            self.event_queue.as_ref(),
                            self.session_manager.as_ref(),
                            None,
                            &session_id,
                            &dialog_turn_id,
                            &join_error,
                        )
                        .await;
                        Self::finalize_persisted_turn_in_workspace_if_needed(
                            self.session_manager.as_ref(),
                            &session_id,
                            &dialog_turn_id,
                            turn_index,
                            &agent_type,
                            &user_input_text,
                            subagent_workspace_path.as_deref(),
                            subagent_session_storage_path.as_deref(),
                            Some(crate::service::session::TurnStatus::Error),
                            None,
                        )
                        .await;
                        error!(
                            "Subagent execution failed to join: agent_type={}, session={}, error={}",
                            agent_type, session_id, error
                        );

                        if let Err(cleanup_err) = self.cleanup_subagent_resources(&session_id).await {
                            warn!(
                                "Failed to cleanup subagent resources after join failure: session={}, error={}",
                                session_id, cleanup_err
                            );
                        }
                        let mut registry = self.subagent_timeout_registry.write().await;
                        registry.remove(&session_id);

                        execution_scope.disarm();
                        return Err(join_error);
                    }
                };

                // Return SubagentPhase2Output for successful execution (to be finalised by phase3)
                Ok(SubagentPhase2Output {
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
                    subagent_parent_info,
                    subagent_cancel_token,
                    execution_task,
                    execution_scope,
                    subagent_started_at,
                })
            }
            SubagentExecutionOutcome::Cancelled => {
                // Return error directly — phase3 not called for cancellation
                Err(NortHingError::Cancelled("Subagent task has been cancelled".to_string()))
            }
            SubagentExecutionOutcome::TimedOut => {
                // Return error directly — phase3 not called for timeout
                Err(NortHingError::Timeout(timeout_error_message))
            }
        }
    }
}
