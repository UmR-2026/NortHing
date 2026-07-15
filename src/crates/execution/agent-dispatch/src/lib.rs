#![allow(clippy::too_many_arguments)]
//! Lightweight actor and one-shot dispatcher runtime for `northhing`.
//!
//! ## Status
//!
//! **Phase 2 partial — trait bodies landed; no call-site wiring yet.**
//! Per [`docs/plans/2026-06-19-post-reference-roadmap.md`](../../../plans/2026-06-19-post-reference-roadmap.md)
//! Phase B tasks B.1–B.6 (Phase 1) and impl plan 2.1–2.3 (Phase 2 partial),
//! this crate currently contains:
//!
//! - 4 const flags (all `false`) controlling rollout of the new surfaces
//! - The `TelemetrySink` trait contract
//! - A `ToolDispatcher` port stub re-exported from `runtime-ports`
//! - Stub `tokio` (in-process) and `ipc` (stub returning `"ipc-stub"`) spawn adapters
//! - **Phase 2 partial**: the `SkillActor` trait body + `ActorHandle` /
//!   `ActorRuntime` skeleton (`actor.rs`, `runtime.rs`). `OneShot` is fully
//!   implemented; `Periodic` and `OnSignal` are stub bodies that run a
//!   single tick — the scheduler loop lands in Phase 2.6.
//!
//! Behavior surfaces (replacing `ConversationCoordinator::execute_hidden_subagent_internal`,
//! live scheduling, IPC body) land in Phase 2.6+. **All 4 const flags
//! default to `false`**, so flipping any of them requires explicit action —
//! see `.agents/reference/actor/06-const-flag-usage.md` for the
//! project-standard flip process.
//!
//! ## Reference
//!
//! Pattern source: `.agents/reference/actor/01-skill-actor-trait.rs` and
//! `03-actor-runtime.rs` (design docs, 2026-06-19). The Rust bodies here
//! are the first compilable shape; the design-doc files remain `unimplemented!()`
//! references until the next phase rewrites them.

#![allow(dead_code)]

pub mod actor;
pub mod flags;
pub mod long_running;
pub mod runtime;
pub mod spawn;
pub mod telemetry;

pub use actor::{ActorContext, ActorError, ActorOutput, ActorSchedule, ActorTrigger, SkillActor};
pub use flags::{USE_ACTOR_IPC, USE_DISPATCHER_IPC, USE_LIGHTWEIGHT_ACTOR, USE_ONESHOT_DISPATCHER};
pub use long_running::{LongRunningRequest, LongRunningSkill, LongRunningTickOutput};
pub use runtime::{ActorHandle, ActorRuntime};
pub use telemetry::{NoopTelemetrySink, TelemetryEvent, TelemetrySink};
