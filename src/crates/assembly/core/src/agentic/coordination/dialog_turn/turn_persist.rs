//! Sub-domain: turn persistence helpers.
//! Extracted from turn.rs (R34 refactor).
//! Contains persist/finalize methods for completed, cancelled, and failed dialog turns.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::turn_outcome::TurnOutcome;

use crate::agentic::core::{MessageContent, SessionState};
use crate::agentic::events::{AgenticEvent, EventPriority, EventQueue};
use crate::agentic::execution::ExecutionResult;
use crate::agentic::session::SessionManager;
use crate::util::errors::NortHingError;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

impl ConversationCoordinator {
    pub(crate) async fn persist_completed_dialog_turn(
        session_manager: &SessionManager,
        scheduler_notify_tx: Option<&mpsc::Sender<(String, TurnOutcome)>>,
        session_id: &str,
        turn_id: &str,
        execution_result: &ExecutionResult,
    ) -> (crate::service::session::TurnStatus, String) {
        let final_response = match &execution_result.final_message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Mixed { text, .. } => text.clone(),
            _ => String::new(),
        };

        info!(
            "Dialog turn completed: session={}, turn={}, rounds={}",
            session_id, turn_id, execution_result.total_rounds
        );

        if let Err(error) = session_manager
            .complete_dialog_turn(
                session_id,
                turn_id,
                final_response.clone(),
                crate::agentic::core::TurnStats {
                    total_rounds: execution_result.total_rounds,
                    total_tools: execution_result.total_tools,
                    total_tokens: 0,
                    duration_ms: execution_result.duration_ms,
                },
            )
            .await
        {
            error!(
                "Failed to complete dialog turn: session_id={}, turn_id={}, error={}",
                session_id, turn_id, error
            );
        }

