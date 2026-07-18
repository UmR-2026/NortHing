//! Sub-domain: turn cancel/delete helpers.
//! Extracted from turn.rs (R34 refactor).
//! Contains cancel, delete, and subagent execution control methods.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use crate::agentic::core::{Message, SessionState};
use crate::agentic::events::AgenticEvent;
use crate::agentic::execution::ExecutionContext;
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::session::SessionManager;
use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
use crate::util::errors::{NortHingError, NortHingResult};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

use super::turn_subhandlers::TurnContext;

impl ConversationCoordinator {
    /// Wrapper for `start_dialog_turn_internal`. Splits the original 701-line
    /// god-method into 4 sub-handlers (`prepare_turn` -> `dispatch_turn` ->
    /// `finalize_turn` -> `cleanup_turn`) that share state via `TurnContext`.
    /// Public facade signature preserved.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_dialog_turn_internal(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        image_contexts: Option<Vec<ImageContextData>>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        extra_user_message_metadata: Option<serde_json::Value>,
        additional_prepended_messages: Vec<Message>,
        suppress_session_title_generation: bool,
    ) -> NortHingResult<()> {
        let mut ctx = TurnContext::new(
            session_id,
            user_input,
            original_user_input,
            image_contexts,
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            extra_user_message_metadata,
            additional_prepended_messages,
            suppress_session_title_generation,
        );
        self.prepare_turn(&mut ctx).await?;
        self.dispatch_turn(&mut ctx).await?;
        self.finalize_turn(&mut ctx).await?;
        self.cleanup_turn(&mut ctx).await?;
        Ok(())
    }

    pub(super) async fn cancel_active_subagents_for_parent_turn(
        &self,
        parent_session_id: &str,
        parent_dialog_turn_id: &str,
    ) {
        let active_subagents: Vec<ActiveSubagentExecution> = self
            .active_subagent_executions
            .iter()
            .filter(|entry| {
                entry.parent_session_id == parent_session_id && entry.parent_dialog_turn_id == parent_dialog_turn_id
            })
            .map(|entry| entry.value().clone())
            .collect();

        if active_subagents.is_empty() {
            return;
        }

        info!(
            "Cancelling {} active subagent execution(s) for parent turn: parent_session_id={}, parent_dialog_turn_id={}",
            active_subagents.len(),
            parent_session_id,
            parent_dialog_turn_id
        );

        for active in active_subagents {
            self.stop_active_subagent_execution(&active, "Parent dialog turn cancelled")
                .await;
        }
    }

    pub(super) async fn stop_active_subagent_execution(&self, active: &ActiveSubagentExecution, reason: &str) {
        debug!(
            "Stopping active subagent execution: subagent_session_id={}, subagent_dialog_turn_id={}, parent_session_id={}, parent_dialog_turn_id={}, reason={}",
            active.subagent_session_id,
            active.subagent_dialog_turn_id,
            active.parent_session_id,
            active.parent_dialog_turn_id,
            reason
        );

        active.cancel_token.cancel();
        active.abort_handle.abort();

        if let Err(error) = self
            .execution_engine
            .cancel_dialog_turn(&active.subagent_dialog_turn_id)
            .await
        {
            warn!(
                "Failed to cancel active subagent dialog turn: subagent_session_id={}, subagent_dialog_turn_id={}, error={}",
                active.subagent_session_id, active.subagent_dialog_turn_id, error
            );
        }

        if let Err(error) = self
            .tool_pipeline
            .cancel_dialog_turn_tools(&active.subagent_dialog_turn_id)
            .await
        {
            warn!(
                "Failed to cancel active subagent tools: subagent_session_id={}, subagent_dialog_turn_id={}, error={}",
                active.subagent_session_id, active.subagent_dialog_turn_id, error
            );
        }

        Self::persist_cancelled_dialog_turn(
            self.event_queue.as_ref(),
            self.session_manager.as_ref(),
            None,
            &active.subagent_session_id,
            &active.subagent_dialog_turn_id,
        )
        .await;

        self.session_manager
            .reset_session_state_if_processing(&active.subagent_session_id, &active.subagent_dialog_turn_id);

        self.active_subagent_executions.remove(&active.subagent_session_id);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn cancel_dialog_turn_impl(&self, session_id: &str, dialog_turn_id: &str) -> NortHingResult<()> {
        info!(
            "Received cancel request: dialog_turn_id={}, session_id={}",
            dialog_turn_id, session_id
        );

        abort_thread_goal_continuation_for_session(session_id);

        let old_state = self
            .session_manager
            .get_session(session_id)
            .map(|s| format!("{:?}", s.state))
            .unwrap_or_else(|| "Unknown".to_string());
        debug!("Current state: {}", old_state);

        // Step 1: Immediately update session state to Idle only if this
        // cancellation still targets the currently processing turn. A delayed
        // cancel request for an older turn must not clear a newer turn.
        debug!("Conditionally updating session state to Idle for cancelled turn");
        let state_updated = self
            .session_manager
            .update_session_state_for_turn_if_processing(session_id, dialog_turn_id, SessionState::Idle)
            .await?;

        let new_state = self
            .session_manager
            .get_session(session_id)
            .map(|s| format!("{:?}", s.state))
            .unwrap_or_else(|| "Unknown".to_string());
        debug!("State updated: {} -> {}", old_state, new_state);

        // Step 2: Immediately send state change event only when this cancel
        // actually changed the active turn state.
        if state_updated {
            self.emit_event(AgenticEvent::SessionStateChanged {
                session_id: session_id.to_string(),
                new_state: "idle".to_string(),
            })
            .await;
            debug!("Session state change event sent");
            self.pause_thread_goal_after_user_cancel(session_id).await;
        } else {
            debug!(
                "Skipped idle event for stale cancellation: session_id={}, dialog_turn_id={}",
                session_id, dialog_turn_id
            );
        }

        // Step 3: Trigger cancellation tokens so the running turn unwinds. We
        // do this synchronously (not spawn) because the calls themselves are
        // cheap (just signalling tokens); the actual long-running work
        // (waiting for the spawn task to drain) is handled via
        // `wait_session_drained` below.
        if let Err(e) = self.execution_engine.cancel_dialog_turn(dialog_turn_id).await {
            warn!("Failed to cancel execution engine: {}", e);
        }
        if let Err(e) = self.tool_pipeline.cancel_dialog_turn_tools(dialog_turn_id).await {
            warn!("Failed to cancel tool execution: {}", e);
        }

        self.cancel_active_subagents_for_parent_turn(session_id, dialog_turn_id)
            .await;

        // Step 4: Wait briefly for the spawn task that owns this turn to drain
        // its in-memory message writes before returning. Capped so the RPC
        // never blocks longer than ~1.5s — beyond that we let the new turn
        // proceed and rely on the cancellation token already being signalled.
        let pending = self.wait_session_drained(session_id, Duration::from_millis(1500)).await;
        if pending > 0 {
            warn!(
                "Cancelled turn did not fully drain within 1500ms: session_id={}, dialog_turn_id={}, pending={}",
                session_id, dialog_turn_id, pending
            );
            // 2026-07-18 (W3a-3): Convergence fallback. The turn task is still
            // alive after the drain window (e.g. stuck in an uninterruptible
            // await such as AskUserQuestion). The spawned task will not emit
            // DialogTurnCancelled on its own, so the desktop UI would stay in
            // streaming state forever. Emit the terminal event and persist the
            // terminal state here unconditionally so the UI converges to Idle.
            // Duplicates are harmless (see persist_cancelled_dialog_turn).
            if state_updated {
                Self::persist_cancelled_dialog_turn(
                    self.event_queue.as_ref(),
                    self.session_manager.as_ref(),
                    self.scheduler_notify_tx.get(),
                    session_id,
                    dialog_turn_id,
                )
                .await;
            }
        } else {
            debug!(
                "Cancelled turn fully drained: session_id={}, dialog_turn_id={}",
                session_id, dialog_turn_id
            );
        }

        Ok(())
    }

    pub(super) async fn cancel_active_turn_for_session_impl(
        &self,
        session_id: &str,
        wait_timeout: Duration,
    ) -> NortHingResult<Option<String>> {
        abort_thread_goal_continuation_for_session(session_id);

        let Some(session) = self.session_manager.get_session(session_id) else {
            return Ok(None);
        };

        let SessionState::Processing { current_turn_id, .. } = session.state else {
            return Ok(None);
        };

        self.cancel_dialog_turn(session_id, &current_turn_id).await?;

        let deadline = Instant::now() + wait_timeout;
        while self.execution_engine.has_active_turn(&current_turn_id) {
            if Instant::now() >= deadline {
                warn!(
                    "Timed out waiting for active turn cancellation: session_id={}, dialog_turn_id={}, timeout_ms={}",
                    session_id,
                    current_turn_id,
                    wait_timeout.as_millis()
                );
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }

        Ok(Some(current_turn_id))
    }

    /// Delete session
    pub(super) async fn delete_session_impl(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        self.session_manager.delete_session(workspace_path, session_id).await?;
        self.emit_event(AgenticEvent::SessionDeleted {
            session_id: session_id.to_string(),
        })
        .await;
        Ok(())
    }

    pub(super) async fn delete_hidden_subagent_sessions_for_parent_turns_impl(
        &self,
        workspace_path: &Path,
        parent_session_id: &str,
        parent_dialog_turn_ids: &HashSet<String>,
    ) -> NortHingResult<Vec<String>> {
        let session_ids = self
            .session_manager
            .collect_hidden_subagent_cascade_for_parent_turns(workspace_path, parent_session_id, parent_dialog_turn_ids)
            .await?;

        let mut deleted_session_ids = Vec::new();

        for session_id in session_ids {
            if let Err(e) = self
                .cancel_active_turn_for_session(&session_id, Duration::from_secs(2))
                .await
            {
                warn!(
                    "Failed to cancel hidden subagent session before deletion: \
                     session_id={}, parent_session_id={}, error={}",
                    session_id, parent_session_id, e
                );
            }

            self.delete_session(workspace_path, &session_id).await?;
            deleted_session_ids.push(session_id);
        }

        Ok(deleted_session_ids)
    }
}
