//! Sub-domain: thread_goal.
//! Spec §2.1 — extracted from dialog_turn.rs (Round 6 refactor).
//! Contains private/pub(crate) helper methods; public API stays in the facade mod.rs.
//!
//! Sibling imports `use super::super::coordinator::*` for the struct definition.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::turn_outcome::TurnOutcome;

use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use crate::agentic::agents::agent_registry;
use crate::agentic::context_profile::ContextProfilePolicy;
use crate::agentic::core::{
    InternalReminderKind, Message, MessageContent, ProcessingPhase, Session, SessionConfig, SessionKind, SessionState,
    SessionSummary, TurnStats,
};
use crate::agentic::events::{
    AgenticEvent, DeepReviewQueueState, EventPriority, EventQueue, EventRouter, EventSubscriber,
};
use crate::agentic::execution::{ContextCompactionOutcome, ExecutionContext, ExecutionEngine, ExecutionResult};
use crate::agentic::fork_agent::ForkAgentContextSnapshot;
use crate::agentic::goal_mode::{
    effective_subagent_timeout_seconds, is_usage_limit_error, maybe_build_continuation_after_turn,
    should_skip_goal_continuation_after_turn, should_skip_goal_for_turn, thread_goal_status_is_resumable,
    user_facing_thread_goal_error, ThreadGoalRuntime, ThreadGoalStore,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::remote_file_delivery::{
    needs_computer_links_for_source, remote_file_delivery_reminder, TOOL_CONTEXT_REMOTE_FILE_DELIVERY_KEY,
};
use crate::agentic::round_preempt::DialogRoundInjectionSource;
use crate::agentic::session::SessionManager;
use crate::agentic::side_question::build_btw_user_input;
use crate::agentic::skill_agent_snapshot::{
    diff_skill_agent_snapshot, resolve_skill_agent_snapshot, TurnSkillAgentSnapshot,
};
use crate::agentic::tools::pipeline::{SubagentParentInfo, ToolPipeline};
use crate::agentic::tools::{
    is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions,
};
use crate::agentic::workspace::WorkspaceServices;
use crate::agentic::WorkspaceBinding;
use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};
use crate::service::config::global::GlobalConfigManager;
use crate::service::remote_ssh::normalize_remote_workspace_path;
use crate::service::session::{SessionRelationship, SessionRelationshipKind};
use crate::service::workspace::{global_workspace_service, WorkspaceCreateOptions, WorkspaceKind};
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::errors::{NortHingError, NortHingResult};
use dashmap::DashMap;
use northhing_runtime_ports::{
    AgentBackgroundResultRequest, AgentThreadGoalDeliveryKind, AgentThreadGoalDeliveryRequest, DelegationPolicy,
    SubagentContextMode, ThreadGoal, ThreadGoalContinuationPlan, ThreadGoalStatus,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

use northhing_agent_dispatch::{ActorRuntime, USE_LIGHTWEIGHT_ACTOR};
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

const MANUAL_COMPACTION_COMMAND: &str = "/compact";
const CONTEXT_COMPRESSION_TOOL_NAME: &str = "ContextCompression";
const DEFAULT_SUBAGENT_MAX_CONCURRENCY: usize = 5;
const MAX_SUBAGENT_MAX_CONCURRENCY: usize = 64;

impl ConversationCoordinator {
    pub(super) fn thread_goal_store(&self) -> ThreadGoalStore<'_> {
        ThreadGoalStore::new(self.session_manager.as_ref())
    }

    pub(super) async fn apply_objective_updated_steering(&self, session_id: &str, goal: &ThreadGoal) {
        if !goal.is_active() {
            return;
        }
        let agent_type = match self.session_manager.get_session(session_id) {
            Some(session) => {
                let agent_type = session.agent_type.trim();
                if agent_type.is_empty() {
                    "agentic".to_string()
                } else {
                    agent_type.to_string()
                }
            }
            None => "agentic".to_string(),
        };
        let workspace_path = self
            .require_main_session_workspace(session_id)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
        let runtime = match CoreServiceAgentRuntime::global_agent_runtime_with_lifecycle_delivery() {
            Ok(runtime) => runtime,
            Err(error) => {
                warn!(
                    "Agent runtime lifecycle delivery is not available; objective_updated steering skipped: session_id={}, error={}",
                    session_id, error
                );
                return;
            }
        };
        if let Err(error) = runtime
            .deliver_thread_goal(AgentThreadGoalDeliveryRequest {
                session_id: session_id.to_string(),
                agent_type,
                workspace_path,
                kind: AgentThreadGoalDeliveryKind::ObjectiveUpdated,
                goal: goal.clone(),
            })
            .await
        {
            warn!(
                "Failed to deliver objective_updated steering: session_id={}, error={}",
                session_id,
                CoreServiceAgentRuntime::runtime_error_message(error)
            );
        }
    }

    pub(super) fn schedule_thread_goal_resumed_steering(&self, session_id: &str, goal: &ThreadGoal) {
        if !goal.is_active() {
            return;
        }
        let agent_type = match self.session_manager.get_session(session_id) {
            Some(session) => {
                let agent_type = session.agent_type.trim();
                if agent_type.is_empty() {
                    "agentic".to_string()
                } else {
                    agent_type.to_string()
                }
            }
            None => "agentic".to_string(),
        };
        let workspace_path = self
            .require_main_session_workspace(session_id)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
        let session_id = session_id.to_string();
        let goal = goal.clone();
        tokio::spawn(async move {
            let runtime = match CoreServiceAgentRuntime::global_agent_runtime_with_lifecycle_delivery() {
                Ok(runtime) => runtime,
                Err(error) => {
                    warn!(
                            "Agent runtime lifecycle delivery is not available; thread goal resume steering skipped: session_id={}, error={}",
                            session_id, error
                        );
                    return;
                }
            };
            if let Err(error) = runtime
                .deliver_thread_goal(AgentThreadGoalDeliveryRequest {
                    session_id: session_id.clone(),
                    agent_type,
                    workspace_path,
                    kind: AgentThreadGoalDeliveryKind::Resumed,
                    goal,
                })
                .await
            {
                warn!(
                    "Failed to deliver thread goal resume steering: session_id={}, error={}",
                    session_id,
                    CoreServiceAgentRuntime::runtime_error_message(error)
                );
            }
        });
    }

    pub(crate) async fn load_active_thread_goal(&self, session_id: &str) -> NortHingResult<Option<ThreadGoal>> {
        let workspace_path = self.require_main_session_workspace(session_id)?;
        Ok(self
            .get_thread_goal(session_id, workspace_path.as_path())
            .await?
            .filter(ThreadGoal::is_active))
    }

    pub(super) async fn update_thread_goal_objective_impl(
        &self,
        session_id: &str,
        workspace_path: &Path,
        objective: String,
    ) -> NortHingResult<ThreadGoal> {
        self.require_main_session_workspace(session_id)?;
        let existing = self.get_thread_goal(session_id, workspace_path).await?.ok_or_else(|| {
            NortHingError::NotFound(format!("cannot edit goal for session {session_id}: no goal exists"))
        })?;
        let status = match existing.status {
            ThreadGoalStatus::BudgetLimited | ThreadGoalStatus::Complete => Some(ThreadGoalStatus::Active),
            _ => None,
        };
        let result = self
            .thread_goal_store()
            .set_thread_goal(session_id, workspace_path, Some(objective), status, None, false)
            .await?;
        let objective_changed = existing.objective != result.goal.objective;
        if result.goal.is_active() {
            self.thread_goal_runtime.mark_turn_started("", Some(&result.goal));
        }
        self.emit_thread_goal_updated_impl(session_id, Some(result.goal.clone()))
            .await;
        if objective_changed && result.goal.is_active() {
            self.apply_objective_updated_steering(session_id, &result.goal).await;
        }
        Ok(result.goal)
    }

    pub(super) async fn set_thread_goal_objective_impl(
        &self,
        session_id: &str,
        workspace_path: &Path,
        objective: String,
        replace_existing: bool,
    ) -> NortHingResult<ThreadGoal> {
        self.require_main_session_workspace(session_id)?;
        let previous = self.get_thread_goal(session_id, workspace_path).await?;
        let status = if previous.is_some() && !replace_existing {
            None
        } else {
            Some(ThreadGoalStatus::Active)
        };
        let result = self
            .thread_goal_store()
            .set_thread_goal(
                session_id,
                workspace_path,
                Some(objective),
                status,
                None,
                replace_existing,
            )
            .await?;
        let objective_changed = previous
            .as_ref()
            .map(|goal| goal.objective != result.goal.objective)
            .unwrap_or(true);
        if result.goal.is_active() {
            self.thread_goal_runtime.mark_turn_started("", Some(&result.goal));
        }
        self.emit_thread_goal_updated_impl(session_id, Some(result.goal.clone()))
            .await;
        if objective_changed && result.goal.is_active() {
            self.apply_objective_updated_steering(session_id, &result.goal).await;
        }
        Ok(result.goal)
    }

    pub(super) async fn maybe_mark_thread_goal_usage_limited_impl(&self, session_id: &str, error: &NortHingError) {
        if !is_usage_limit_error(error) {
            return;
        }
        let workspace_path = match self.require_main_session_workspace(session_id) {
            Ok(path) => path,
            Err(_) => return,
        };
        let Ok(Some(goal)) = self.get_thread_goal(session_id, workspace_path.as_path()).await else {
            return;
        };
        if !goal.is_active() {
            return;
        }
        if let Err(error) = self
            .set_thread_goal_status_impl(session_id, workspace_path.as_path(), ThreadGoalStatus::UsageLimited)
            .await
        {
            warn!(
                "Failed to mark thread goal usage limited: session_id={}, error={}",
                session_id, error
            );
        }
    }

    pub(super) async fn set_thread_goal_status_impl(
        &self,
        session_id: &str,
        workspace_path: &Path,
        status: ThreadGoalStatus,
    ) -> NortHingResult<ThreadGoal> {
        self.require_main_session_workspace(session_id)?;
        let previous = self.get_thread_goal(session_id, workspace_path).await?;
        let resuming = status == ThreadGoalStatus::Active
            && previous
                .as_ref()
                .is_some_and(|goal| thread_goal_status_is_resumable(goal.status));
        let result = self
            .thread_goal_store()
            .set_thread_goal(session_id, workspace_path, None, Some(status), None, false)
            .await?;
        if !result.goal.is_active() {
            self.thread_goal_runtime.clear_active_goal(None);
        } else if resuming {
            self.thread_goal_runtime.mark_turn_started("", Some(&result.goal));
        }
        self.emit_thread_goal_updated_impl(session_id, Some(result.goal.clone()))
            .await;
        if resuming && result.goal.is_active() {
            clear_thread_goal_continuation_abort(session_id);
            self.schedule_thread_goal_resumed_steering(session_id, &result.goal);
        }
        Ok(result.goal)
    }

    pub(super) async fn update_thread_goal_status_impl(
        &self,
        session_id: &str,
        workspace_path: &Path,
        status: ThreadGoalStatus,
        turn_id: Option<&str>,
    ) -> NortHingResult<ThreadGoal> {
        let goal = self
            .set_thread_goal_status_impl(session_id, workspace_path, status)
            .await?;
        self.thread_goal_runtime.clear_active_goal(turn_id);
        Ok(goal)
    }

    pub(super) async fn emit_thread_goal_updated_impl(&self, session_id: &str, goal: Option<ThreadGoal>) {
        let goal = northhing_agent_runtime::thread_goal::thread_goal_event_payload(goal);
        self.emit_event(AgenticEvent::ThreadGoalUpdated {
            session_id: session_id.to_string(),
            goal,
        })
        .await;
    }

    pub(super) async fn activate_session_goal_impl(
        &self,
        session_id: String,
        user_hint: Option<String>,
    ) -> NortHingResult<ThreadGoal> {
        let objective = user_hint.ok_or_else(|| {
            NortHingError::Validation("Goal objective is required. Use /goal <objective>.".to_string())
        })?;
        let workspace_path = self.require_main_session_workspace(&session_id)?;
        let existing = self.get_thread_goal(&session_id, workspace_path.as_path()).await?;
        let replace_existing = existing.is_some();
        let goal = self
            .set_thread_goal_objective_impl(&session_id, workspace_path.as_path(), objective, replace_existing)
            .await
            .map_err(user_facing_thread_goal_error)?;
        info!(
            "Thread goal set from /goal: session_id={}, objective={}",
            session_id, goal.objective
        );
        Ok(goal)
    }

    /// Continue an active thread goal after a dialog turn completes (Codex-style).
    pub(super) async fn prepare_goal_continuation_after_turn_impl(
        &self,
        session_id: &str,
        source_turn_id: &str,
        user_input: &str,
        user_message_metadata: Option<&serde_json::Value>,
        turn_completed: bool,
    ) -> NortHingResult<Option<ThreadGoalContinuationPlan>> {
        if should_skip_goal_continuation_after_turn(user_input, user_message_metadata) {
            return Ok(None);
        }

        let workspace_path = match self.require_main_session_workspace(session_id) {
            Ok(path) => path,
            Err(_) => return Ok(None),
        };

        let turn_tokens = self.thread_goal_runtime.turn_cumulative_billable_tokens(source_turn_id);

        let goal_before = self.get_thread_goal(session_id, workspace_path.as_path()).await?;

        let plan = maybe_build_continuation_after_turn(
            &self.thread_goal_store(),
            self.thread_goal_runtime.as_ref(),
            session_id,
            workspace_path.as_path(),
            source_turn_id,
            turn_tokens,
            turn_completed,
        )
        .await?;

        let goal_after = self.get_thread_goal(session_id, workspace_path.as_path()).await?;
        if goal_before.as_ref().map(|goal| goal.status) != goal_after.as_ref().map(|goal| goal.status) {
            if let Some(goal) = goal_after {
                self.emit_thread_goal_updated_impl(session_id, Some(goal)).await;
            }
        }

        Ok(plan)
    }
}