        match session_manager
            .update_session_state_for_turn_if_processing(session_id, turn_id, SessionState::Idle)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                debug!(
                    "Skipped setting session Idle after completion for stale turn: session_id={}, turn_id={}",
                    session_id, turn_id
                );
            }
            Err(error) => {
                error!(
                    "Failed to set session state to Idle after completion: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, error
                );
            }
        }

        if let Some(tx) = scheduler_notify_tx {
            if let Err(error) = tx.try_send((
                session_id.to_string(),
                TurnOutcome::Completed {
                    turn_id: turn_id.to_string(),
                    final_response: final_response.clone(),
                },
            )) {
                error!(
                    "Failed to notify scheduler of turn completion: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, error
                );
            }
        }

        (crate::service::session::TurnStatus::Completed, final_response)
    }

    pub(super) async fn persist_cancelled_dialog_turn(
        event_queue: &EventQueue,
        session_manager: &SessionManager,
        scheduler_notify_tx: Option<&mpsc::Sender<(String, TurnOutcome)>>,
        session_id: &str,
        turn_id: &str,
    ) -> crate::service::session::TurnStatus {
        info!("Dialog turn cancelled: session={}, turn={}", session_id, turn_id);

        // The execution engine only emits DialogTurnCancelled when cancellation is
        // detected between rounds. If cancellation interrupted streaming mid-round,
        // no event was emitted. Emit it here unconditionally; duplicates are harmless.
        if let Err(error) = event_queue
            .enqueue(
                AgenticEvent::DialogTurnCancelled {
                    session_id: session_id.to_string(),
                    turn_id: turn_id.to_string(),
                },
                Some(EventPriority::Critical),
            )
            .await
        {
            error!(
                "Failed to emit DialogTurnCancelled event: session_id={}, turn_id={}, error={}",
                session_id, turn_id, error
            );
        }

        if let Err(error) = session_manager.cancel_dialog_turn(session_id, turn_id).await {
            error!(
                "Failed to cancel dialog turn in persistence: session_id={}, turn_id={}, error={}",
                session_id, turn_id, error
            );
        }

        match session_manager
            .update_session_state_for_turn_if_processing(session_id, turn_id, SessionState::Idle)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                debug!(
                    "Skipped setting session Idle after cancellation for stale turn: session_id={}, turn_id={}",
                    session_id, turn_id
                );
            }
            Err(error) => {
                error!(
                    "Failed to set session state to Idle after cancellation: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, error
                );
            }
        }

        if let Some(tx) = scheduler_notify_tx {
            if let Err(error) = tx.try_send((
                session_id.to_string(),
                TurnOutcome::Cancelled {
                    turn_id: turn_id.to_string(),
                },
            )) {
                error!(
                    "Failed to notify scheduler of turn cancellation: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, error
                );
            }
        }

        crate::service::session::TurnStatus::Cancelled
    }

    pub(crate) async fn persist_failed_dialog_turn(
        event_queue: &EventQueue,
        session_manager: &SessionManager,
        scheduler_notify_tx: Option<&mpsc::Sender<(String, TurnOutcome)>>,
        session_id: &str,
        turn_id: &str,
        error: &NortHingError,
    ) -> crate::service::session::TurnStatus {
        let error_text = error.to_string();
        let recoverable = !matches!(error, NortHingError::AIClient(_) | NortHingError::Timeout(_));

        error!("Dialog turn execution failed: {}", error_text);

        if let Err(queue_error) = event_queue
            .enqueue(
                AgenticEvent::DialogTurnFailed {
                    session_id: session_id.to_string(),
                    turn_id: turn_id.to_string(),
                    error: error_text.clone(),
                    error_category: Some(error.error_category()),
                    error_detail: Some(error.error_detail()),
                },
                Some(EventPriority::Critical),
            )
            .await
        {
            error!(
                "Failed to emit DialogTurnFailed event: session_id={}, turn_id={}, error={}",
                session_id, turn_id, queue_error
            );
        }

        if let Err(persist_error) = session_manager
            .fail_dialog_turn(session_id, turn_id, error_text.clone())
            .await
        {
            error!(
                "Failed to mark dialog turn as failed: session_id={}, turn_id={}, error={}",
                session_id, turn_id, persist_error
            );
        }

        match session_manager
            .update_session_state_for_turn_if_processing(
                session_id,
                turn_id,
                SessionState::Error {
                    error: error_text.clone(),
                    recoverable,
                },
            )
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                debug!(
                    "Skipped setting session Error after failure for stale turn: session_id={}, turn_id={}",
                    session_id, turn_id
                );
            }
            Err(state_error) => {
                error!(
                    "Failed to set session state to Error: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, state_error
                );
            }
        }

        if let Some(tx) = scheduler_notify_tx {
            if let Err(notify_error) = tx.try_send((
                session_id.to_string(),
                TurnOutcome::Failed {
                    turn_id: turn_id.to_string(),
                    error: error_text.clone(),
                },
            )) {
                error!(
                    "Failed to notify scheduler of turn failure: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, notify_error
                );
            }
        }

        if let Some(coordinator) = global_coordinator() {
            coordinator
                .maybe_mark_thread_goal_usage_limited(session_id, error)
                .await;
        }

        crate::service::session::TurnStatus::Error
    }

    pub(crate) async fn finalize_persisted_turn_in_workspace_if_needed(
        session_manager: &SessionManager,
        session_id: &str,
        turn_id: &str,
        turn_index: usize,
        agent_type: &str,
        user_input: &str,
        workspace_path: Option<&str>,
        resolved_session_storage_path: Option<&std::path::Path>,
        status: Option<crate::service::session::TurnStatus>,
        user_message_metadata: Option<serde_json::Value>,
    ) {
        if !session_manager.should_persist_session_id(session_id) {
            return;
        }

        if let (Some(workspace_path), Some(status)) = (workspace_path, status) {
            Self::finalize_turn_in_workspace(
                session_id,
                turn_id,
                turn_index,
                agent_type,
                user_input,
                workspace_path,
                resolved_session_storage_path,
                status,
                user_message_metadata,
            )
            .await;
        }
    }
}
