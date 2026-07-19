// Subagent types + accessors + simple helpers (pre-impl)
// (lines 1-617 of original coordinator.rs)
// No impl ConversationCoordinator here; that is split across dialog_turn.rs + subagent_orchestrator.rs.

use super::{
    scheduler::{
        abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
    },
    turn_outcome::TurnOutcome,
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

/// Subagent execution result
///
/// Contains the text response after subagent execution
#[derive(Debug, Clone)]
pub struct SubagentResult {
    /// AI text response
    pub text: String,
    /// Structured JSON output if the tool returned valid JSON.
    /// `None` when the output is plain text or invalid JSON.
    pub structured_output: Option<serde_json::Value>,
    pub status: SubagentResultStatus,
    pub reason: Option<String>,
    pub ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentResultStatus {
    Completed,
    PartialTimeout,
}

#[derive(Debug, Clone)]
pub(crate) struct SubagentExecutionRequest {
    pub(crate) task_description: String,
    pub(crate) context_mode: SubagentContextMode,
    pub(crate) subagent_type: Option<String>,
    pub(crate) workspace_path: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) subagent_parent_info: SubagentParentInfo,
    pub(crate) context: HashMap<String, String>,
    /// Execution policy for the child subagent session being launched.
    pub(crate) delegation_policy: DelegationPolicy,
}

pub(crate) struct WrappedUserInputPayload {
    pub(crate) content: String,
    pub(crate) prepended_messages: Vec<Message>,
    pub(crate) skill_agent_snapshot: TurnSkillAgentSnapshot,
    pub(crate) snapshot_persistence: SkillAgentSnapshotPersistence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillAgentSnapshotPersistence {
    None,
    SaveCurrentTurn,
    RecoverFirstTurnBaseline,
}

impl SubagentResult {
    pub(crate) fn completed(text: String) -> Self {
        Self {
            text,
            structured_output: None,
            status: SubagentResultStatus::Completed,
            reason: None,
            ledger_event_id: None,
        }
    }

    // reason: partial_timeout() constructor is reserved for the upcoming cancellation surface; today partial-timeout results are constructed inline by the boundary tests
    #[allow(dead_code)]
    pub(crate) fn partial_timeout(text: String, reason: String) -> Self {
        Self {
            text,
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some(reason),
            ledger_event_id: None,
        }
    }

    // reason: with_ledger_event_id() builder is reserved for the upcoming ledger-event correlation tracking; today callers set ledger_event_id directly
    #[allow(dead_code)]
    pub(crate) fn with_ledger_event_id(mut self, event_id: String) -> Self {
        self.ledger_event_id = Some(event_id);
        self
    }

    pub fn is_partial_timeout(&self) -> bool {
        self.status == SubagentResultStatus::PartialTimeout
    }

    pub fn ledger_event_id(&self) -> Option<&str> {
        self.ledger_event_id.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct BackgroundSubagentStartResult {
    pub background_task_id: String,
}

fn format_background_subagent_delivery_text(
    background_task_id: &str,
    agent_type: &str,
    outcome: Result<&SubagentResult, &NortHingError>,
) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed with partial timeout result:\n<partial_result status=\"partial_timeout\">\n{}\n</partial_result>",
                    agent_type, background_task_id, result.text
                )
            } else {
                format!(
                    "Background subagent '{}' (background_task_id='{}') completed successfully:\n<result>\n{}\n</result>",
                    agent_type, background_task_id, result.text
                )
            }
        }
        Err(error) => {
            format!(
                "Background subagent '{}' (background_task_id='{}') failed before producing a final result.\nError: {}",
                agent_type, background_task_id, error
            )
        }
    }
}

fn format_background_subagent_display_text(outcome: Result<&SubagentResult, &NortHingError>) -> String {
    match outcome {
        Ok(result) => {
            if result.is_partial_timeout() {
                "Background subagent completed with a partial timeout result.".to_string()
            } else {
                "Background subagent completed successfully.".to_string()
            }
        }
        Err(_) => "Background subagent failed before producing a final result.".to_string(),
    }
}

