//! In-process spawn adapter (tokio current-handle).
//!
//! Pattern source: `.agents/reference/actor/04-coordinator-spawn-pattern.rs`
//! (Pattern 5 — `tokio::spawn` on the current runtime).
//!
//! ## Phase 1 status
//!
//! The adapter is **structurally** complete (it owns a `tokio::runtime::Handle`
//! and can spawn a closure on it) but does **not** wire to any actor or
//! dispatcher body — those land in Phase 2. The `spawn_actor` and
//! `spawn_dispatch` methods are typed so callers can pass them around in
//! Phase 1, but they return a placeholder handle and log a debug message.
//!
//! Callers MUST NOT rely on this adapter for real work until Phase 2 lands.

use std::future::Future;
use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// In-process spawn adapter.
///
/// Construct once at app start (or pass an existing `Handle` via
/// [`TokioSpawnAdapter::with_handle`]) and share via `Arc`.
///
/// The struct is `Clone + Send + Sync` — cloning shares the same underlying
/// `Handle` via `Arc` so the runtime is never duplicated.
#[derive(Clone)]
pub struct TokioSpawnAdapter {
    handle: Arc<Handle>,
}

impl std::fmt::Debug for TokioSpawnAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioSpawnAdapter")
            .field("handle", &"<tokio::runtime::Handle>")
            .finish()
    }
}

impl Default for TokioSpawnAdapter {
    /// Builds the adapter against the **current** tokio runtime handle, if one
    /// is active on this thread. Returns `None` from `try_default` if no
    /// runtime is active — callers in that situation should use
    /// `with_handle` with an explicit handle.
    fn default() -> Self {
        Self::try_default().expect(
            "TokioSpawnAdapter::default() called outside a tokio runtime; \
             use TokioSpawnAdapter::with_handle(explicit_handle) instead",
        )
    }
}

impl TokioSpawnAdapter {
    /// Capture the current tokio runtime's handle.
    pub fn try_default() -> Option<Self> {
        Handle::try_current().ok().map(|h| Self { handle: Arc::new(h) })
    }

    /// Wrap an explicit handle (useful for tests or for callers that own the
    /// runtime separately).
    pub fn with_handle(handle: Handle) -> Self {
        Self {
            handle: Arc::new(handle),
        }
    }

    /// Spawn an actor on the current runtime. Phase 1 is a no-op stub —
    /// the `Future` is spawned and immediately cancelled. The returned
    /// [`JoinHandle`] can be awaited to confirm the (trivial) completion.
    ///
    /// The real body (calling the actor's `tick` on a periodic schedule with
    /// per-tick timeout) lands in Phase 2.
    pub fn spawn_actor<F>(&self, future: F) -> JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handle.spawn(future)
    }

    /// Spawn a one-shot dispatch on the current runtime. The `cancel`
    /// token is wired so the dispatcher implementation can observe it on
    /// every blocking call (per spec invariant #2 on `SkillActor`, which
    /// extends to the dispatcher by symmetry).
    ///
    /// The Phase 1 body is intentionally minimal — see [`spawn_actor`].
    pub fn spawn_dispatch<F>(&self, future: F, cancel: CancellationToken) -> JoinHandle<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handle.spawn(async move {
            tokio::select! {
                _ = future => {}
                _ = cancel.cancelled() => {}
            }
        })
    }

    /// Returns the inner handle for callers that need direct spawn access.
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adapter_captures_current_handle() {
        let adapter = TokioSpawnAdapter::try_default().expect("tokio runtime is active");
        let cloned = adapter.clone();
        // Cloning shares the same handle — both adapters refer to the same runtime.
        assert!(Arc::ptr_eq(&adapter.handle, &cloned.handle));
    }

    #[tokio::test]
    async fn spawn_actor_runs_on_current_runtime() {
        let adapter = TokioSpawnAdapter::try_default().expect("tokio runtime is active");
        let handle = adapter.spawn_actor(async {
            // If we got here, spawn worked.
        });
        handle.await.expect("actor task must complete cleanly");
    }

    #[tokio::test]
    async fn spawn_dispatch_observes_cancel() {
        let adapter = TokioSpawnAdapter::try_default().expect("tokio runtime is active");
        let cancel = CancellationToken::new();
        let handle = adapter.spawn_dispatch(
            async {
                // Long-running future that should be cancelled.
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            },
            cancel.clone(),
        );
        // Cancel immediately; the spawned task must observe it and exit.
        cancel.cancel();
        // Bound the wait so the test fails loudly if cancel is broken.
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "spawn_dispatch did not observe cancel within 2s");
    }
}
