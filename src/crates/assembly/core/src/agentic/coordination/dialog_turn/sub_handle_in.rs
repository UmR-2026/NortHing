//! Sub-domain: turn_subhandlers input/preparation phase (R47b refactor).
//!
//! Owns `prepare_turn` — the first of the 4 sub-handler phases. Resolves the
//! session (with on-demand restore), the effective agent type, validates the
//! session state machine, and triggers history restore when the context is
//! cold. Populates the initial half of `TurnContext`.
//!
//! Spec §2.1 R47b — extracted from `turn_subhandlers.rs` god-file.
//! Sibling imports `use super::super::coordinator::*` for the struct and
//! `use super::super::scheduler::DialogSubmissionPolicy` for the policy type.

use super::super::coordinator::*;
use super::super::scheduler::*;

use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use super::sub_handle_types::TurnContext;

use crate::agentic::core::SessionState;
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};

impl ConversationCoordinator {
    pub(super) async fn prepare_turn(&self, ctx: &mut TurnContext) -> NortHingResult<()> {
        let session_id = ctx.session_id.clone();
        let workspace_path = ctx.workspace_path.clone();
        let agent_type = ctx.agent_type.clone();
        let turn_id = ctx.turn_id.clone();
        let submission_policy = ctx.submission_policy.clone();
        let suppress_session_title_generation = ctx.suppress_session_title_generation;
        let session = match self.session_manager.get_session(&session_id) {
            Some(session) => session,
            None => {
                debug!(
                    "Session not found in memory, attempting restore before starting dialog: session_id={}",
                    session_id
                );
                let workspace_path = workspace_path.clone().ok_or_else(|| {
                    NortHingError::Validation(format!(
                        "workspace_path is required when restoring session: {}",
                        session_id
                    ))
                })?;
                self.session_manager
                    .restore_session(Path::new(&workspace_path), &session_id)
                    .await?
            }
        };
        let previous_agent_type = session.last_user_dialog_agent_type.clone();
        let requested_agent_type = agent_type.trim().to_string();
        let provisional_agent_type = if !requested_agent_type.is_empty() {
            requested_agent_type.clone()
        } else if !session.agent_type.is_empty() {
            session.agent_type.clone()
        } else {
            "agentic".to_string()
        };
        let effective_agent_type = Self::normalize_agent_type(&provisional_agent_type);
        Self::track_session_workspace_activity_best_effort(&session.config, "dialog_started").await;
        debug!(
            "Resolved dialog turn agent type: session_id={}, turn_id={}, requested_agent_type={}, session_agent_type={}, effective_agent_type={}, trigger_source={:?}, queue_priority={:?}, skip_tool_confirmation={}",
            session_id,
            turn_id.as_deref().unwrap_or(""),
            if requested_agent_type.is_empty() {
                "<empty>"
            } else {
                requested_agent_type.as_str()
            },
            if session.agent_type.is_empty() {
                "<empty>"
            } else {
                session.agent_type.as_str()
            },
            effective_agent_type,
            submission_policy.trigger_source,
            submission_policy.queue_priority,
            submission_policy.skip_tool_confirmation
        );
        if session.agent_type != effective_agent_type {
            self.session_manager
                .update_session_agent_type(&session_id, &effective_agent_type)
                .await?;
        }
        debug!(
            "Checking session state: session_id={}, state={:?}",
            session_id, session.state
        );
        let pending = self.wait_session_drained(&session_id, Duration::from_millis(800)).await;
        if pending > 0 {
            warn!(
                "Starting new dialog while previous turn still draining: session_id={}, pending={}",
                session_id, pending
            );
        }
        match &session.state {
            SessionState::Idle => {
                debug!("Session state is Idle, allowing new dialog: session_id={}", session_id);
            }
            SessionState::Error { .. } => {
                debug!(
                    "Session in error state, allowing new dialog (user retry): session_id={}",
                    session_id
                );
            }
            SessionState::Processing { current_turn_id, phase } => {
                warn!(
                    "Session still processing, rejecting new dialog: session_id={}, current_turn_id={}, phase={:?}",
                    session_id, current_turn_id, phase
                );
                return Err(NortHingError::Validation(format!(
                    "Session state does not allow starting new dialog: {:?}",
                    session.state
                )));
            }
        }
        let context_messages = self.session_manager.get_context_messages(&session_id).await?;
        let needs_restore = if context_messages.is_empty() {
            debug!("Session {} context is empty, restoring from persistence", session_id);
            true
        } else if context_messages.len() == 1 && !session.dialog_turn_ids.is_empty() {
            debug!(
                "Session {} has {} turns but only {} messages, restoring history",
                session_id,
                session.dialog_turn_ids.len(),
                context_messages.len()
            );
            true
        } else {
            debug!(
                "Session {} context exists ({} messages, {} turns), no restore needed",
                session_id,
                context_messages.len(),
                session.dialog_turn_ids.len()
            );
            false
        };
        if needs_restore {
            debug!("Starting session history restore: session_id={}", session_id);
            match self
                .session_manager
                .restore_session(
                    Path::new(
                        session
                            .config
                            .workspace_path
                            .as_deref()
                            .or(workspace_path.as_deref())
                            .ok_or_else(|| {
                                NortHingError::Validation(format!(
                                    "workspace_path is required when restoring session: {}",
                                    session_id
                                ))
                            })?,
                    ),
                    &session_id,
                )
                .await
            {
                Ok(_) => {
                    let restored_messages = self.session_manager.get_context_messages(&session_id).await?;
                    info!(
                        "Session history restored from persistence: session_id={}, messages: {} -> {}",
                        session_id,
                        context_messages.len(),
                        restored_messages.len()
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to restore session history (may be new session): session_id={}, error={}",
                        session_id, e
                    );
                }
            }
        }
        ctx.session = Some(session);
        ctx.effective_agent_type = effective_agent_type;
        ctx.previous_agent_type = previous_agent_type;
        Ok(())
    }
}