fn build_subagent_session_relationship(
    parent_info: Option<&SubagentParentInfo>,
    agent_type: &str,
) -> SessionRelationship {
    SessionRelationship {
        kind: Some(SessionRelationshipKind::Subagent),
        parent_session_id: parent_info.map(|info| info.session_id.clone()),
        parent_request_id: None,
        parent_dialog_turn_id: parent_info.map(|info| info.dialog_turn_id.clone()),
        parent_turn_index: None,
        parent_tool_call_id: parent_info.map(|info| info.tool_call_id.clone()),
        subagent_type: Some(agent_type.to_string()),
    }
}

fn fork_subagent_system_reminder() -> String {
    "<system_reminder>You are now running as a forked subagent. Messages before this reminder were inherited from the parent agent as context. Messages after this reminder are the request for you. Do not call the Task tool to launch another subagent. Use the tools available to complete the task directly.</system_reminder>".to_string()
}

fn runtime_tool_restrictions_for_delegation_policy(delegation_policy: DelegationPolicy) -> ToolRuntimeRestrictions {
    let mut restrictions = ToolRuntimeRestrictions::default();
    if !delegation_policy.allow_subagent_spawn {
        restrictions.denied_tool_names.insert("Task".to_string());
        restrictions.denied_tool_messages.insert(
            "Task".to_string(),
            "Recursive subagent delegation is blocked. Use direct tools instead.".to_string(),
        );
    }
    restrictions
}

#[derive(Clone)]
pub(crate) struct HiddenSubagentExecutionRequest {
    pub(crate) session_name: String,
    pub(crate) agent_type: String,
    pub(crate) session_config: SessionConfig,
    pub(crate) initial_messages: Vec<Message>,
    pub(crate) user_input_text: String,
    pub(crate) created_by: Option<String>,
    pub(crate) subagent_parent_info: Option<SubagentParentInfo>,
    pub(crate) context: HashMap<String, String>,
    pub(crate) delegation_policy: DelegationPolicy,
    pub(crate) runtime_tool_restrictions: ToolRuntimeRestrictions,
    pub(crate) prompt_cache_source_session_id: Option<String>,
}

