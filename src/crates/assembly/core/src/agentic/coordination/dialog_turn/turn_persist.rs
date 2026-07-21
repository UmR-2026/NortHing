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
use tracing::{debug, error, info, warn};

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
            // Backpressure is correct here: the scheduler must drain turn
            // outcomes, so await the send instead of dropping on a full buffer.
            if let Err(error) = tx
                .send((
                    session_id.to_string(),
                    TurnOutcome::Completed {
                        turn_id: turn_id.to_string(),
                        final_response: final_response.clone(),
                    },
                ))
                .await
            {
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
            // Backpressure is correct here: the scheduler must drain turn
            // outcomes, so await the send instead of dropping on a full buffer.
            if let Err(error) = tx
                .send((
                    session_id.to_string(),
                    TurnOutcome::Cancelled {
                        turn_id: turn_id.to_string(),
                    },
                ))
                .await
            {
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
            // Backpressure is correct here: the scheduler must drain turn
            // outcomes, so await the send instead of dropping on a full buffer.
            if let Err(notify_error) = tx
                .send((
                    session_id.to_string(),
                    TurnOutcome::Failed {
                        turn_id: turn_id.to_string(),
                        error: error_text.clone(),
                    },
                ))
                .await
            {
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

        // Check both options are Some before extracting to avoid move issues
        let (Some(wp), Some(st)) = (workspace_path, status) else {
            return;
        };

        // Clone status since we need to pass it to both finalize and episode logging
        let st_for_episode = st.clone();

        Self::finalize_turn_in_workspace(
            session_id,
            turn_id,
            turn_index,
            agent_type,
            user_input,
            wp,
            resolved_session_storage_path,
            st,
            user_message_metadata,
        )
        .await;

        // Hook: distill episode and append to growth log.
        // Must not fail the finalize flow - warn on any error.
        Self::append_episode_log_entry(
            session_id,
            turn_id,
            turn_index,
            agent_type,
            user_input,
            &wp,
            resolved_session_storage_path,
            st_for_episode,
        )
        .await;

        // Hook: distill facts from user input and append to facts store.
        // Must not fail the finalize flow - warn on any error.
        Self::append_facts_entry(
            session_id,
            turn_id,
            &wp,
            resolved_session_storage_path,
            user_input,
            agent_type,
        )
        .await;
    }

    /// Distill and append episode log entry after turn finalization.
    /// Failures are logged as warnings and do not propagate.
    async fn append_episode_log_entry(
        session_id: &str,
        turn_id: &str,
        turn_index: usize,
        agent_type: &str,
        user_input: &str,
        workspace_path: &str,
        resolved_session_storage_path: Option<&std::path::Path>,
        status: crate::service::session::TurnStatus,
    ) {
        use crate::agentic::episodes;
        use crate::agentic::episodes::types::EpisodeOutcome;
        use crate::agentic::persistence::PersistenceManager;
        use crate::infrastructure::PathManager;

        // Map TurnStatus to EpisodeOutcome
        let outcome = match status {
            crate::service::session::TurnStatus::Completed => EpisodeOutcome::Completed,
            crate::service::session::TurnStatus::Error => EpisodeOutcome::Failed,
            crate::service::session::TurnStatus::Cancelled => EpisodeOutcome::Cancelled,
            _ => return, // Skip other statuses (e.g., InProgress)
        };

        let path_manager = match PathManager::new() {
            Ok(pm) => std::sync::Arc::new(pm),
            Err(e) => {
                warn!("Episode log: failed to create PathManager: {}", e);
                return;
            }
        };

        let workspace_path_buf = match resolved_session_storage_path {
            Some(p) => p.to_path_buf(),
            None => std::path::PathBuf::from(workspace_path),
        };

        // Compute workspace slug BEFORE moving path_manager into PersistenceManager
        let slug = path_manager.workspace_slug(&workspace_path_buf);

        let persistence_manager = match PersistenceManager::new(path_manager) {
            Ok(manager) => manager,
            Err(e) => {
                warn!("Episode log: failed to create PersistenceManager: {}", e);
                return;
            }
        };

        // Read the persisted turn
        let turn = match persistence_manager
            .load_dialog_turn(&workspace_path_buf, session_id, turn_index)
            .await
        {
            Ok(Some(t)) => t,
            Ok(None) => {
                warn!(
                    "Episode log: turn not found for distillation: session_id={}, turn_id={}",
                    session_id, turn_id
                );
                return;
            }
            Err(e) => {
                warn!(
                    "Episode log: failed to load turn for distillation: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, e
                );
                return;
            }
        };

        // Task summary: first 120 chars of user_input
        let task_summary = user_input.chars().take(120).collect::<String>();

        // Distill and append
        let episode = episodes::distill_episode(&turn, task_summary, slug, agent_type.to_string(), outcome);

        if let Err(e) = episodes::append_episode(&episode).await {
            warn!(
                "Episode log: failed to append episode: session_id={}, turn_id={}, error={}",
                session_id, turn_id, e
            );
        }
    }

    /// Distill candidate facts from user input and append to facts store.
    /// Failures are logged as warnings and do not propagate.
    async fn append_facts_entry(
        session_id: &str,
        turn_id: &str,
        workspace_path: &str,
        resolved_session_storage_path: Option<&std::path::Path>,
        user_input: &str,
        _agent_type: &str,
    ) {
        use crate::infrastructure::PathManager;
        use crate::service::agent_memory::{append_facts, distill_facts_from_user_message, read_facts};

        // Distill candidate facts from user input using keyword triggers
        let candidates = distill_facts_from_user_message(user_input, session_id, turn_id);
        if candidates.is_empty() {
            return;
        }

        let path_manager = match PathManager::new() {
            Ok(pm) => std::sync::Arc::new(pm),
            Err(e) => {
                warn!("Facts: failed to create PathManager: {}", e);
                return;
            }
        };

        let workspace_path_buf = match resolved_session_storage_path {
            Some(p) => p.to_path_buf(),
            None => std::path::PathBuf::from(workspace_path),
        };

        // Get the memory directory path
        let memory_dir = path_manager.project_memory_dir(&workspace_path_buf);

        // Read existing facts for deduplication (exact text match)
        // If read fails, warn and return — do not continue with empty set (would cause duplicates)
        let existing_facts = match read_facts(&memory_dir).await {
            Ok(facts) => facts,
            Err(e) => {
                warn!(
                    "Facts: failed to read existing facts for deduplication, skipping append: session_id={}, turn_id={}, error={}",
                    session_id, turn_id, e
                );
                return;
            }
        };

        // Unified deduplication: history + batch (HashSet::insert returns false if already present)
        let mut seen: std::collections::HashSet<String> =
            existing_facts.iter().map(|f| f.text.clone()).collect();

        let new_facts: Vec<_> = candidates
            .into_iter()
            .filter(|c| seen.insert(c.text.clone()))
            .collect();

        if new_facts.is_empty() {
            debug!(
                "Facts: no new facts to append (all duplicates): session_id={}, turn_id={}",
                session_id, turn_id
            );
            return;
        }

        // Append new facts
        if let Err(e) = append_facts(&memory_dir, &new_facts).await {
            warn!(
                "Facts: failed to append facts: session_id={}, turn_id={}, error={}",
                session_id, turn_id, e
            );
        } else {
            debug!(
                "Facts: appended {} facts: session_id={}, turn_id={}",
                new_facts.len(),
                session_id,
                turn_id
            );
        }
    }
}
