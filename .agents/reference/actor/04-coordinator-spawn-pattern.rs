// REFERENCE — extracted from
//   src/crates/assembly/core/src/agentic/coordination/coordinator.rs (lines 1-100, 301-360, 431-528, 2378, 3234, 3276, 4482, 5316)
// Last synced: 2813b36 (v3-restructure)
// These are the load-bearing `tokio::spawn` patterns from the existing
// coordinator. The actor/dispatcher design should borrow the plumbing
// style (mpsc + watch + CancellationToken + DashMap) but NOT the
// structural shape (OnceLock injection, 4-way facade).

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::agentic::coordination::turn_outcome::TurnOutcome;
use crate::agentic::execution::execution_engine::ExecutionEngine;

// ═══════════════════════════════════════════════════════════════════════
// Pattern 1: mpsc back-channel from spawned task to scheduler
// (coordinator.rs:518-528)
// ═══════════════════════════════════════════════════════════════════════

/// The scheduler → coordinator mpsc is wired via `OnceLock`:
///   - Coordinator is constructed first (no scheduler yet).
///   - Scheduler is constructed and calls `coordinator.set_scheduler_notify_tx(tx)`.
///   - From then on, coordinator uses `scheduler_notify_tx.get().unwrap().send(...)`.
///
/// ★ This is a "wiring at startup" pattern. The actor/dispatcher design
/// should NOT copy it for the actor registry — wire at construction
/// instead. Use this pattern only when two independent subsystems must
/// come up in a specific order (e.g. coordinator before scheduler).
pub struct CoordinatorWithSchedulerWire {
    scheduler_notify_tx: std::sync::OnceLock<mpsc::Sender<(String, TurnOutcome)>>,
}

impl CoordinatorWithSchedulerWire {
    pub fn set_scheduler_notify_tx(&self, tx: mpsc::Sender<(String, TurnOutcome)>) {
        let _ = self.scheduler_notify_tx.set(tx);
    }

    /// Called from a spawned task. The `get().unwrap()` is safe because
    /// `set_scheduler_notify_tx` is called during startup, before any
    /// turn is started.
    pub async fn notify_outcome(&self, session_id: String, outcome: TurnOutcome) {
        let tx = self.scheduler_notify_tx.get().expect("scheduler not wired");
        let _ = tx.send((session_id, outcome)).await;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 2: spawned turn with cancellation and timeout
// (coordinator.rs:301, 360, 431-502)
// ═══════════════════════════════════════════════════════════════════════

/// A spawned dialog turn. The actor design should follow the same shape:
///   1. Create a CancellationToken.
///   2. Create a watch::Sender<Option<Instant>> for dynamic timeout.
///   3. Spawn the turn, passing clones of the cancel token and the watch.
///   4. Register the cancel token in a DashMap keyed by turn id (so
///      external callers can cancel).
///   5. On completion, remove from the DashMap and emit the outcome.
pub struct SpawnedTurn {
    pub cancel: CancellationToken,
    pub timeout: watch::Sender<Option<std::time::Instant>>,
    pub join: tokio::task::JoinHandle<NortHingResult<TurnOutcome>>,
}

pub async fn spawn_turn(
    engine: Arc<ExecutionEngine>,
    session_id: String,
    turn_id: String,
    cancel: CancellationToken,
) -> SpawnedTurn {
    let (timeout_tx, timeout_rx) = watch::channel(None);
    let cancel_for_task = cancel.clone();
    let engine_for_task = engine.clone();
    let join = tokio::spawn(async move {
        // The actual work would call into ExecutionEngine::execute_dialog_turn.
        // For brevity, this stub returns a "started" outcome.
        let _ = (engine_for_task, session_id, turn_id, cancel_for_task, timeout_rx);
        Ok(TurnOutcome::Completed)
    });
    SpawnedTurn { cancel, timeout: timeout_tx, join }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 3: DashMap-keyed registry of cancel tokens
// (coordinator.rs:511, active_subagent_executions: Arc<DashMap<String, CancellationToken>>)
// ═══════════════════════════════════════════════════════════════════════

/// Registry of in-flight actors / subagent executions, keyed by id.
/// Lets the runtime cancel a specific actor by id (e.g. on user
/// "stop this" action) without having to track JoinHandles everywhere.
pub struct ActiveExecutions {
    inner: Arc<DashMap<String, CancellationToken>>,
}

impl ActiveExecutions {
    pub fn new() -> Self { Self { inner: Arc::new(DashMap::new()) } }

    pub fn register(&self, id: String, cancel: CancellationToken) {
        self.inner.insert(id, cancel);
    }

    pub fn cancel(&self, id: &str) -> bool {
        if let Some(entry) = self.inner.get(id) {
            entry.value().cancel();
            true
        } else {
            false
        }
    }

    pub fn unregister(&self, id: &str) -> Option<CancellationToken> {
        self.inner.remove(id).map(|(_, c)| c)
    }

    pub fn is_active(&self, id: &str) -> bool { self.inner.contains_key(id) }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 4: Semaphore-based concurrency limiter per profile
// (coordinator.rs:512, subagent_profile_concurrency_limiters: Arc<RwLock<HashMap<usize, SubagentConcurrencyLimiter>>>)
// ═══════════════════════════════════════════════════════════════════════

/// Per-profile concurrency limiter. Each profile gets its own semaphore;
/// the holder of an OwnedSemaphorePermit is allowed to run.
pub struct ProfileConcurrencyLimiters {
    limiters: Arc<RwLock<std::collections::HashMap<usize, Arc<Semaphore>>>>,
}

impl ProfileConcurrencyLimiters {
    pub fn new() -> Self { Self { limiters: Arc::new(RwLock::new(Default::default())) } }

    pub async fn get_or_create(&self, profile_id: usize, max: usize) -> Arc<Semaphore> {
        {
            let limiters = self.limiters.read().await;
            if let Some(s) = limiters.get(&profile_id) { return s.clone(); }
        }
        let mut limiters = self.limiters.write().await;
        limiters.entry(profile_id).or_insert_with(|| Arc::new(Semaphore::new(max))).clone()
    }

    pub async fn acquire(&self, profile_id: usize, max: usize) -> OwnedSemaphorePermit {
        self.get_or_create(profile_id, max).await.acquire_owned().await.expect("semaphore closed")
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern 5: spawn-from-callback context (coordinator.rs:5316)
// ═══════════════════════════════════════════════════════════════════════

/// Inside `start_dialog_turn_internal`, the coordinator spawns the turn
/// onto its own `tokio::runtime::Handle::current()`. The actor design
/// should do the same — actors run on the same runtime as the
/// coordinator, not on a separate one. This is what makes
/// `CancellationToken` work across the boundary.
pub fn spawn_actor_on_current_runtime<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}
