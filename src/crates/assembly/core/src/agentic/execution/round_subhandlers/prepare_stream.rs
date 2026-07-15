//! `RoundExecutor::prepare_stream` sub-handler.
//!
//! Initializes per-round state: `round_started_at`, `is_subagent`,
//! `round_id`, `cancel_token`; emits `ModelRoundStarted` event; prepares
//! the model exchange trace handle; sets `max_attempts`.

use super::super::model_exchange_trace::prepare_model_exchange_trace;
use super::super::round_executor::RoundExecutor;
use super::round_state::RoundState;
use crate::agentic::events::{AgenticEvent, EventPriority};
use crate::util::errors::NortHingResult;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

impl RoundExecutor {
    /// Initialize round state: round_started_at, is_subagent, round_id, cancel_token,
    /// emit ModelRoundStarted, prepare_model_exchange_trace, max_attempts.
    pub(crate) async fn prepare_stream(&self, state: &mut RoundState) -> NortHingResult<()> {
        state.round_started_at = Instant::now();
        state.subagent_parent_info = state.context.subagent_parent_info.clone();
        state.is_subagent = state.subagent_parent_info.is_some();

        state.round_id = uuid::Uuid::new_v4().to_string();

        // Create or reuse cancellation token
        state.cancel_token =
            if let Some(existing_token) = self.cancellation_tokens.get(&state.context.dialog_turn_id.clone()) {
                existing_token.clone()
            } else {
                // Create new token
                let new_token = CancellationToken::new();
                self.cancellation_tokens
                    .insert(state.context.dialog_turn_id.clone(), new_token.clone());
                new_token
            };

        // Emit model round started event
        self.emit_event(
            AgenticEvent::ModelRoundStarted {
                session_id: state.context.session_id.clone(),
                turn_id: state.context.dialog_turn_id.clone(),
                round_id: state.round_id.clone(),
                round_index: state.context.round_number,
                model_id: Some(state.context.model_name.clone()),
            },
            EventPriority::High,
        )
        .await;

        state.trace_config =
            prepare_model_exchange_trace(&state.context, &state.round_id, state.ai_client.as_ref()).await;
        state.max_attempts = Self::MAX_STREAM_ATTEMPTS;
        Ok(())
    }
}
