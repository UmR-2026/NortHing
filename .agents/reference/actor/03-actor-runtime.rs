// REFERENCE — extracted from
//   docs/superpowers/specs/2026-06-18-lightweight-actor-design.md (lines 137-169)
// Last synced: 2026-06-19 (design doc only)
// DESIGN DOC — NOT IMPLEMENTED.

#![allow(dead_code)]

//! ActorRuntime — designed but NOT implemented.
//!
//! Owns a registry of running actors and dispatches ticks to them.
//! Planned location: `crates/agent-dispatch/src/runtime.rs` (not yet
//! scaffolded; see `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`).

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::skill_actor::{ActorContext, SkillActor};
use super::tool_dispatcher::ToolDispatcher;
use super::telemetry::TelemetrySink;

/// Public handle to a running actor. Lets the runtime (or a test)
/// stop the actor cleanly.
pub struct ActorHandle {
    pub id: String,
    cancel: CancellationToken,
    join: JoinHandle<()>,
}

impl ActorHandle {
    /// Stop the actor (sets cancel). Does not wait for join — call `await_join()`.
    pub fn stop(&self) { self.cancel.cancel(); }
    pub async fn await_join(self) -> Result<(), tokio::task::JoinError> { self.join.await }
}

/// Owns the registry. Construct once at app start; share via `Arc`.
pub struct ActorRuntime {
    actors: DashMap<String, ActorHandle>,
    dispatcher: Arc<dyn ToolDispatcher>,
    telemetry: Arc<dyn TelemetrySink>,
    /// Default per-tick timeout (spec suggests 30s; overridable per actor).
    default_tick_timeout: Duration,
}

impl ActorRuntime {
    pub fn new(
        dispatcher: Arc<dyn ToolDispatcher>,
        telemetry: Arc<dyn TelemetrySink>,
    ) -> Self {
        Self {
            actors: DashMap::new(),
            dispatcher,
            telemetry,
            default_tick_timeout: Duration::from_secs(30),
        }
    }

    /// Spawn a new actor. The runtime takes ownership of the boxed actor
    /// and starts ticking it on the configured schedule.
    pub fn spawn_actor(
        &self,
        actor: Box<dyn SkillActor>,
        schedule: ActorSchedule,
    ) -> ActorHandle { unimplemented!("see spec for full body") }

    /// Stop an actor by id. No-op if not running.
    pub fn stop_actor(&self, id: &str) { unimplemented!() }

    /// Stop all actors. Used at shutdown.
    pub fn stop_all(&self) { unimplemented!() }
}

/// When the runtime ticks the actor. Spec leaves this open; a sensible
/// default set is provided.
pub enum ActorSchedule {
    /// Tick every `period` (best effort; honors actor's per-tick duration).
    Periodic(Duration),
    /// Tick on a channel signal (event-driven).
    OnSignal(tokio::sync::mpsc::Receiver<ActorTrigger>),
    /// Tick on a cron-style schedule.
    Cron(String),
    /// One-shot (tick once, then stop). Common for one-shot subagents.
    OneShot,
}

pub enum ActorTrigger { /* opaque */ _private: () }

// ═══════════════════════════════════════════════════════════════════════
// IPC adapter (STUB) — see NOTES.md
// ═══════════════════════════════════════════════════════════════════════
//
// The spec defines `IpcSpawnAdapter` for spawning actors in a separate
// process. This is a STUB in the planned Phase 1 (returns `"ipc-stub"`);
// the IPC body lands in Phase 3. Do not write code against the IPC
// surface before Phase 3 ships.

// ═══════════════════════════════════════════════════════════════════════
// CONST FLAGS (planned)
// ═══════════════════════════════════════════════════════════════════════
//
// pub const USE_LIGHTWEIGHT_ACTOR: bool = false;  // per plan: default off
// pub const USE_ONESHOT_DISPATCHER: bool = false; // per plan: default off
// pub const USE_ACTOR_IPC: bool = false;          // per plan: default off
// pub const USE_DISPATCHER_IPC: bool = false;     // per plan: default off
//
// When any flag is true, the corresponding surface is enabled; when
// false, the call site falls back to the existing
// ConversationCoordinator::execute_hidden_subagent_internal path.