pub use northhing_runtime_ports::DialogTriggerSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantBootstrapSkipReason {
    BootstrapNotRequired,
    SessionHasExistingTurns,
    SessionNotIdle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantBootstrapBlockReason {
    ModelUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistantBootstrapEnsureOutcome {
    Started {
        session_id: String,
        turn_id: String,
    },
    Skipped {
        session_id: String,
        reason: AssistantBootstrapSkipReason,
    },
    Blocked {
        session_id: String,
        reason: AssistantBootstrapBlockReason,
        detail: String,
    },
}

pub const ASSISTANT_BOOTSTRAP_AGENT_TYPE: &str = "Claw";

/// Cancel token cleanup guard
///
/// Automatically cleans up cancel tokens in ExecutionEngine when dropped
pub(crate) struct CancelTokenGuard {
    pub(crate) execution_engine: Arc<ExecutionEngine>,
    pub(crate) dialog_turn_id: String,
}

impl Drop for CancelTokenGuard {
    fn drop(&mut self) {
        let execution_engine = self.execution_engine.clone();
        let dialog_turn_id = self.dialog_turn_id.clone();

        tokio::spawn(async move {
            execution_engine.cleanup_cancel_token(&dialog_turn_id).await;
        });
    }
}

#[derive(Clone)]
pub(crate) struct ActiveSubagentExecution {
    pub(crate) parent_session_id: String,
    pub(crate) parent_dialog_turn_id: String,
    pub(crate) subagent_session_id: String,
    pub(crate) subagent_dialog_turn_id: String,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) abort_handle: tokio::task::AbortHandle,
}

/// Ensures orphaned subagent work is stopped when the parent tool await is dropped.
pub(crate) struct SubagentExecutionScope {
    pub(crate) execution_engine: Arc<ExecutionEngine>,
    pub(crate) tool_pipeline: Arc<ToolPipeline>,
    pub(crate) session_manager: Arc<SessionManager>,
    pub(crate) active_subagent_executions: Arc<DashMap<String, ActiveSubagentExecution>>,
    pub(crate) subagent_session_id: String,
    pub(crate) subagent_dialog_turn_id: String,
    pub(crate) subagent_cancel_token: CancellationToken,
    pub(crate) abort_handle: tokio::task::AbortHandle,
    pub(crate) disarmed: bool,
}

impl SubagentExecutionScope {
    pub(crate) fn disarm(&mut self) {
        self.disarmed = true;
        self.active_subagent_executions.remove(&self.subagent_session_id);
    }
}

impl Drop for SubagentExecutionScope {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }

        warn!(
            "Subagent execution scope dropped without normal completion; stopping orphaned subagent: session_id={}, dialog_turn_id={}",
            self.subagent_session_id, self.subagent_dialog_turn_id
        );

        self.subagent_cancel_token.cancel();
        self.abort_handle.abort();
        self.active_subagent_executions.remove(&self.subagent_session_id);

        let execution_engine = self.execution_engine.clone();
        let tool_pipeline = self.tool_pipeline.clone();
        let session_manager = self.session_manager.clone();
        let subagent_session_id = self.subagent_session_id.clone();
        let subagent_dialog_turn_id = self.subagent_dialog_turn_id.clone();

        tokio::spawn(async move {
            if let Err(error) = execution_engine.cancel_dialog_turn(&subagent_dialog_turn_id).await {
                warn!(
                    "Failed to cancel orphaned subagent dialog turn: session_id={}, dialog_turn_id={}, error={}",
                    subagent_session_id, subagent_dialog_turn_id, error
                );
            }

            if let Err(error) = tool_pipeline.cancel_dialog_turn_tools(&subagent_dialog_turn_id).await {
                warn!(
                    "Failed to cancel orphaned subagent tools: session_id={}, dialog_turn_id={}, error={}",
                    subagent_session_id, subagent_dialog_turn_id, error
                );
            }

            session_manager.reset_session_state_if_processing(&subagent_session_id, &subagent_dialog_turn_id);
        });
    }
}

#[derive(Clone)]
pub(crate) struct SubagentConcurrencyLimiter {
    pub(crate) semaphore: Arc<Semaphore>,
    pub(crate) max_concurrency: usize,
}

pub(crate) struct SubagentConcurrencyPermitGuard {
    permits: Vec<(OwnedSemaphorePermit, SubagentConcurrencyLimiter)>,
    agent_type: String,
}

impl SubagentConcurrencyPermitGuard {
    pub(crate) fn new(permits: Vec<(OwnedSemaphorePermit, SubagentConcurrencyLimiter)>, agent_type: String) -> Self {
        Self { permits, agent_type }
    }
}

impl Drop for SubagentConcurrencyPermitGuard {
    fn drop(&mut self) {
        for (permit, limiter) in std::mem::take(&mut self.permits) {
            drop(permit);

            let active_subagents = limiter
                .max_concurrency
                .saturating_sub(limiter.semaphore.available_permits());
            debug!(
                "Released subagent concurrency permit: agent_type={}, active_subagents={}, max_concurrency={}",
                self.agent_type, active_subagents, limiter.max_concurrency
            );
        }
    }
}

pub(crate) fn normalize_subagent_max_concurrency(raw: usize) -> usize {
    raw.clamp(1, MAX_SUBAGENT_MAX_CONCURRENCY)
}

/// Actions for dynamically adjusting a subagent's timeout.
#[derive(Debug, Clone)]
pub enum SubagentTimeoutAction {
    /// Disable timeout (run without limit).
    Disable,
    /// Restore timeout using the remaining time captured at disable.
    Restore,
    /// Extend timeout by specified seconds from now.
    Extend { seconds: u64 },
}

