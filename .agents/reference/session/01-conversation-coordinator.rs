// REFERENCE — extracted from
//   src/crates/assembly/core/src/agentic/coordination/coordinator.rs
// Last synced: 2813b36 (v3-restructure)
// The full file is 6366 lines. This is the public surface + the load-bearing
// state-machine guard point. Read the source for subagent plumbing,
// execution routing, and event emission.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use tokio::sync::{mpsc, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio_util::sync::CancellationToken;

use super::state_manager::SessionStateManager;
use super::turn_outcome::TurnOutcome;
use crate::agentic::core::state::{ProcessingPhase, SessionState};
use crate::agentic::session::session_manager::SessionManager;
use crate::agentic::WorkspaceBinding;
use crate::contracts::runtime_ports::{
    AgentDialogTurnRequest, DialogSubmissionPolicy, DialogTriggerSource,
};

/// ★ Top-level integration of session / execution / events / scheduler / subagents.
pub struct ConversationCoordinator {
    session_manager: Arc<SessionManager>,
    execution_engine: Arc<crate::agentic::execution::execution_engine::ExecutionEngine>,
    tool_pipeline: Arc<crate::agentic::tools::pipeline::ToolPipeline>,
    event_queue: Arc<crate::agentic::events::queue::EventQueue>,
    event_router: Arc<crate::agentic::events::router::EventRouter>,
    subagent_concurrency_limiter: Arc<RwLock<Option<SubagentConcurrencyLimiter>>>,
    subagent_profile_concurrency_limiters: Arc<RwLock<std::collections::HashMap<usize, SubagentConcurrencyLimiter>>>,
    /// Registry for dynamically adjusting subagent timeouts.
    subagent_timeout_registry: Arc<RwLock<std::collections::HashMap<String, Arc<SubagentTimeoutHandle>>>>,
    active_subagent_executions: Arc<DashMap<String, CancellationToken>>,
    /// mpsc sender to the scheduler for `(session_id, TurnOutcome)` pairs.
    /// Set once at construction; `OnceLock` is used because the scheduler
    /// may be wired in after coordinator creation. ★ **See NOTES.md — do
    /// not copy this pattern for the new actor design.**
    scheduler_notify_tx: OnceLock<mpsc::Sender<(String, TurnOutcome)>>,
    round_injection_source: OnceLock<Arc<dyn DialogRoundInjectionSource>>,
    active_turns_per_session: Arc<DashMap<String, Arc<std::sync::atomic::AtomicUsize>>>,
    thread_goal_runtime: Arc<crate::agentic::thread_goal::ThreadGoalRuntime>,
}

impl ConversationCoordinator {
    /// Internal constructor used by tests. Production code goes through `get_global_coordinator()`.
    pub fn new(/* ... */) -> Self { /* coordinator.rs:399 */ unimplemented!() }

    // ════════════════════════════════════════════════════════════════
    // ★★★ THE 6 PUBLIC ENTRY POINTS (consumed by `app_state.rs`) ★★★
    // ════════════════════════════════════════════════════════════════

    /// Create a new session in the given workspace.
    /// Line 1085.
    pub async fn create_session(
        &self,
        name: Option<String>,
        agent_type: String,
        config: SessionConfig,
    ) -> NortHingResult<SessionSummary> { unimplemented!() }

    /// Start a new dialog turn. The main hot path.
    /// Line 1929. Funnels through `start_dialog_turn_internal` (2717) which
    /// enforces the `Idle | Error` state-machine guard.
    pub async fn start_dialog_turn(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<std::path::PathBuf>,
        submission_policy: DialogSubmissionPolicy,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<DialogSubmitOutcome> { unimplemented!() }

    /// Delete a session and its persisted state.
    /// Line 3670. Takes 2 args: workspace_path + session_id.
    pub async fn delete_session(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<()> { unimplemented!() }

    /// List all sessions in a workspace.
    /// Line 3866.
    pub async fn list_sessions(
        &self,
        workspace_path: &Path,
    ) -> NortHingResult<Vec<SessionSummary>> { unimplemented!() }

    /// Get all messages for a session (full history).
    /// Line 3880.
    pub async fn get_messages(
        &self,
        session_id: &str,
    ) -> NortHingResult<Vec<Message>> { unimplemented!() }

    /// Paginated message fetch.
    /// Line 3885.
    pub async fn get_messages_paginated(
        &self,
        session_id: &str,
        before: Option<MessageId>,
        limit: usize,
    ) -> NortHingResult<Vec<Message>> { unimplemented!() }

    // ════════════════════════════════════════════════════════════════
    // Other notable public methods
    // ════════════════════════════════════════════════════════════════

    /// 12 variants for restore (line 3721-3863). `restore_session` is the
    /// base; the rest accept progressively more context.
    pub async fn restore_session(&self, /* ... */) -> NortHingResult<()> { /* line 3721 */ unimplemented!() }

    /// Delete hidden subagent sessions that were spawned by a parent turn.
    /// Line 3685. Cleanup path used after the parent turn ends.
    pub async fn delete_hidden_subagent_sessions_for_parent_turns(
        &self, /* ... */
    ) -> NortHingResult<()> { /* line 3685 */ unimplemented!() }

    /// Resolve the actual workspace path for a session, even if it was
    /// moved after creation. Line 3870.
    pub async fn resolve_session_workspace_path(
        &self, session_id: &str,
    ) -> NortHingResult<std::path::PathBuf> { /* line 3870 */ unimplemented!() }
}

// ═══════════════════════════════════════════════════════════════════════
// ★★★ THE STATE-MACHINE GUARD POINT (load-bearing for correctness) ★★★
// ═══════════════════════════════════════════════════════════════════════
//
// In `start_dialog_turn_internal` at coordinator.rs:2717-2840, the coordinator
// checks the session's current state and only allows a new turn when the
// state is `Idle` or `Error {..}`. A `Processing {..}` state causes rejection
// with a `Validation` error. This is THE place to extend if you add new
// states — e.g. a `Paused` state would need an additional match arm here.
//
// Reference: `SessionState` and `ProcessingPhase` in `04-session-state.rs`.

// ═══════════════════════════════════════════════════════════════════════
// 4 sibling entry points — DO NOT copy this pattern (see NOTES.md)
// ═══════════════════════════════════════════════════════════════════════
//
// `start_dialog_turn` has 4 sibling functions at lines 1929/1957/1986/2015:
//   - start_dialog_turn
//   - start_dialog_turn_with_prepended_messages         (line 1957)
//   - start_dialog_turn_with_image_contexts              (line 1986)
//   - start_dialog_turn_with_image_contexts_and_prepended_messages (line 2015)
// All four funnel into `start_dialog_turn_internal` (line 2717). The new
// actor / dispatcher design should NOT copy this 4-way facade — collapse
// into one parameterized entry instead.
