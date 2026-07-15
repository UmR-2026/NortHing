//! Round state and dispatch outcome types shared across all 4 sub-handlers.
//!
//! Hosts:
//! - `RoundState`: state populated progressively by `prepare_stream` and
//!   mutated by `dispatch_stream`.
//! - `DispatchOutcome`: return value of `dispatch_stream`, consumed by
//!   `process_result`.
//! - `RoundState::new`: constructor invoked from `round_executor::execute_round`.
//! - `RoundExecutor::handle_error`: no-op placeholder preserving the
//!   4-stage lifecycle symmetry.

use super::super::round_executor::RoundExecutor;
use super::super::stream_processor::StreamResult;
use super::super::types::RoundContext as TypesRoundContext;
use crate::agentic::tools::pipeline::SubagentParentInfo;
use crate::infrastructure::ai::AIClient;
use crate::util::errors::NortHingResult;
use crate::util::types::Message as AIMessage;
use crate::util::types::ToolDefinition;
use northhing_ai_adapters::{ModelExchangeRequestTraceHandle, ModelExchangeTraceConfig};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// State shared across the 4 sub-handlers. Populated progressively:
/// - `RoundState::new` sets the 5 input fields + zero-init outputs
/// - `prepare_stream` fills 7 more outputs (round_id, cancel_token, ...)
/// - `dispatch_stream` mutates `attempt_index` and produces `DispatchOutcome`
pub(crate) struct RoundState {
    // Inputs (immutable after new)
    pub(super) ai_client: std::sync::Arc<AIClient>,
    pub(super) context: TypesRoundContext,
    pub(super) ai_messages: Vec<AIMessage>,
    pub(super) tool_definitions: Option<Vec<ToolDefinition>>,
    pub(super) context_window: Option<usize>,

    // Outputs of prepare_stream
    pub(super) round_started_at: Instant,
    pub(super) subagent_parent_info: Option<SubagentParentInfo>,
    pub(super) is_subagent: bool,
    pub(super) round_id: String,
    pub(super) cancel_token: CancellationToken,
    pub(super) max_attempts: usize,
    pub(super) trace_config: Option<ModelExchangeTraceConfig>,

    // Output of dispatch_stream (mutated)
    pub(super) attempt_index: usize,
}

/// Outcome of `dispatch_stream`: what `process_result` consumes.
pub(crate) struct DispatchOutcome {
    pub(super) stream_result: StreamResult,
    pub(super) send_to_stream_ms: u64,
    pub(super) stream_processing_ms: u64,
    pub(super) trace_handle: Option<ModelExchangeRequestTraceHandle>,
}

impl RoundState {
    pub(crate) fn new(
        ai_client: std::sync::Arc<AIClient>,
        context: TypesRoundContext,
        ai_messages: Vec<AIMessage>,
        tool_definitions: Option<Vec<ToolDefinition>>,
        context_window: Option<usize>,
    ) -> Self {
        Self {
            ai_client,
            context,
            ai_messages,
            tool_definitions,
            context_window,
            round_started_at: Instant::now(),
            subagent_parent_info: None,
            is_subagent: false,
            round_id: String::new(),
            cancel_token: CancellationToken::new(),
            max_attempts: 0,
            trace_config: None,
            attempt_index: 0,
        }
    }
}

impl RoundExecutor {
    /// No-op (errors propagate via `?`). Preserves 4-stage lifecycle symmetry.
    pub(crate) async fn handle_error(&self, _state: &mut RoundState) -> NortHingResult<()> {
        Ok(())
    }
}