/// Shared handle for dynamically adjusting a subagent's timeout deadline.
pub(crate) struct SubagentTimeoutHandle {
    /// watch sender: None = no timeout, Some(instant) = deadline.
    pub(crate) deadline_tx: watch::Sender<Option<Instant>>,
    /// Session ID this handle belongs to.
    // reason: session_id is held for the upcoming handle-inspection API (today's timeout handle is consumed by the runtime only)
    #[allow(dead_code)]
    pub(crate) session_id: String,
    /// Original timeout in seconds (for restore calculations).
    pub(crate) original_timeout_seconds: Option<u64>,
    /// Remaining seconds at the moment timeout was disabled.
    pub(crate) remaining_at_pause: std::sync::Mutex<Option<u64>>,
}

impl SubagentTimeoutHandle {
    fn disable_timeout(&self) {
        let remaining = match *self.deadline_tx.borrow() {
            Some(deadline) => {
                let now = Instant::now();
                if deadline > now {
                    deadline.duration_since(now).as_secs()
                } else {
                    0
                }
            }
            None => self.original_timeout_seconds.unwrap_or(0),
        };
        if let Ok(mut guard) = self.remaining_at_pause.lock() {
            *guard = Some(remaining);
        } else {
            warn!(
                "Timeout pause: failed to acquire remaining_at_pause lock; \
                 pause remainder not recorded"
            );
        }
        if let Err(e) = self.deadline_tx.send(None) {
            warn!(
                "Timeout pause: failed to send deadline-clear signal (receiver dropped?): {e}; \
                 active deadline will expire naturally"
            );
        }
    }

    fn restore_timeout(&self) {
        let remaining = self
            .remaining_at_pause
            .lock()
            .ok()
            .and_then(|guard| *guard)
            .unwrap_or_else(|| self.original_timeout_seconds.unwrap_or(0));
        let new_deadline = Instant::now() + Duration::from_secs(remaining);
        if let Err(e) = self.deadline_tx.send(Some(new_deadline)) {
            warn!(
                "Timeout restore: failed to send deadline-update signal (receiver dropped?): {e}; \
                 deadline will fall back to original timeout"
            );
        }
        if let Ok(mut guard) = self.remaining_at_pause.lock() {
            *guard = None;
        } else {
            warn!("Timeout restore: failed to acquire remaining_at_pause lock");
        }
    }

    fn extend_timeout(&self, seconds: u64) {
        let new_deadline = Instant::now() + Duration::from_secs(seconds);
        if let Err(e) = self.deadline_tx.send(Some(new_deadline)) {
            warn!(
                "Timeout extend: failed to send deadline-update signal (receiver dropped?): {e}; \
                 extension has no effect"
            );
        }
        if let Ok(mut guard) = self.remaining_at_pause.lock() {
            *guard = None;
        } else {
            warn!("Timeout extend: failed to acquire remaining_at_pause lock");
        }
    }

    pub(crate) fn apply_action(&self, action: SubagentTimeoutAction) {
        match action {
            SubagentTimeoutAction::Disable => self.disable_timeout(),
            SubagentTimeoutAction::Restore => self.restore_timeout(),
            SubagentTimeoutAction::Extend { seconds } => self.extend_timeout(seconds),
        }
    }
}

