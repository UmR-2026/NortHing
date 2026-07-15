//! Sub-domain: session.
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
    pub(crate) fn normalize_agent_type(agent_type: &str) -> String {
        if agent_type.trim().is_empty() {
            "agentic".to_string()
        } else {
            agent_type.trim().to_string()
        }
    }

    pub(crate) async fn create_hidden_subagent_session(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
        created_by: Option<String>,
    ) -> NortHingResult<Session> {
        self.session_manager
            .create_session_with_id_and_details(
                session_id,
                session_name,
                agent_type,
                config,
                created_by,
                SessionKind::Subagent,
            )
            .await
    }

    pub(crate) async fn load_session_context_messages(&self, session: &Session) -> NortHingResult<Vec<Message>> {
        let session_id = &session.session_id;
        let mut context_messages = self.session_manager.get_context_messages(session_id).await?;

        if context_messages.is_empty() && !session.dialog_turn_ids.is_empty() {
            if let Some(workspace_path) = session.config.workspace_path.as_deref() {
                match self
                    .session_manager
                    .restore_session(Path::new(workspace_path), session_id)
                    .await
                {
                    Ok(_) => {
                        context_messages = self.session_manager.get_context_messages(session_id).await?;
                    }
                    Err(e) => {
                        debug!(
                            "Failed to restore parent session context for fork capture: session_id={}, error={}",
                            session_id, e
                        );
                    }
                }
            }
        }

        Ok(context_messages)
    }

    pub(super) async fn wrap_user_input(
        &self,
        session_id: &str,
        turn_index: usize,
        agent_type: &str,
        previous_agent_type: Option<&str>,
        user_input: String,
        workspace: Option<&WorkspaceBinding>,
        workspace_services: Option<&WorkspaceServices>,
        enable_tools: bool,
        skill_agent_context_vars: &HashMap<String, String>,
    ) -> NortHingResult<WrappedUserInputPayload> {
        let agent_registry = agent_registry();
        if let Some(workspace) = workspace {
            if !workspace.is_remote() {
                agent_registry.load_custom_subagents(workspace.root_path()).await;
            }
        }
        let current_agent = agent_registry
            .get_agent(agent_type, workspace.map(|binding| binding.root_path()))
            .ok_or_else(|| NortHingError::NotFound(format!("Agent not found: {}", agent_type)))?;
        let current_agent_reminder = current_agent
            .get_system_reminder(previous_agent_type, workspace)
            .await?;
        let surface_resolution = resolve_skill_agent_snapshot(
            agent_type,
            workspace,
            workspace_services,
            enable_tools,
            skill_agent_context_vars,
            Some(&user_input),
        )
        .await;

        let mut prepended_messages = Vec::new();

        let snapshot_persistence = if turn_index == 0 {
            SkillAgentSnapshotPersistence::SaveCurrentTurn
        } else if self
            .session_manager
            .turn_skill_agent_snapshot(session_id, 0)
            .await
            .is_none()
        {
            warn!(
                "First-turn skill-agent snapshot missing; recovering baseline from current skill-agent snapshot: session_id={}, turn_index={}",
                session_id, turn_index
            );
            SkillAgentSnapshotPersistence::RecoverFirstTurnBaseline
        } else if let Some((baseline_turn_index, previous_snapshot)) = self
            .session_manager
            .latest_turn_skill_agent_snapshot_at_or_before(session_id, turn_index - 1)
            .await
        {
            let diff = diff_skill_agent_snapshot(&previous_snapshot, &surface_resolution.snapshot);
            if let Some(skill_update) = diff.render_skill_listing_update() {
                prepended_messages.push(Message::internal_reminder(
                    InternalReminderKind::SkillListingDiff,
                    skill_update,
                ));
            }
            if let Some(agent_update) = diff.render_agent_listing_update() {
                prepended_messages.push(Message::internal_reminder(
                    InternalReminderKind::AgentListingDiff,
                    agent_update,
                ));
            }
            if diff.is_empty() {
                SkillAgentSnapshotPersistence::None
            } else {
                debug!(
                    "Skill-agent snapshot changed; persisting sparse snapshot: session_id={}, turn_index={}, baseline_turn_index={}",
                    session_id, turn_index, baseline_turn_index
                );
                SkillAgentSnapshotPersistence::SaveCurrentTurn
            }
        } else {
            warn!(
                "No prior skill-agent snapshot available for diff; skipping skill-agent diff: session_id={}, turn_index={}",
                session_id, turn_index
            );
            SkillAgentSnapshotPersistence::None
        };

        if !current_agent_reminder.is_empty() {
            prepended_messages.push(Message::internal_reminder(
                InternalReminderKind::AgentMode,
                current_agent_reminder,
            ));
        }

        Ok(WrappedUserInputPayload {
            content: user_input,
            prepended_messages,
            skill_agent_snapshot: surface_resolution.snapshot,
            snapshot_persistence,
        })
    }

    // ------------------------------------------------------------------------
    // R21c: 9 miscellaneous methods migrated from mod.rs L1571-1644.
    // Visibility: `pub(super)` (sibling callers). mod.rs holds the facade
    // delegates with the original names. Inner methods use the `_inner` suffix
    // per R21 spec §2.4 (avoid facade-method name collision in
    // `impl ConversationCoordinator` blocks across sibling files).
    //
    // Group breakdown (per R21 §2.3):
    //   - 2 session accessors: list_sessions, resolve_session_workspace_path
    //   - 2 message reads: get_messages, get_messages_paginated
    //   - 2 subscriber registry: subscribe_internal, unsubscribe_internal
    //   - 3 tool control (kept here per R21 §2.3, future R22 may split into
    //     tool_control.rs): confirm_tool, reject_tool, cancel_tool
    // ------------------------------------------------------------------------

    /// List all sessions for a workspace.
    pub(super) async fn list_sessions_impl(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>> {
        self.session_manager.list_sessions(workspace_path).await
    }

    /// Resolve the workspace path that owns the given session id.
    pub(super) async fn resolve_session_workspace_path_impl(&self, session_id: &str) -> Option<std::path::PathBuf> {
        self.session_manager.resolve_session_workspace_path(session_id).await
    }

    /// Get a best-effort message view for a session.
    pub(super) async fn get_messages_impl(&self, session_id: &str) -> NortHingResult<Vec<Message>> {
        self.session_manager.get_messages(session_id).await
    }

    /// Get a paginated best-effort message view for a session.
    pub(super) async fn get_messages_paginated_impl(
        &self,
        session_id: &str,
        limit: usize,
        before_message_id: Option<&str>,
    ) -> NortHingResult<(Vec<Message>, bool)> {
        self.session_manager
            .get_messages_paginated(session_id, limit, before_message_id)
            .await
    }

    /// Subscribe to internal events (for internal systems such as logging or
    /// monitoring).
    pub(super) fn subscribe_internal_impl<H>(&self, subscriber_id: String, handler: H)
    where
        H: EventSubscriber + 'static,
    {
        self.event_router.subscribe_internal(subscriber_id, Arc::new(handler));
    }

    /// Unsubscribe from internal events previously added via
    /// `subscribe_internal_impl`.
    pub(super) fn unsubscribe_internal_impl(&self, subscriber_id: &str) {
        self.event_router.unsubscribe_internal(subscriber_id);
    }

    /// Confirm tool execution.
    pub(super) async fn confirm_tool_impl(
        &self,
        tool_id: &str,
        updated_input: Option<serde_json::Value>,
    ) -> NortHingResult<()> {
        self.tool_pipeline.confirm_tool(tool_id, updated_input).await
    }

    /// Reject tool execution.
    pub(super) async fn reject_tool_impl(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        self.tool_pipeline.reject_tool(tool_id, reason).await
    }

    /// Cancel tool execution.
    pub(super) async fn cancel_tool_impl(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        self.tool_pipeline.cancel_tool(tool_id, reason).await
    }
}
