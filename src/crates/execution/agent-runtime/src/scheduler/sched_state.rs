//! Scheduler owner state stores and round-injection sources.
//!
//! Mutable, DashMap-backed state used by the concrete scheduler. This file
//! owns:
//!
//! - The three per-session state stores keyed by `session_id`:
//!   [`ActiveDialogTurnStore`], [`DialogReplySuppressionSet`], and
//!   [`SessionAbortFlags`].
//! - The per-session priority queue [`DialogTurnQueue`] used to throttle
//!   incoming dialog turns to the product max-depth policy.
//! - Round-injection sources: the always-empty [`NoopDialogRoundInjectionSource`]
//!   used in tests, the [`DialogRoundInjectionInterrupt`] wrapper that asks
//!   a shared source whether a given session/turn should be interrupted,
//!   and the full [`SessionRoundInjectionBuffer`] that both buffers and
//!   drains round injections for a session.
//!
//! All types are publicly re-exported from the facade
//! (`crate::scheduler`) so external import paths are unchanged.
//!
//! Sub-domain layout:
//! - `scheduler.rs` (facade)  — module wiring, `pub use` re-exports, tests.
//! - `sched_types.rs`         — data types + inherent impls.
//! - `sched_state.rs`         — state stores + injection sources (this file).
//! - `sched_filter.rs`        — pure decide / resolve functions.

use super::sched_types::{ActiveDialogTurn, DialogTurnQueueError, DEFAULT_MAX_DIALOG_QUEUE_DEPTH};
use northhing_runtime_ports::{DialogQueuePriority, DialogRoundInjectionSource, RoundInjection, RoundInjectionTarget};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

/// Per-session map of currently active dialog turns.
#[derive(Debug, Default)]
pub struct ActiveDialogTurnStore {
    inner: dashmap::DashMap<String, ActiveDialogTurn>,
}

impl ActiveDialogTurnStore {
    pub fn insert(&self, session_id: &str, turn: ActiveDialogTurn) {
        self.inner.insert(session_id.to_string(), turn);
    }

    pub fn remove(&self, session_id: &str) -> Option<ActiveDialogTurn> {
        self.inner.remove(session_id).map(|(_, turn)| turn)
    }

    pub fn contains(&self, session_id: &str) -> bool {
        self.inner.contains_key(session_id)
    }

    pub fn suppression_key_for_requester(
        &self,
        target_session_id: &str,
        requester_session_id: &str,
    ) -> Option<(String, String)> {
        self.inner.get(target_session_id).and_then(|active_turn| {
            active_turn
                .should_suppress_cancelled_reply_for_requester(requester_session_id)
                .then(|| (target_session_id.to_string(), active_turn.turn_id().to_string()))
        })
    }
}

/// Tracks `(session_id, turn_id)` pairs whose automated reply was suppressed.
#[derive(Debug, Default)]
pub struct DialogReplySuppressionSet {
    inner: dashmap::DashMap<(String, String), ()>,
}

impl DialogReplySuppressionSet {
    pub fn mark(&self, session_id: &str, turn_id: &str) {
        self.inner.insert((session_id.to_string(), turn_id.to_string()), ());
    }

    pub fn clear(&self, session_id: &str, turn_id: &str) {
        self.inner.remove(&(session_id.to_string(), turn_id.to_string()));
    }

    pub fn take(&self, session_id: &str, turn_id: &str) -> bool {
        self.inner
            .remove(&(session_id.to_string(), turn_id.to_string()))
            .is_some()
    }
}

/// Per-session flags signalling that an in-flight turn was aborted and the
/// concrete scheduler should not auto-dispatch the next queued turn.
#[derive(Debug, Default)]
pub struct SessionAbortFlags {
    inner: dashmap::DashMap<String, ()>,
}

impl SessionAbortFlags {
    pub fn mark(&self, session_id: &str) {
        self.inner.insert(session_id.to_string(), ());
    }

    pub fn clear(&self, session_id: &str) {
        self.inner.remove(session_id);
    }

    pub fn contains(&self, session_id: &str) -> bool {
        self.inner.contains_key(session_id)
    }
}

#[derive(Debug, Clone)]
struct QueuedDialogTurn<T> {
    priority: DialogQueuePriority,
    turn: T,
}

/// Per-session dialog-turn queue with product scheduler priority semantics.
#[derive(Debug)]
pub struct DialogTurnQueue<T> {
    max_depth: usize,
    inner: dashmap::DashMap<String, VecDeque<QueuedDialogTurn<T>>>,
}

impl<T> Default for DialogTurnQueue<T> {
    fn default() -> Self {
        Self::with_max_depth(DEFAULT_MAX_DIALOG_QUEUE_DEPTH)
    }
}

