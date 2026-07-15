//! `SkillActor` trait — Phase 2 partial implementation.
//!
//! Pattern source: `.agents/reference/actor/01-skill-actor-trait.rs`
//! (the design doc, 2026-06-19). This file is the **first compilable
//! Rust shape**; the design-doc file is `unimplemented!()` body and is
//! not wired into the crate.
//!
//! ## Status
//!
//! Phase 2 **partial** — trait body defined; no runtime body, no call-site
//! wiring. The companion runtime (`runtime.rs`) provides the
//! `ActorHandle` skeleton that would spawn a `SkillActor`.
//!
//! `USE_LIGHTWEIGHT_ACTOR` (defined in `flags.rs`) stays `false` —
//! **no behavior change** ships with this milestone. The trait compiles,
//! tests pass, but no caller constructs or ticks an actor yet.
//!
//! ## Spec invariants (from `.agents/reference/actor/01-skill-actor-trait.rs` lines 70-99)
//!
//! 1. `SkillActor::tick` MUST NOT call any LLM directly. If a skill needs
//!    an LLM, it must use `ctx.tool_dispatcher.dispatch_once(...)`.
//! 2. `SkillActor::tick` MUST be cancel-aware — observe `ctx.cancel` on
//!    every blocking call.
//! 3. Actor state is in-memory. Restart loses state.
//! 4. Per-tick timeout is enforced by the runtime (default 30s).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::telemetry::TelemetrySink;
use northhing_runtime_ports::ToolDispatcherPort;
// `LightweightTaskOutput` / `LightweightTaskRequest` are only referenced
// from the test module below; the trait body itself doesn't name them.
// They are re-imported inside the test module to keep the lib's
// top-level imports minimal.

/// Context passed to every `SkillActor::tick` call.
///
/// The runtime constructs this once per tick and hands the actor a
/// reference; actors MUST NOT retain the reference beyond the tick —
/// the underlying `cancel` token can be replaced on the next tick.
pub struct ActorContext {
    /// Handle to the one-shot tool dispatcher. Implementations MUST
    /// use this (rather than calling any LLM directly) when the actor
    /// needs to perform an LLM-mediated action — see invariant #1.
    pub tool_dispatcher: Arc<dyn ToolDispatcherPort>,
    /// Cancel token. The runtime cancels this when the actor should
    /// stop (graceful shutdown, per-tick timeout, or explicit `stop`).
    /// Actors MUST observe it on every blocking call — see invariant #2.
    pub cancel: CancellationToken,
    /// Telemetry sink for this tick. The runtime owns the lifetime;
    /// actors may emit `ActorTicked` / `ActorErrored` events but
    /// MUST NOT block on the emit path.
    pub telemetry: Arc<dyn TelemetrySink>,
}

/// What an actor returned from `tick`. The runtime interprets each
/// variant to update state, dispatch events, or surface errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorOutput {
    /// No-op; the runtime records an `ActorTicked` event and waits
    /// for the next scheduled tick.
    Silent,
    /// A user-visible event flowed back to the main session's event bus.
    Event(ActorEvent),
    /// A recoverable error; the runtime logs it and continues ticking.
    /// Hard errors (broken invariants) should use `ActorError` returned
    /// from `tick` rather than this variant.
    Error(ActorError),
}

/// An opaque event payload. The runtime transports it through the main
/// session's event bus; the actor and the consumer don't need to
/// share a concrete type at compile time.
///
/// Future work may add typed event categories (e.g., `CodeChange`,
/// `TestResult`); Phase 2 ships the opaque shape so the trait body
/// compiles without taking a position on the event taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorEvent {
    _private: (),
}

/// A recoverable error from a tick. The runtime logs the message and
/// continues; the actor itself is not torn down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorError {
    pub message: String,
}

impl ActorError {
    /// Convenience constructor for actors that hit a recoverable error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "actor error: {}", self.message)
    }
}

impl std::error::Error for ActorError {}

