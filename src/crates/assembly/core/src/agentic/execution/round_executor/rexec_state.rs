//! Cancel state and event emission for `RoundExecutor`.
//!
//! Sibling module to `round_executor/mod.rs` (Round 47c split). Holds:
//! - `MAX_STREAM_ATTEMPTS` constant
//! - `sleep_with_cancellation` cancellable-sleep helper (used by retry loops)
//! - Public cancel-state APIs: `register_cancel_token`, `has_active_dialog_turn`,
//!   `is_dialog_turn_cancelled`, `cancel_token_for_dialog_turn`, `cancel_dialog_turn`,
//!   `cleanup_dialog_turn`
//! - `emit_event` (sibling-visible event emission helper used by `rexec_run.rs`)

use super::RoundExecutor;
use crate::agentic::events::{AgenticEvent, EventPriority};
use crate::util::errors::{NortHingError, NortHingResult};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::debug;

impl RoundExecutor {
    pub(in crate::agentic::execution) const MAX_STREAM_ATTEMPTS: usize = 10;

    pub(in crate::agentic::execution) async fn sleep_with_cancellation(
        delay_ms: u64,
        cancel_token: &CancellationToken,
    ) -> NortHingResult<()> {
        tokio::select! {
            _ = cancel_token.cancelled() => Err(NortHingError::Cancelled("Execution cancelled".to_string())),
            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => Ok(()),
        }
    }

    /// Check if dialog turn is still active (used to detect cancellation)
    pub fn has_active_dialog_turn(&self, dialog_turn_id: &str) -> bool {
        self.cancellation_tokens.contains_key(dialog_turn_id)
    }

    /// Check if dialog turn cancellation has been requested.
    pub fn is_dialog_turn_cancelled(&self, dialog_turn_id: &str) -> bool {
        self.cancellation_tokens
            .get(dialog_turn_id)
            .is_some_and(|token| token.is_cancelled())
    }

    /// Register cancellation token (for external control, e.g., execute_subagent)
    pub fn register_cancel_token(&self, dialog_turn_id: &str, token: CancellationToken) {
        self.cancellation_tokens.insert(dialog_turn_id.to_string(), token);
    }

    /// Return a clone of the cancellation token registered for a dialog turn.
    pub fn cancel_token_for_dialog_turn(&self, dialog_turn_id: &str) -> Option<CancellationToken> {
        self.cancellation_tokens.get(dialog_turn_id).map(|entry| entry.clone())
    }

    /// Cancel dialog turn (using dialog_turn_id)
    pub async fn cancel_dialog_turn(&self, dialog_turn_id: &str) -> NortHingResult<()> {
        debug!("Cancelling dialog turn: dialog_turn_id={}", dialog_turn_id);

        if let Some(token) = self.cancellation_tokens.get(dialog_turn_id).map(|entry| entry.clone()) {
            debug!("Found cancel token, triggering cancellation");
            token.cancel();
            debug!("Cancel token triggered");
        } else {
            debug!("Cancel token not found (dialog may have completed or not started)");
        }

        Ok(())
    }

    /// Cleanup dialog turn token (called on normal completion)
    pub async fn cleanup_dialog_turn(&self, dialog_turn_id: &str) {
        if self.cancellation_tokens.remove(dialog_turn_id).is_some() {
            debug!("Cleaned up cancel token: dialog_turn_id={}", dialog_turn_id);
        }
    }

    /// Emit event (sibling-visible helper used by `rexec_run.rs`)
    pub(in crate::agentic::execution) async fn emit_event(&self, event: AgenticEvent, priority: EventPriority) {
        let _ = self.event_queue.enqueue(event, Some(priority)).await;
    }
}
