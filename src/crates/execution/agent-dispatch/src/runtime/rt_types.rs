//! `ActorRuntime` core types — `ActorHandle` (public runtime handle) and
//! `ActorRuntime` (private struct holding the actor registry + shared
//! port/telemetry/handle references).
//!
//! These are pure data + cheap impls; the bulk of the runtime logic
//! lives in sibling files:
//!
//! - [`super::rt_state`] — constructors and registry operations (stop,
//!   deregister, len/is_empty)
//! - [`super::rt_dispatch`] — actor spawn entry points
//!   (`spawn_actor`, `spawn_one_shot`, `spawn_long_running`)
//! - [`super::rt_handlers`] — per-tick helpers (`run_one_tick`) and
//!   the `#[cfg(test)] mod tests`
//!
//! Fields are `pub(super)` so the sibling impls (all living under the
//! `runtime` module) can read/write them directly without round-trip
//! accessor methods. External callers go through the `pub` API on
//! `ActorRuntime` defined in [`super::rt_state`] /
//! [`super::rt_dispatch`].

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio_util::sync::CancellationToken;

use crate::telemetry::TelemetrySink;
use northhing_runtime_ports::ToolDispatcherPort;

/// Public handle to a running actor. Lets the runtime (or a test)
/// stop the actor cleanly.
///
/// `ActorHandle` is `Clone`-able: cloning shares the underlying
/// `CancellationToken` + `Notify` so multiple owners can request the
/// same actor's shutdown or wait for completion without race. The
/// spawned task calls `notify.notify_waiters()` on completion, after
/// which every clone's `await_join` resolves.
///
/// Design note: a `JoinHandle` is not `Clone` and doesn't survive an
/// `Arc` (it owns the join token). `tokio::sync::Notify` is the
/// idiomatic replacement for cross-handle completion signaling —
/// `notify_one()` from the task, `notified().await` from each handle.
#[derive(Clone)]
pub struct ActorHandle {
    /// Stable id (matches the `SkillActor::id()` of the running actor).
    pub(super) id: String,
    pub(super) cancel: CancellationToken,
    /// Fires when the actor's spawned task finishes (clean exit, panic,
    /// or `notify.notify_waiters()` from inside the task wrapper).
    pub(super) notify: Arc<tokio::sync::Notify>,
}

impl std::fmt::Debug for ActorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorHandle")
            .field("id", &self.id)
            .field("cancel", &"<CancellationToken>")
            .field("notify", &"<tokio::sync::Notify>")
            .finish()
    }
}

impl ActorHandle {
    /// Construct a new `ActorHandle` from the running pieces.
    ///
    /// Used by `ActorRuntime::spawn_actor`; not normally constructed by
    /// callers (the runtime owns the lifecycle).
    pub fn new(id: String, cancel: CancellationToken, notify: Arc<tokio::sync::Notify>) -> Self {
        Self { id, cancel, notify }
    }

    /// Stop the actor (sets cancel). Does **not** wait for join —
    /// call `await_join()` for that.
    pub fn stop(&self) {
        self.cancel.cancel();
    }

    /// Wait for the actor's task to finish. Returns `Ok(())` when the
    /// spawned task signals completion (via `notify.notify_waiters()`).
    ///
    /// Note: does NOT distinguish "task panicked" from "task completed
    /// cleanly" — the notify abstraction only signals completion. If a
    /// caller needs the `JoinError` outcome (panic / abort), they should
    /// own the unique `JoinHandle` from `tokio::spawn` directly instead of
    /// going through the handle API. For MVP purposes the distinction
    /// doesn't matter — `run_one_tick` already emits an `ActorError`
    /// telemetry event when the tick returns `Err`.
    pub async fn await_join(&self) -> Result<(), tokio::task::JoinError> {
        self.notify.notified().await;
        Ok(())
    }

    /// Wait for the actor's cancel token to fire (or for it to already
    /// be cancelled). Does NOT own the join — clones can all call this
    /// concurrently. Useful for code that just wants to know "is the
    /// actor done shutting down" without racing on the `JoinHandle`.
    pub async fn wait_cancelled(&self) {
        self.cancel.cancelled().await
    }

    /// Whether the actor has been asked to stop (cancel was called).
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// The actor's id. Mirrors the `SkillActor::id()` value.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Owns the registry of running actors. Construct once at app start;
/// share via `Arc`.
///
/// The runtime is **not** `Send`-bound by itself — `spawn_actor` requires
/// a tokio runtime handle, which is captured at construction time via
/// [`ActorRuntime::with_handle`]. The default constructor ([`ActorRuntime::new`])
/// captures the current tokio runtime's handle; if no runtime is
/// active, it panics.
pub struct ActorRuntime {
    /// Registry of currently-running actors. Keyed by `SkillActor::id()`.
    pub(super) actors: DashMap<String, ActorHandle>,
    /// Tool dispatcher shared by all actors (per-tick `ActorContext`).
    pub(super) dispatcher: Arc<dyn ToolDispatcherPort>,
    /// Telemetry sink shared by all actors.
    pub(super) telemetry: Arc<dyn TelemetrySink>,
    /// Default per-tick timeout (spec suggests 30s; overridable per actor
    /// in future phases via a custom `ActorSchedule`).
    pub(super) default_tick_timeout: Duration,
    /// The tokio runtime handle used to spawn actor tasks.
    pub(super) handle: Arc<tokio::runtime::Handle>,
}

impl std::fmt::Debug for ActorRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorRuntime")
            .field("actors", &self.actors.len())
            .field("default_tick_timeout", &self.default_tick_timeout)
            .finish()
    }
}
