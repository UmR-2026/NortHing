//! Const flags controlling the actor / dispatcher rollout.
//!
//! Pattern source: `.agents/reference/actor/06-const-flag-usage.md`.
//!
//! Only one flag remains. Flipping it requires:
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
///
/// Phase 3 IPC (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC / USE_DISPATCHER_IPC)
/// officially descoped 2026-07-20 — those flags have been deleted.
pub const USE_LIGHTWEIGHT_ACTOR: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags reflect the current phase of the rollout.
    ///
    /// As of 2026-06-23 (spec
    /// `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`),
    /// `USE_LIGHTWEIGHT_ACTOR` is ACTIVATED.
    /// The Phase 3 IPC flags (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC /
    /// USE_DISPATCHER_IPC) were descoped and deleted 2026-07-20.
    #[test]
    fn flags_phase_appropriate() {
        assert!(USE_LIGHTWEIGHT_ACTOR);
    }
}
