// REFERENCE — extracted from
//   docs/superpowers/specs/2026-06-18-lightweight-actor-design.md (lines 70-99)
// Last synced: 2026-06-19 (design doc only; no Rust impl yet — see status below)
// DESIGN DOC — NOT IMPLEMENTED. The "Last synced" field points at the spec SHA,
// not a source commit. This is the canonical shape future code should implement.

#![allow(dead_code)] // This file is a design reference, not compiled.

//! SkillActor trait — designed but NOT implemented as of 2026-06-19.
//!
//! Status: spec + plan written, no Rust crate scaffolded. The plan to
//! implement this lives in `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`
//! (19 tasks across 4 phases; default recommendation: execute Phase 1
//! (skeleton only, 5 tasks) this session, defer Phases 2-4).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

// Forward declarations — actual types live in the (planned) actor crate.
pub trait ToolDispatcher: Send + Sync { /* see 02-tool-dispatcher-trait.rs */ }
pub trait TelemetrySink: Send + Sync {
    fn record(&self, event: TelemetryEvent);
}

pub enum TelemetryEvent {
    ActorTicked { actor_id: String, duration: Duration },
    ActorErrored { actor_id: String, error: String },
    ActorCancelled { actor_id: String },
}

/// Context provided to a SkillActor on every tick.
pub struct ActorContext {
    pub tool_dispatcher: Arc<dyn ToolDispatcher>,
    pub cancel: CancellationToken,
    pub telemetry: Arc<dyn TelemetrySink>,
}

/// Output of a single actor tick. The runtime interprets this to update
/// state, dispatch events, or surface errors.
pub enum ActorOutput {
    /// No-op; the runtime just notes that the actor is alive.
    Silent,
    /// An event to flow into the main session's event bus.
    Event(ActorEvent),
    /// A recoverable error; the runtime logs and continues.
    Error(ActorError),
}

pub struct ActorEvent { /* opaque payload */ _private: () }
pub struct ActorError { pub message: String }

/// ★ The headline trait. Designed to live in
/// `crates/services/services-core/src/skill_runtime/async_mode.rs`
/// per the spec, but not yet placed there.
#[async_trait]
pub trait SkillActor: Send + Sync {
    fn id(&self) -> &str;
    fn skill_name(&self) -> &str;

    /// Called by ActorRuntime on a schedule or event. Returning `Ok(None)`
    /// is a silent tick.
    async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>, ActorError>;
}

// ═══════════════════════════════════════════════════════════════════════
// INVARIANTS — these are part of the spec, not implementation details.
// When implementing or extending the actor, the runtime MUST enforce them.
// ═══════════════════════════════════════════════════════════════════════
//
// 1. `SkillActor::tick` MUST NOT call any LLM directly. If a skill needs
//    LLM to function, it is not an actor — either promote to a full
//    subagent (existing Task tool path) or have the actor call
//    `ctx.tool_dispatcher.dispatch_once(...)` for a single LLM call.
//    Multi-round LLM loops are NOT allowed in an actor.
//
// 2. `SkillActor::tick` MUST be cancel-aware. It must observe
//    `ctx.cancel` on every blocking call (sleep, IO, sub-dispatch).
//    Failing to do so is a bug; the runtime will warn-log on join if
//    an actor survived its cancel token.
//
// 3. Actor state is in-memory. Restart loses state. Skill registration
//    is persistent; actor instance is NOT.
//
// 4. Per-tick timeout enforced by the runtime (default 30s). The
//    actor can shorten via `ctx.cancel` but cannot extend.
