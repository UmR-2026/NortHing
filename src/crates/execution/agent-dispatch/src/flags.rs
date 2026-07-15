//! Const flags controlling the actor / dispatcher rollout.
//!
//! Pattern source: `.agents/reference/actor/06-const-flag-usage.md`.
//!
//! All four flags **default to `false`**. Flipping any of them requires:
//!   1. Integration test passing with flag = `true` (and `false` regression).
//!   2. `PROJECT_STATE.md` update recording the new state.
//!   3. Rollback stays one-line (`const FLAG: bool = false;` + commit).

/// Enable the `SkillActor` runtime (replaces the heavy
/// `ConversationCoordinator::execute_hidden_subagent_internal` path for
/// periodic / cron / signal-driven actors).
///
/// ACTIVATED 2026-06-23 per
/// `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`.
/// Phase 2 of the impl plan has passed integration; the A2 long-running path
/// now replaces the legacy `execute_hidden_subagent_phase1/2/3` for all
/// `Task` tool invocations on the desktop app.
pub const USE_LIGHTWEIGHT_ACTOR: bool = true;

/// Enable the `ToolDispatcher` for one-shot subagent dispatches.
///
/// Flip to `true` **only after** Phase 1 of the impl plan passes integration.
pub const USE_ONESHOT_DISPATCHER: bool = false;

/// Allow actors to spawn in a separate process (IPC).
///
/// Flip to `true` **only after** Phase 3 of the impl plan lands.
pub const USE_ACTOR_IPC: bool = false;

/// Allow dispatches to run in a separate process (IPC).
///
/// Flip to `true` **only after** Phase 3 of the impl plan lands.
pub const USE_DISPATCHER_IPC: bool = false;

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags reflect the current phase of the rollout.
    ///
    /// As of 2026-06-23 (spec
    /// `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`),
    /// `USE_LIGHTWEIGHT_ACTOR` is ACTIVATED. The other three flags represent
    /// future work (one-shot dispatcher + IPC adapters) and remain off.
    /// If any of the three future flags ever flip, the flip must be paired
    /// with a regression test (see `06-const-flag-usage.md` rule 4).
    #[test]
    fn flags_phase_appropriate() {
        assert!(USE_LIGHTWEIGHT_ACTOR);
        assert!(!USE_ONESHOT_DISPATCHER);
        assert!(!USE_ACTOR_IPC);
        assert!(!USE_DISPATCHER_IPC);
    }
}