/// When the runtime ticks the actor. Mirrors
/// `.agents/reference/actor/03-actor-runtime.rs::ActorSchedule`.
///
/// `OnSignal` carries an `mpsc::Receiver` so the runtime can consume
/// triggers produced by external code (e.g., the existing task tool
/// routing). The receiver is moved into the runtime when the actor
/// is spawned; the sender side is held by the producer.
pub enum ActorSchedule {
    /// Tick every `period` (best effort; honors actor's per-tick duration).
    Periodic(Duration),
    /// Tick when the runtime receives a signal on the channel
    /// (event-driven).
    OnSignal(tokio::sync::mpsc::Receiver<ActorTrigger>),
    /// One-shot (tick once, then stop). Common for one-shot subagents.
    OneShot,
}

/// A trigger payload pushed by the producer side of an
/// `ActorSchedule::OnSignal` channel. Phase 2.6 keeps the payload
/// opaque; Phase 3 will type it once the subagent routing code lands
/// (impl plan 3.x).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorTrigger {
    /// Opaque payload placeholder. Phase 3 will replace this with the
    /// concrete trigger variants the subagent routing needs.
    Opaque,
}

impl std::fmt::Debug for ActorSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorSchedule::Periodic(d) => write!(f, "Periodic({:?})", d),
            ActorSchedule::OnSignal(_) => write!(f, "OnSignal(<receiver>)"),
            ActorSchedule::OneShot => write!(f, "OneShot"),
        }
    }
}

/// The headline trait. Designed to live in `agent-dispatch::actor` per
/// Phase 2 of the impl plan; not yet wired to any caller.
#[async_trait]
pub trait SkillActor: Send + Sync {
    /// Stable id used as the `DashMap` key in `ActorRuntime`. Must be
    /// unique per registered actor.
    fn id(&self) -> &str;

    /// The skill this actor implements. Used for telemetry correlation
    /// and (in future phases) for routing telemetry events back to the
    /// skill's owner.
    fn skill_name(&self) -> &str;

    /// Called by `ActorRuntime` on the configured schedule. Returning
    /// `Ok(None)` is treated as `Ok(Some(ActorOutput::Silent))`.
    async fn tick(&mut self, ctx: &ActorContext) -> Result<Option<ActorOutput>, ActorError>;
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest};

    /// A trivial actor used in the trait-body smoke tests. Returns
    /// `Silent` on every tick and records how many ticks it has run.
    #[derive(Default)]
    struct CountingActor {
        id: String,
        ticks: u32,
    }

    #[async_trait]
    impl SkillActor for CountingActor {
        fn id(&self) -> &str {
            &self.id
        }
        fn skill_name(&self) -> &str {
            "counting"
        }
        async fn tick(&mut self, _ctx: &ActorContext) -> Result<Option<ActorOutput>, ActorError> {
            self.ticks += 1;
            Ok(Some(ActorOutput::Silent))
        }
    }

    #[test]
    fn actor_error_display_format_is_stable() {
        let err = ActorError::new("disk full");
        assert_eq!(err.to_string(), "actor error: disk full");
    }

    #[test]
    fn actor_output_variants_are_distinct() {
        // The runtime dispatches on the variant — distinguishability is
        // a compile-time guarantee, but we pin the PartialEq behavior
        // for callers that compare outputs.
        let silent = ActorOutput::Silent;
        let silent2 = ActorOutput::Silent;
        assert_eq!(silent, silent2);
    }

    #[tokio::test]
    async fn counting_actor_records_ticks() {
        // Smoke test the trait body — even though no runtime calls
        // `tick` yet, the trait compiles and an in-test invocation
        // works. The runtime body lands in `runtime.rs`.
        use crate::telemetry::NoopTelemetrySink;
        use northhing_runtime_ports::ToolDispatcherPort;

        struct NullDispatcher;
        #[async_trait::async_trait]
        impl ToolDispatcherPort for NullDispatcher {
            async fn dispatch_once(&self, _req: LightweightTaskRequest) -> LightweightTaskOutput {
                LightweightTaskOutput::NoToolMatched { reason: "null".into() }
            }
        }

        let mut actor = CountingActor {
            id: "actor-1".into(),
            ticks: 0,
        };
        let ctx = ActorContext {
            tool_dispatcher: Arc::new(NullDispatcher),
            cancel: CancellationToken::new(),
            telemetry: Arc::new(NoopTelemetrySink),
        };
        let out = actor.tick(&ctx).await.expect("tick ok");
        assert_eq!(out, Some(ActorOutput::Silent));
        assert_eq!(actor.ticks, 1);
    }
}
