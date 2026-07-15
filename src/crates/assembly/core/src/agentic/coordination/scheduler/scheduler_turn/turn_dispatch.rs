use super::super::scheduler_types::{DialogScheduler, DialogSteerOutcome, QueuedTurn};
use crate::agentic::core::SessionState;
use northhing_agent_runtime::scheduler::{resolve_dialog_steering_action, DialogSteeringAction};
use std::time::SystemTime;
use tracing::{debug, info, warn};
use uuid::Uuid;

impl DialogScheduler {
    /// Submit a user "steering" message into the currently running dialog turn.
    ///
    /// Unlike [`Self::submit`], this never starts or queues a new turn — it only buffers
    /// the message so the [`ExecutionEngine`](super::super::execution::ExecutionEngine)
    /// can inject it at the next model-round boundary. Errors:
    ///
    /// - Session is not currently `Processing` the requested `turn_id` (the targeted turn
    ///   already finished or never existed). Caller should fall back to `submit`.
    pub async fn submit_steering(
        &self,
        session_id: String,
        turn_id: String,
        content: String,
        display_content: Option<String>,
    ) -> Result<DialogSteerOutcome, String> {
        let active_turn_id = match self.session_manager.get_session(&session_id).map(|s| s.state.clone()) {
            Some(SessionState::Processing { current_turn_id, .. }) => Some(current_turn_id),
            _ => None,
        };

        let steering_id = Uuid::new_v4().to_string();
        match resolve_dialog_steering_action(
            active_turn_id.as_deref(),
            &session_id,
            &turn_id,
            content,
            display_content,
            steering_id,
            SystemTime::now(),
        ) {
            DialogSteeringAction::Reject { error } => {
                warn!(
                    "submit_steering rejected: target turn is not running: session_id={}, turn_id={}",
                    session_id, turn_id
                );
                Err(error)
            }
            DialogSteeringAction::Buffer { injection, outcome } => {
                self.round_injection_buffer.push(&session_id, injection);
                let DialogSteerOutcome::Buffered { steering_id, .. } = &outcome;
                info!(
                    "Steering message buffered: session_id={}, turn_id={}, steering_id={}, pending={}",
                    session_id,
                    turn_id,
                    steering_id,
                    self.round_injection_buffer.pending_count(&session_id)
                );

                Ok(outcome)
            }
        }
    }

    pub(crate) fn enqueue(&self, session_id: &str, queued_turn: QueuedTurn) -> Result<(), String> {
        let priority = queued_turn.policy.queue_priority;
        let new_len = match self.queues.enqueue(session_id, queued_turn, priority) {
            Ok(new_len) => new_len,
            Err(error) => {
                let max_depth = self.queues.max_depth();
                warn!(
                    "Queue full, rejecting message: session_id={}, max={}",
                    session_id, max_depth
                );
                return Err(error.to_string());
            }
        };

        debug!(
            "Message queued: session_id={}, queue_depth={}, priority={:?}",
            session_id, new_len, priority
        );
        Ok(())
    }

    pub(crate) fn clear_queue(&self, session_id: &str) {
        let count = self.queues.clear(session_id);
        if count > 0 {
            info!("Cleared {} queued messages: session_id={}", count, session_id);
        }
    }

    pub(crate) fn dequeue_next(&self, session_id: &str) -> Option<QueuedTurn> {
        self.queues.dequeue_next(session_id)
    }

    pub(crate) fn requeue_front(&self, session_id: &str, turn: QueuedTurn) {
        let priority = turn.policy.queue_priority;
        self.queues.requeue_front(session_id, turn, priority);
    }
}