impl<T> DialogTurnQueue<T> {
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            inner: dashmap::DashMap::new(),
        }
    }

    pub const fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn depth(&self, session_id: &str) -> usize {
        self.inner.get(session_id).map(|q| q.len()).unwrap_or(0)
    }

    pub fn has_items(&self, session_id: &str) -> bool {
        self.depth(session_id) > 0
    }

    pub fn enqueue(
        &self,
        session_id: &str,
        turn: T,
        priority: DialogQueuePriority,
    ) -> Result<usize, DialogTurnQueueError> {
        let mut queue = self.inner.entry(session_id.to_string()).or_default();
        if queue.len() >= self.max_depth {
            return Err(DialogTurnQueueError::Full {
                session_id: session_id.to_string(),
                max_depth: self.max_depth,
            });
        }

        let queued = QueuedDialogTurn { priority, turn };
        let insert_at = queue.iter().position(|existing| existing.priority < queued.priority);
        if let Some(index) = insert_at {
            queue.insert(index, queued);
        } else {
            queue.push_back(queued);
        }

        Ok(queue.len())
    }

    pub fn clear(&self, session_id: &str) -> usize {
        self.inner.remove(session_id).map(|(_, queue)| queue.len()).unwrap_or(0)
    }

    pub fn dequeue_next(&self, session_id: &str) -> Option<T> {
        self.inner
            .get_mut(session_id)
            .and_then(|mut q| q.pop_front().map(|item| item.turn))
    }

    pub fn requeue_front(&self, session_id: &str, turn: T, priority: DialogQueuePriority) {
        self.inner
            .entry(session_id.to_string())
            .or_default()
            .push_front(QueuedDialogTurn { priority, turn });
    }
}

/// Used when no scheduler is wired (e.g. tests, isolated execution).
pub struct NoopDialogRoundInjectionSource;

impl DialogRoundInjectionSource for NoopDialogRoundInjectionSource {
    fn has_pending(&self, _session_id: &str, _turn_id: &str) -> bool {
        false
    }

    fn take_pending(&self, _session_id: &str, _turn_id: &str) -> Vec<RoundInjection> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct DialogRoundInjectionInterrupt {
    session_id: String,
    turn_id: String,
    source: Arc<dyn DialogRoundInjectionSource>,
}

impl fmt::Debug for DialogRoundInjectionInterrupt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DialogRoundInjectionInterrupt")
            .field("session_id", &self.session_id)
            .field("turn_id", &self.turn_id)
            .finish_non_exhaustive()
    }
}

impl DialogRoundInjectionInterrupt {
    pub fn new(session_id: String, turn_id: String, source: Arc<dyn DialogRoundInjectionSource>) -> Self {
        Self {
            session_id,
            turn_id,
            source,
        }
    }

    pub fn should_interrupt(&self) -> bool {
        self.source.has_pending(&self.session_id, &self.turn_id)
    }
}

/// Per-session FIFO buffer of round injections keyed by `session_id`.
#[derive(Debug, Default)]
pub struct SessionRoundInjectionBuffer {
    inner: dashmap::DashMap<String, Vec<RoundInjection>>,
}

impl SessionRoundInjectionBuffer {
    pub fn push(&self, session_id: &str, message: RoundInjection) {
        self.inner.entry(session_id.to_string()).or_default().push(message);
    }

    /// Drain all messages eligible for the currently running turn. Exact-turn
    /// injections that target a different turn are retained until the targeted
    /// turn consumes them or the session is cleared.
    pub fn drain_for_turn(&self, session_id: &str, turn_id: &str) -> Vec<RoundInjection> {
        let Some(mut entry) = self.inner.get_mut(session_id) else {
            return Vec::new();
        };
        let mut taken = Vec::new();
        let mut keep = Vec::new();
        for msg in entry.drain(..) {
            match &msg.target {
                RoundInjectionTarget::ExactTurn(target_turn_id) if target_turn_id == turn_id => {
                    taken.push(msg);
                }
                RoundInjectionTarget::CurrentRunningTurn => taken.push(msg),
                RoundInjectionTarget::ExactTurn(_) => keep.push(msg),
            }
        }
        *entry = keep;
        taken
    }

    pub fn has_pending_for_turn(&self, session_id: &str, turn_id: &str) -> bool {
        self.inner
            .get(session_id)
            .map(|entry| {
                entry.iter().any(|msg| match &msg.target {
                    RoundInjectionTarget::ExactTurn(target_turn_id) => target_turn_id == turn_id,
                    RoundInjectionTarget::CurrentRunningTurn => true,
                })
            })
            .unwrap_or(false)
    }

    /// Drop all messages for a session (e.g. session deleted or unrecoverable error).
    pub fn clear(&self, session_id: &str) {
        self.inner.remove(session_id);
    }

    pub fn pending_count(&self, session_id: &str) -> usize {
        self.inner.get(session_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl DialogRoundInjectionSource for SessionRoundInjectionBuffer {
    fn has_pending(&self, session_id: &str, turn_id: &str) -> bool {
        self.has_pending_for_turn(session_id, turn_id)
    }

    fn take_pending(&self, session_id: &str, turn_id: &str) -> Vec<RoundInjection> {
        self.drain_for_turn(session_id, turn_id)
    }
}
