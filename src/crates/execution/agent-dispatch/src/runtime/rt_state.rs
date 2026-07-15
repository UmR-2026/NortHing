//! `ActorRuntime` constructors and registry operations.
//!
//! Split from the original monolithic `runtime.rs` so the lifecycle
//! primitives (new / with_handle / set_default_tick_timeout /
//! stop_actor / stop_all / deregister / len / is_empty) live in one
//! place; spawn entry points are in [`super::rt_dispatch`].
//!
//! Visibility: this file reads/writes the `pub(super)` fields declared
//! in [`super::rt_types`] (actors map, dispatcher, telemetry,
//! default_tick_timeout, handle). External callers only see the `pub`
//! methods on [`ActorRuntime`].

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use northhing_runtime_ports::ToolDispatcherPort;

use crate::telemetry::TelemetrySink;

use super::rt_types::ActorRuntime;

impl ActorRuntime {
    /// Construct the runtime against the **current** tokio runtime.
    /// Panics if no tokio runtime is active on this thread — callers
    /// in that situation should use [`ActorRuntime::with_handle`].
    pub fn new(dispatcher: Arc<dyn ToolDispatcherPort>, telemetry: Arc<dyn TelemetrySink>) -> Self {
        let handle = tokio::runtime::Handle::current();
        Self::with_handle(handle, dispatcher, telemetry)
    }

    /// Construct the runtime against an explicit tokio handle.
    pub fn with_handle(
        handle: tokio::runtime::Handle,
        dispatcher: Arc<dyn ToolDispatcherPort>,
        telemetry: Arc<dyn TelemetrySink>,
    ) -> Self {
        Self {
            actors: DashMap::new(),
            dispatcher,
            telemetry,
            default_tick_timeout: Duration::from_secs(30),
            handle: Arc::new(handle),
        }
    }

    /// Override the default per-tick timeout. Future phases may expose
    /// per-actor timeouts via `ActorSchedule::Periodic(Duration)`; for
    /// Phase 2 the runtime-wide default is the only knob.
    pub fn set_default_tick_timeout(&mut self, timeout: Duration) {
        self.default_tick_timeout = timeout;
    }

    /// How many actors are currently registered.
    pub fn len(&self) -> usize {
        self.actors.len()
    }

    /// Whether the registry is empty. Useful for shutdown cleanup.
    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

    /// Stop an actor by id. No-op if not running. The actor's task
    /// observes the cancel on its next tick; `ActorHandle::await_join`
    /// (returned from `spawn_actor`) is the way to wait for it to
    /// finish.
    pub fn stop_actor(&self, id: &str) {
        if let Some(handle) = self.actors.get(id) {
            handle.stop();
        }
    }

    /// Stop all actors. Used at shutdown — broadcasts cancel to every
    /// registered handle.
    pub fn stop_all(&self) {
        let ids: Vec<String> = self.actors.iter().map(|e| e.key().clone()).collect();
        for id in ids {
            self.stop_actor(&id);
        }
    }

    /// Remove an actor's handle from the registry. Called after
    /// `ActorHandle::await_join` returns so the registry doesn't grow
    /// unbounded over a long session.
    pub fn deregister(&self, id: &str) -> Option<super::rt_types::ActorHandle> {
        self.actors.remove(id).map(|(_, h)| h)
    }
}
