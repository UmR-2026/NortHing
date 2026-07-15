//! `ActorRuntime` — Phase 2 partial implementation.
//!
//! Pattern source: `.agents/reference/actor/03-actor-runtime.rs` (design
//! doc, 2026-06-19). This file ships the **first compilable Rust shape**
//! of the runtime; the `spawn_actor` body is intentionally minimal
//! because the live scheduling body lands in Phase 2.6 (impl plan 2.3 + 2.5).
//!
//! ## Status
//!
//! Phase 2 partial — `ActorHandle` is fully functional (cancel + join),
//! `ActorRuntime` owns the registry and can spawn a `SkillActor` on
//! the current tokio runtime with cancel observation. The **scheduling**
//! body (periodic / on-signal / cron) is **not** implemented; spawning
//! a `Periodic` actor currently runs a single tick and exits.
//!
//! `USE_LIGHTWEIGHT_ACTOR` (defined in `flags.rs`) stays `false` — no
//! call site constructs an `ActorRuntime` yet. The runtime compiles
//! and the unit tests exercise the cancel path.
//!
//! ## Spec invariants (carried over from `actor.rs`)
//!
//! - Per-tick timeout enforced by the runtime (default 30s).
//! - `ActorRuntime` observes the cancel token on every blocking call.
//! - State is in-memory; restart loses state.
//!
//! ## Module layout (R47a split)
//!
//! The runtime body is split across sibling files so the source stays
//! scannable and the four sub-domains can evolve independently:
//!
//! - [`rt_types`] — `ActorHandle`, `ActorRuntime` struct decls + Debug impls
//! - [`rt_state`] — constructors (`new`, `with_handle`) and registry
//!   operations (`len`, `is_empty`, `stop_actor`, `stop_all`,
//!   `deregister`, `set_default_tick_timeout`)
//! - [`rt_dispatch`] — spawn entry points (`spawn_actor`, `spawn_one_shot`,
//!   `spawn_long_running`)
//! - [`rt_handlers`] — `run_one_tick` helper + the `#[cfg(test)] mod tests`
//!
//! External callers see [`ActorHandle`] and [`ActorRuntime`] at this
//! path (`crate::runtime::ActorRuntime`); the `pub use` re-exports
//! below preserve the historical surface that `lib.rs` flattens into
//! `northhing_agent_dispatch::{ActorHandle, ActorRuntime}`.

mod rt_dispatch;
mod rt_handlers;
mod rt_state;
mod rt_types;

pub use rt_types::{ActorHandle, ActorRuntime};
