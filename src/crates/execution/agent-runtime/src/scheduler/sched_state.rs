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

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_runtime_ports::{
        AgentSubmissionSource, AgentSessionReplyRoute, DialogQueuePriority, DialogRoundInjectionSource,
        DialogSubmissionPolicy, RoundInjection, RoundInjectionKind, RoundInjectionTarget,
    };
    use std::time::SystemTime;

    fn make_round_injection(
        id: &str,
        target: RoundInjectionTarget,
    ) -> RoundInjection {
        RoundInjection {
            id: id.to_string(),
            kind: RoundInjectionKind::UserSteering,
            target,
            content: format!("content-{}", id),
            display_content: format!("display-{}", id),
            created_at: SystemTime::now(),
        }
    }

    fn agent_session_policy_with_route(
        _source_session_id: &str,
    ) -> DialogSubmissionPolicy {
        DialogSubmissionPolicy::new(
            AgentSubmissionSource::AgentSession,
            DialogQueuePriority::Low,
            true,
        )
    }

    fn desktop_ui_policy() -> DialogSubmissionPolicy {
        DialogSubmissionPolicy::new(
            AgentSubmissionSource::DesktopUi,
            DialogQueuePriority::Normal,
            false,
        )
    }

    // ── DialogTurnQueue ─────────────────────────────────────────────────────

    #[test]
    fn queue_enqueue_orders_by_priority_then_fifo() {
        let queue = DialogTurnQueue::<String>::with_max_depth(10);
        let sid = "s1";

        queue.enqueue(sid, "low-a".into(), DialogQueuePriority::Low,).unwrap();
        queue.enqueue(sid, "high-x".into(), DialogQueuePriority::High).unwrap();
        queue.enqueue(sid, "normal-y".into(), DialogQueuePriority::Normal).unwrap();
        queue.enqueue(sid, "low-b".into(), DialogQueuePriority::Low,).unwrap();
        queue.enqueue(sid, "normal-z".into(), DialogQueuePriority::Normal).unwrap();

        // High comes first (priority 2), then Normal FIFO (y before z), then Low FIFO (a before b)
        assert_eq!(queue.dequeue_next(sid), Some("high-x".into()));
        assert_eq!(queue.dequeue_next(sid), Some("normal-y".into()));
        assert_eq!(queue.dequeue_next(sid), Some("normal-z".into()));
        assert_eq!(queue.dequeue_next(sid), Some("low-a".into()));
        assert_eq!(queue.dequeue_next(sid), Some("low-b".into()));
        assert_eq!(queue.dequeue_next(sid), None);
    }

    #[test]
    fn queue_full_returns_error_and_preserves_existing_entries() {
        let queue = DialogTurnQueue::<String>::with_max_depth(2);
        let sid = "s1";

        assert_eq!(queue.enqueue(sid, "a".into(), DialogQueuePriority::Normal).unwrap(), 1);
        assert_eq!(queue.enqueue(sid, "b".into(), DialogQueuePriority::Normal).unwrap(), 2);

        // Third item is rejected
        let err = queue.enqueue(sid, "c".into(), DialogQueuePriority::Normal).unwrap_err();
        assert!(matches!(err, DialogTurnQueueError::Full { max_depth: 2, .. }));

        // Depth unchanged, order unchanged
        assert_eq!(queue.depth(sid), 2);
        assert_eq!(queue.dequeue_next(sid), Some("a".into()));
        assert_eq!(queue.dequeue_next(sid), Some("b".into()));
        assert_eq!(queue.dequeue_next(sid), None);
    }

    #[test]
    fn queue_dequeue_missing_session_returns_none_and_clear_reports_drained_count() {
        let queue = DialogTurnQueue::<String>::with_max_depth(10);

        // Unknown session
        assert_eq!(queue.dequeue_next("unknown"), None);
        assert_eq!(queue.depth("unknown"), 0);

        // Enqueue two, clear, verify drained count and zero depth
        let sid = "s1";
        queue.enqueue(sid, "x".into(), DialogQueuePriority::Normal).unwrap();
        queue.enqueue(sid, "y".into(), DialogQueuePriority::Normal).unwrap();
        assert_eq!(queue.clear(sid), 2);
        assert_eq!(queue.depth(sid), 0);
        assert_eq!(queue.dequeue_next(sid), None);
    }

    #[test]
    fn queue_requeue_front_bypasses_depth_check_and_takes_head() {
        let queue = DialogTurnQueue::<String>::with_max_depth(2);
        let sid = "s1";

        // Fill to max
        queue.enqueue(sid, "first".into(), DialogQueuePriority::Normal).unwrap();
        queue.enqueue(sid, "second".into(), DialogQueuePriority::Normal).unwrap();
        assert_eq!(queue.depth(sid), 2);

        // requeue_front bypasses the max_depth guard
        queue.requeue_front(sid, "requeued".into(), DialogQueuePriority::High);
        assert_eq!(queue.depth(sid), 3); // max+1

        // Requeued item comes out first
        assert_eq!(queue.dequeue_next(sid), Some("requeued".into()));
        assert_eq!(queue.dequeue_next(sid), Some("first".into()));
        assert_eq!(queue.dequeue_next(sid), Some("second".into()));
        assert_eq!(queue.dequeue_next(sid), None);
    }

    #[test]
    fn queue_default_max_depth_matches_product_constant() {
        assert_eq!(
            DialogTurnQueue::<String>::default().max_depth(),
            DEFAULT_MAX_DIALOG_QUEUE_DEPTH
        );
        assert_eq!(DEFAULT_MAX_DIALOG_QUEUE_DEPTH, 20);
    }

    // ── SessionRoundInjectionBuffer ─────────────────────────────────────────

    #[test]
    fn injection_buffer_drain_takes_exact_match_and_current_running_only() {
        let buf = SessionRoundInjectionBuffer::default();
        let sid = "s1";

        buf.push(sid, make_round_injection("t1-exact", RoundInjectionTarget::ExactTurn("t1".into())));
        buf.push(sid, make_round_injection("current", RoundInjectionTarget::CurrentRunningTurn));
        buf.push(sid, make_round_injection("t2-exact", RoundInjectionTarget::ExactTurn("t2".into())));

        // drain for t1 returns the t1 ExactTurn and the CurrentRunningTurn (2 items)
        let drained = buf.drain_for_turn(sid, "t1");
        assert_eq!(drained.len(), 2);
        let ids: Vec<_> = drained.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"t1-exact"));
        assert!(ids.contains(&"current"));

        // t2 ExactTurn is retained
        assert_eq!(buf.pending_count(sid), 1);
        assert!(buf.has_pending_for_turn(sid, "t2"));
    }

    #[test]
    fn injection_buffer_drain_missing_session_returns_empty() {
        let buf = SessionRoundInjectionBuffer::default();
        assert!(buf.drain_for_turn("unknown", "any-turn").is_empty());
        assert_eq!(buf.pending_count("unknown"), 0);
    }

    #[test]
    fn injection_buffer_has_pending_current_running_counts_for_any_turn() {
        let buf = SessionRoundInjectionBuffer::default();
        let sid = "s1";

        buf.push(sid, make_round_injection("cur", RoundInjectionTarget::CurrentRunningTurn));

        // CurrentRunningTurn is pending for any turn_id (including non-matching exact turns)
        assert!(buf.has_pending_for_turn(sid, "turn-a"));
        assert!(buf.has_pending_for_turn(sid, "turn-b"));
        // unknown-session here is a turn_id (not session_id), and CurrentRunningTurn matches any
        assert!(buf.has_pending_for_turn(sid, "any-turn-name"));
        // But a truly absent session returns false
        assert!(!buf.has_pending_for_turn("absent-session", "any-turn"));
    }

    #[test]
    fn injection_buffer_clear_drops_all_and_trait_delegation_works() {
        let sid = "s1";

        // Part 1: clear via concrete type
        let buf = SessionRoundInjectionBuffer::default();
        buf.push(sid, make_round_injection("a", RoundInjectionTarget::ExactTurn("t1".into())));
        buf.push(sid, make_round_injection("b", RoundInjectionTarget::CurrentRunningTurn));
        assert_eq!(buf.pending_count(sid), 2);
        buf.clear(sid);
        assert_eq!(buf.pending_count(sid), 0);
        assert!(!buf.has_pending_for_turn(sid, "any-turn"));

        // Part 2: trait delegation via Arc<dyn DialogRoundInjectionSource>
        let buf2 = SessionRoundInjectionBuffer::default();
        buf2.push(sid, make_round_injection("x", RoundInjectionTarget::ExactTurn("t2".into())));
        buf2.push(sid, make_round_injection("y", RoundInjectionTarget::CurrentRunningTurn));
        // Verify concrete pending_count
        assert_eq!(buf2.pending_count(sid), 2);
        // Verify has_pending and take_pending work through the trait
        let source: Arc<dyn DialogRoundInjectionSource> = Arc::new(buf2);
        assert!(source.has_pending(sid, "any-turn"));
        let taken = source.take_pending(sid, "any-turn");
        // drain_for_turn("s1", "any-turn") with [ExactTurn("t2"), CurrentRunningTurn]
        // should take CurrentRunningTurn (always) = 1 item
        assert_eq!(taken.len(), 1);
        // After drain, subsequent calls return empty
        assert!(!source.has_pending(sid, "any-turn"));
        assert!(source.take_pending(sid, "any-turn").is_empty());
    }

    // ── ActiveDialogTurnStore ────────────────────────────────────────────────

    #[test]
    fn active_turn_store_insert_contains_remove_roundtrip() {
        let store = ActiveDialogTurnStore::default();
        let sid = "s1";

        assert!(!store.contains(sid));

        let turn = ActiveDialogTurn::new(
            "turn-1".into(),
            None,
            "deep-review".into(),
            "user input".into(),
            None,
            desktop_ui_policy(),
            None,
        );
        store.insert(sid, turn.clone());

        assert!(store.contains(sid));
        let removed = store.remove(sid);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().turn_id(), "turn-1");
        assert!(!store.contains(sid));
        assert!(store.remove(sid).is_none());
    }

    #[test]
    fn active_turn_store_suppression_key_only_for_suppressed_requester() {
        let store = ActiveDialogTurnStore::default();
        let target_sid = "target-session";
        let suppressed_requester = "requester-a";
        let non_suppressed_requester = "requester-b";

        // Build a turn whose reply_route.source_session_id == suppressed_requester
        let route = AgentSessionReplyRoute {
            source_session_id: suppressed_requester.to_string(),
            source_workspace_path: "/ws".to_string(),
        };
        let policy = agent_session_policy_with_route(suppressed_requester);
        let turn = ActiveDialogTurn::new(
            "turn-x".into(),
            None,
            "agent".into(),
            "input".into(),
            None,
            policy,
            Some(route),
        );
        store.insert(target_sid, turn);

        // suppressed_requester should get Some((target_sid, turn_id))
        let key = store.suppression_key_for_requester(target_sid, suppressed_requester);
        assert!(key.is_some());
        let (sid, tid) = key.unwrap();
        assert_eq!(sid, target_sid);
        assert_eq!(tid, "turn-x");

        // non-suppressed requester (different session, or not AgentSession source)
        // should return None
        let key2 = store.suppression_key_for_requester(target_sid, non_suppressed_requester);
        assert!(key2.is_none());
    }

    // ── DialogReplySuppressionSet ────────────────────────────────────────────

    #[test]
    fn suppression_set_take_is_one_shot_and_clear_removes() {
        let set = DialogReplySuppressionSet::default();
        let sid = "s1";
        let tid = "turn-1";

        set.mark(sid, tid);

        // First take succeeds (one-shot)
        assert!(set.take(sid, tid));
        assert!(!set.take(sid, tid)); // second returns false

        // clear also removes
        set.mark(sid, tid);
        set.clear(sid, tid);
        assert!(!set.take(sid, tid));
    }

    // ── SessionAbortFlags ────────────────────────────────────────────────────

    #[test]
    fn abort_flags_mark_contains_clear_per_session() {
        let flags = SessionAbortFlags::default();
        let sid = "s1";

        assert!(!flags.contains(sid));

        flags.mark(sid);
        assert!(flags.contains(sid));

        flags.clear(sid);
        assert!(!flags.contains(sid));
    }

    // ── NoopDialogRoundInjectionSource ─────────────────────────────────────

    #[test]
    fn noop_injection_source_is_always_empty() {
        let noop = NoopDialogRoundInjectionSource;

        assert!(!noop.has_pending("any-session", "any-turn"));
        assert!(noop.take_pending("any-session", "any-turn").is_empty());

        // Trait object path
        let source: Arc<dyn DialogRoundInjectionSource> = Arc::new(noop);
        assert!(!source.has_pending("x", "y"));
        assert!(source.take_pending("x", "y").is_empty());
    }
}
