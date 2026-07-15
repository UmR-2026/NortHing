// REFERENCE — extracted from
//   src/crates/execution/agent-runtime/src/scheduler.rs (lines 100-220)
// Last synced: 2813b36 (v3-restructure)
// The DashMap-keyed registry pattern used throughout the scheduler.
// Borrow this shape for the actor's per-session state, per-actor-id
// state, and per-turn state.

use std::collections::VecDeque;

use dashmap::DashMap;

use crate::contracts::runtime_ports::DialogQueuePriority;

// ═══════════════════════════════════════════════════════════════════════
// Pattern 1: per-key store with simple insert/remove/contains
// (ActiveDialogTurnStore at scheduler.rs:101-135)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Default)]
pub struct ActiveDialogTurnStore {
    inner: dashmap::DashMap<String, ActiveDialogTurn>,
}

pub struct ActiveDialogTurn { _private: () }

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
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 2: composite-key store (session_id, turn_id) -> ()
// (DialogReplySuppressionSet at scheduler.rs:137-158)
// ═══════════════════════════════════════════════════════════════════════

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
        self.inner.remove(&(session_id.to_string(), turn_id.to_string())).is_some()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 3: single-key flag store
// (SessionAbortFlags at scheduler.rs:160-177)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Default)]
pub struct SessionAbortFlags {
    inner: dashmap::DashMap<String, ()>,
}

impl SessionAbortFlags {
    pub fn mark(&self, session_id: &str) { self.inner.insert(session_id.to_string(), ()); }
    pub fn clear(&self, session_id: &str) { self.inner.remove(session_id); }
    pub fn contains(&self, session_id: &str) -> bool { self.inner.contains_key(session_id) }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 4: per-key priority queue
// (DialogTurnQueue at scheduler.rs:209-220+)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct QueuedDialogTurn<T> {
    priority: DialogQueuePriority,
    turn: T,
}

pub struct DialogTurnQueue<T> {
    max_depth: usize,
    inner: dashmap::DashMap<String, VecDeque<QueuedDialogTurn<T>>>,
}

impl<T> Default for DialogTurnQueue<T> {
    fn default() -> Self { Self::with_max_depth(64) } // example default
}

impl<T> DialogTurnQueue<T> {
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self { max_depth, inner: DashMap::new() }
    }
    // ★ Note: enqueue/dequeue priority-aware logic in scheduler.rs:220+
    // uses `BinaryHeap<Reverse<...>>` for the priority ordering, then
    // drains to a VecDeque for FIFO within a priority band. See source.
}

// ═══════════════════════════════════════════════════════════════════════
// Why DashMap (project conventions)
// ═══════════════════════════════════════════════════════════════════════
//
// DashMap is the project's idiomatic concurrent map. Rationale:
//   - Sharded internally for high-concurrency read/write workloads.
//   - No async API (use `try_get` / `entry` for sync access; for
//     async contexts, hold the guard for the smallest possible scope).
//   - `entry().or_insert_with(...)` is the right pattern for "get or create".
//
// DO NOT introduce a new concurrent map type (Arc<Mutex<HashMap>> is
// fine for small/rare cases, but a global registry is always DashMap).
