//! Sub-domain: init.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains public constructor + accessor + setter methods on `ConversationCoordinator`.
//!
//! Sibling imports `use super::super::coordinator::*` for the struct definition
//! and field access; sibling struct fields remain `pub(super)` for cross-file access.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::turn_outcome::TurnOutcome;

use crate::agentic::events::{EventQueue, EventRouter};
use crate::agentic::execution::ExecutionEngine;
use crate::agentic::goal_mode::ThreadGoalRuntime;
use crate::agentic::round_preempt::DialogRoundInjectionSource;
use crate::agentic::session::SessionManager;
use crate::agentic::tools::pipeline::ToolPipeline;
use crate::util::errors::{NortHingError, NortHingResult};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

impl ConversationCoordinator {
    pub fn new(
        session_manager: Arc<SessionManager>,
        execution_engine: Arc<ExecutionEngine>,
        tool_pipeline: Arc<ToolPipeline>,
        event_queue: Arc<EventQueue>,
        event_router: Arc<EventRouter>,
    ) -> Self {
        Self {
            session_manager,
            execution_engine,
            tool_pipeline,
            event_queue,
            event_router,
            subagent_concurrency_limiter: Arc::new(RwLock::new(None)),
            subagent_profile_concurrency_limiters: Arc::new(RwLock::new(HashMap::new())),
            subagent_timeout_registry: Arc::new(RwLock::new(HashMap::new())),
            active_subagent_executions: Arc::new(dashmap::DashMap::new()),
            scheduler_notify_tx: OnceLock::new(),
            round_injection_source: OnceLock::new(),
            active_turns_per_session: Arc::new(dashmap::DashMap::new()),
            active_turn_tasks: Arc::new(dashmap::DashMap::new()),
            thread_goal_runtime: Arc::new(ThreadGoalRuntime::new()),
        }
    }

    pub fn thread_goal_runtime(&self) -> Arc<ThreadGoalRuntime> {
        Arc::clone(&self.thread_goal_runtime)
    }

    /// A2: expose execution engine for long-running skill tick API.
    pub(crate) fn execution_engine(&self) -> &Arc<ExecutionEngine> {
        &self.execution_engine
    }

    /// Inject the DialogScheduler notification channel after construction.
    /// Called once during app initialization after the scheduler is created.
    ///
    /// Returns `true` if the binding was installed; `false` if a notifier was
    /// already wired (in which case the call is ignored — first writer wins).
    pub fn set_scheduler_notifier(&self, tx: mpsc::Sender<(String, TurnOutcome)>) -> bool {
        if self.scheduler_notify_tx.set(tx).is_ok() {
            true
        } else {
            warn!(
                "Scheduler notifier already wired; ignoring re-initialization \
                 (a competing initializer beat this caller to it)"
            );
            false
        }
    }

    /// Wire round-boundary injection source (typically the scheduler's
    /// [`SessionRoundInjectionBuffer`](crate::agentic::round_preempt::SessionRoundInjectionBuffer)).
    ///
    /// Returns `true` if the binding was installed; `false` if a source was
    /// already wired.
    pub fn set_round_injection_source(&self, source: Arc<dyn DialogRoundInjectionSource>) -> bool {
        if self.round_injection_source.set(source).is_ok() {
            true
        } else {
            warn!(
                "Round injection source already wired; ignoring re-initialization \
                 (a competing initializer beat this caller to it)"
            );
            false
        }
    }

    /// K.2.3 follow-up: late-bind the actor runtime after
    /// coordinator construction. Forwards to `tool_pipeline` so the
    /// runtime shows up in every `ToolUseContext` built from this
    /// coordinator's pipeline. Idempotent (OnceLock semantics on
    /// the pipeline's setter).
    pub fn set_actor_runtime(&self, runtime: std::sync::Arc<northhing_agent_dispatch::ActorRuntime>) {
        self.tool_pipeline.set_actor_runtime(runtime);
    }

    /// Dynamically adjust a running subagent's timeout.
    pub async fn set_subagent_timeout(&self, session_id: &str, action: SubagentTimeoutAction) -> NortHingResult<()> {
        let registry = self.subagent_timeout_registry.read().await;
        let handle = registry.get(session_id).cloned().ok_or_else(|| {
            NortHingError::tool(format!("No active subagent timeout handle for session {}", session_id))
        })?;
        drop(registry);
        handle.apply_action(action.clone());
        info!(
            "Subagent timeout adjusted: session_id={}, action={:?}",
            session_id,
            std::mem::discriminant(&action)
        );
        Ok(())
    }
}
