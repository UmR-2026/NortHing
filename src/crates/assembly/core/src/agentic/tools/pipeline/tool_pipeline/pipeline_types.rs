use crate::agentic::tools::computer_use_host::ComputerUseHostRef;
use crate::agentic::tools::pipeline::state_manager::ToolStateManager;
use crate::agentic::tools::pipeline::types::*;
use crate::agentic::tools::registry::ToolRegistry;
use crate::util::errors::{NortHingError, NortHingResult};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{oneshot, RwLock as TokioRwLock};
use tokio_util::sync::CancellationToken;

/// Confirmation response type
#[derive(Debug, Clone)]
pub enum ConfirmationResponse {
    Confirmed,
    Rejected(String),
}

/// Tool pipeline
pub struct ToolPipeline {
    pub(crate) tool_registry: Arc<TokioRwLock<ToolRegistry>>,
    pub(crate) state_manager: Arc<ToolStateManager>,
    /// Confirmation channel management (tool_id -> oneshot sender)
    pub(crate) confirmation_channels: Arc<DashMap<String, oneshot::Sender<ConfirmationResponse>>>,
    /// Cancellation token management (tool_id -> CancellationToken)
    pub(crate) cancellation_tokens: Arc<DashMap<String, CancellationToken>>,
    pub(crate) computer_use_host: Option<ComputerUseHostRef>,
    /// K.2.3 follow-up: the optional `ActorRuntime` shared with
    /// `ToolUseContext` so tools can drive long-running skills.
    /// `Arc<OnceLock<...>>` because `ToolPipeline` is wrapped in
    /// `Arc<ToolPipeline>` at call sites — the inner OnceLock gives
    /// idempotent late-binding (matches `AppState::actor_runtime`
    /// pattern).
    pub(crate) actor_runtime: Arc<OnceLock<Arc<northhing_agent_dispatch::ActorRuntime>>>,
}

impl ToolPipeline {
    pub fn new(
        tool_registry: Arc<TokioRwLock<ToolRegistry>>,
        state_manager: Arc<ToolStateManager>,
        computer_use_host: Option<ComputerUseHostRef>,
        actor_runtime: Arc<OnceLock<Arc<northhing_agent_dispatch::ActorRuntime>>>,
    ) -> Self {
        Self {
            tool_registry,
            state_manager,
            confirmation_channels: Arc::new(DashMap::new()),
            cancellation_tokens: Arc::new(DashMap::new()),
            computer_use_host,
            actor_runtime,
        }
    }

    /// K.2.3 follow-up: late-bind the `ActorRuntime` after
    /// `ToolPipeline::new()`. Idempotent — `set` returns Err if
    /// already set, which we silently ignore (matches
    /// `AppState::set_actor_runtime` semantics).
    pub fn set_actor_runtime(&self, runtime: Arc<northhing_agent_dispatch::ActorRuntime>) {
        let _ = self.actor_runtime.set(runtime);
    }

    pub fn computer_use_host(&self) -> Option<ComputerUseHostRef> {
        self.computer_use_host.clone()
    }
}
