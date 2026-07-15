//! Sub-domain: thread_goal.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains 12 thin wrappers around the `*_impl` thread_goal helpers living
//! in the `thread_goal` sibling.

use super::super::coordinator::*;
use super::super::scheduler::*;

use crate::util::errors::{NortHingError, NortHingResult};
use northhing_runtime_ports::{ThreadGoal, ThreadGoalContinuationPlan, ThreadGoalStatus};
use std::path::Path;
use tracing::{debug, info, warn};

impl ConversationCoordinator {
    pub async fn get_thread_goal(&self, session_id: &str, workspace_path: &Path) -> NortHingResult<Option<ThreadGoal>> {
        self.thread_goal_store()
            .get_thread_goal(session_id, workspace_path)
            .await
    }

    pub async fn clear_thread_goal(&self, session_id: &str, workspace_path: &Path) -> NortHingResult<()> {
        self.thread_goal_runtime.clear_active_goal(None);
        self.thread_goal_store()
            .clear_thread_goal(session_id, workspace_path)
            .await?;
        self.emit_thread_goal_updated(session_id, None).await;
        Ok(())
    }

    pub async fn create_thread_goal(
        &self,
        session_id: &str,
        workspace_path: &Path,
        objective: String,
        token_budget: Option<i64>,
    ) -> NortHingResult<ThreadGoal> {
        self.require_main_session_workspace(session_id)?;
        let goal = self
            .thread_goal_store()
            .create_thread_goal(session_id, workspace_path, objective, token_budget)
            .await?;
        self.thread_goal_runtime.mark_turn_started("", Some(&goal));
        self.emit_thread_goal_updated(session_id, Some(goal.clone())).await;
        Ok(goal)
    }

    pub async fn update_thread_goal_objective(
        &self,
        session_id: &str,
        workspace_path: &Path,
        objective: String,
    ) -> NortHingResult<ThreadGoal> {
        self.update_thread_goal_objective_impl(session_id, workspace_path, objective)
            .await
    }

    pub async fn set_thread_goal_objective(
        &self,
        session_id: &str,
        workspace_path: &Path,
        objective: String,
        replace_existing: bool,
    ) -> NortHingResult<ThreadGoal> {
        self.set_thread_goal_objective_impl(session_id, workspace_path, objective, replace_existing)
            .await
    }

    pub async fn maybe_mark_thread_goal_usage_limited(&self, session_id: &str, error: &NortHingError) {
        self.maybe_mark_thread_goal_usage_limited_impl(session_id, error).await
    }

    pub async fn set_thread_goal_status(
        &self,
        session_id: &str,
        workspace_path: &Path,
        status: ThreadGoalStatus,
    ) -> NortHingResult<ThreadGoal> {
        self.set_thread_goal_status_impl(session_id, workspace_path, status)
            .await
    }

    /// Pause an active thread goal after the user manually stops a turn so the UI can offer resume.
    pub async fn pause_thread_goal_after_user_cancel(&self, session_id: &str) {
        let workspace_path = match self.require_main_session_workspace(session_id) {
            Ok(path) => path,
            Err(error) => {
                debug!(
                    "Skipping thread goal pause after cancel (no workspace): session_id={}, error={}",
                    session_id, error
                );
                return;
            }
        };
        let Ok(Some(goal)) = self.get_thread_goal(session_id, workspace_path.as_path()).await else {
            return;
        };
        if !goal.is_active() {
            return;
        }
        if let Err(error) = self
            .set_thread_goal_status(session_id, workspace_path.as_path(), ThreadGoalStatus::Paused)
            .await
        {
            warn!(
                "Failed to pause thread goal after user cancel: session_id={}, error={}",
                session_id, error
            );
        } else {
            info!(
                "Thread goal paused after user cancel: session_id={}, objective={}",
                session_id, goal.objective
            );
        }
    }

    pub async fn update_thread_goal_status(
        &self,
        session_id: &str,
        workspace_path: &Path,
        status: ThreadGoalStatus,
        turn_id: Option<&str>,
    ) -> NortHingResult<ThreadGoal> {
        self.update_thread_goal_status_impl(session_id, workspace_path, status, turn_id)
            .await
    }

    pub async fn emit_thread_goal_updated(&self, session_id: &str, goal: Option<ThreadGoal>) {
        self.emit_thread_goal_updated_impl(session_id, goal).await
    }

    pub async fn activate_session_goal(
        &self,
        session_id: String,
        user_hint: Option<String>,
    ) -> NortHingResult<ThreadGoal> {
        self.activate_session_goal_impl(session_id, user_hint).await
    }

    /// Continue an active thread goal after a dialog turn completes (Codex-style).
    pub async fn prepare_goal_continuation_after_turn(
        &self,
        session_id: &str,
        source_turn_id: &str,
        user_input: &str,
        user_message_metadata: Option<&serde_json::Value>,
        turn_completed: bool,
    ) -> NortHingResult<Option<ThreadGoalContinuationPlan>> {
        self.prepare_goal_continuation_after_turn_impl(
            session_id,
            source_turn_id,
            user_input,
            user_message_metadata,
            turn_completed,
        )
        .await
    }
}