/// Conversation coordinator
pub struct ConversationCoordinator {
    pub session_manager: Arc<SessionManager>,
    pub execution_engine: Arc<ExecutionEngine>,
    pub tool_pipeline: Arc<ToolPipeline>,
    pub event_queue: Arc<EventQueue>,
    pub event_router: Arc<EventRouter>,
    pub(crate) subagent_concurrency_limiter: Arc<RwLock<Option<SubagentConcurrencyLimiter>>>,
    pub(crate) subagent_profile_concurrency_limiters: Arc<RwLock<HashMap<usize, SubagentConcurrencyLimiter>>>,
    /// Registry for dynamically adjusting subagent timeouts.
    pub(crate) subagent_timeout_registry: Arc<RwLock<HashMap<String, Arc<SubagentTimeoutHandle>>>>,
    /// Active subagent executions keyed by subagent session id.
    pub(crate) active_subagent_executions: Arc<DashMap<String, ActiveSubagentExecution>>,
    /// Notifies DialogScheduler of turn outcomes; injected after construction
    pub scheduler_notify_tx: OnceLock<mpsc::Sender<(String, TurnOutcome)>>,
    /// Round-boundary user steering source (mid-turn user message injection); injected after construction
    pub round_injection_source: OnceLock<Arc<dyn DialogRoundInjectionSource>>,
    /// In-flight dialog turn tracker per session, used to serialize cancel→start
    /// transitions so a new turn never starts touching the in-memory message
    /// list while the previous (cancelled) turn's spawn task is still draining.
    /// Map value is a counter shared between the coordinator and the spawn
    /// task; spawn task increments on entry and decrements on exit.
    pub active_turns_per_session: Arc<DashMap<String, Arc<AtomicUsize>>>,
    /// In-flight dialog turn tasks keyed by dialog_turn_id. Storage + cleanup
    /// only (no shutdown-await wiring). Allows callers to track or await
    /// individual turn tasks.
    pub active_turn_tasks: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    pub thread_goal_runtime: Arc<ThreadGoalRuntime>,
}
// ══════════════════════════════════════════════════════════════════════
// SUBAGENT PHASE OUTPUT STRUCTS
// Phase output types used to pass data between helper functions.
// ══════════════════════════════════════════════════════════════════════

/// Output of Phase 1 — session creation, concurrency, timeout, lineage.
pub(crate) struct SubagentPhase1Output {
    pub(crate) agent_type: String,
    pub(crate) session_id: String,
    pub(crate) initial_messages: Vec<Message>,
    pub(crate) user_input_text: String,
    pub(crate) subagent_parent_info: Option<SubagentParentInfo>,
    pub(crate) context: HashMap<String, String>,
    pub(crate) delegation_policy: DelegationPolicy,
    pub(crate) runtime_tool_restrictions: ToolRuntimeRestrictions,
    pub(crate) turn_index: usize,
    pub(crate) dialog_turn_id: String,
    pub(crate) subagent_cancel_token: CancellationToken,
    pub(crate) deadline_rx: tokio::sync::watch::Receiver<Option<Instant>>,
    pub(crate) requested_timeout_seconds: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) timeout_error_message: String,
    pub(crate) parent_session_id: String,
    pub(crate) parent_dialog_turn_id: String,
    pub(crate) parent_tool_call_id: String,
    pub(crate) subagent_workspace: Option<WorkspaceBinding>,
    pub(crate) subagent_started_at: Instant,
}

/// Output of Phase 2 — execution result and metadata for Phase 3 finalisation.
pub(crate) struct SubagentPhase2Output {
    pub(crate) result: NortHingResult<ExecutionResult>,
    pub(crate) session_id: String,
    pub(crate) dialog_turn_id: String,
    pub(crate) turn_index: usize,
    pub(crate) user_input_text: String,
    pub(crate) agent_type: String,
    pub(crate) subagent_workspace_path: Option<String>,
    pub(crate) subagent_session_storage_path: Option<PathBuf>,
    pub(crate) parent_session_id: String,
    pub(crate) parent_dialog_turn_id: String,
    pub(crate) parent_tool_call_id: String,
    #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
    pub(crate) subagent_parent_info: Option<SubagentParentInfo>,
    #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
    pub(crate) subagent_cancel_token: CancellationToken,
    #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
    pub(crate) execution_task: tokio::task::JoinHandle<NortHingResult<ExecutionResult>>,
    pub(crate) execution_scope: SubagentExecutionScope,
    #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
    pub(crate) subagent_started_at: Instant,
}
