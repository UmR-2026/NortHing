//! Sub-domain: hidden subagent execution lifecycle (phase1/2/3).
//! Spec step-3.7 — extracted from subagent_orchestrator.rs (R50b refactor).

use super::super::a1_path::run_a1_path;
use super::super::coordinator::*;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_dispatch::{ActorRuntime, USE_LIGHTWEIGHT_ACTOR};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::debug;

mod cleanup;
mod lifecycle;
mod monitor;
mod spawn;

#[allow(unused_imports)]
pub use cleanup::*;
#[allow(unused_imports)]
pub use lifecycle::*;
#[allow(unused_imports)]
pub use monitor::*;
#[allow(unused_imports)]
pub use spawn::*;

impl ConversationCoordinator {
    /// B-2 (sub-agent hardening): legacy entry point for the
    /// `phase1/2/3` sub-agent execution path. New callers should go
    /// through the `SubAgentHandoff` trait (see
    /// `crate::agentic::coordination::handoff`). This fn is retained
    /// for the A1 fallback path (when `USE_LIGHTWEIGHT_ACTOR=true` and
    /// the caller passes no `actor_runtime`) and will be removed in a
    /// follow-up release once the A1 fallback is fully replaced by
    /// `run_a1_path`. Use the trait-level entry point
    /// `CoordinatorHiddenSubagentHandoff::handoff` for new code.
    #[deprecated(note = "B-2: use `SubAgentHandoff::handoff` (e.g. `CoordinatorHiddenSubagentHandoff`) instead; \
                        this fn remains only for the A1 fallback path (target removal post-0.1.0)")]
    pub(crate) async fn execute_hidden_subagent_internal(
        &self,
        request: HiddenSubagentExecutionRequest,
        cancel_token: Option<&CancellationToken>,
        timeout_seconds: Option<u64>,
        actor_runtime: Option<&Arc<ActorRuntime>>,
    ) -> NortHingResult<SubagentResult> {
        if USE_LIGHTWEIGHT_ACTOR {
            if let Some(runtime) = actor_runtime {
                return run_a1_path(runtime, &request, cancel_token, timeout_seconds).await;
            }
        }

        let phase1 = self
            .execute_hidden_subagent_phase1(request, cancel_token, timeout_seconds)
            .await?;

        let phase2 = self.execute_hidden_subagent_phase2(&phase1, cancel_token).await?;

        self.execute_hidden_subagent_phase3(phase2).await
    }
}
