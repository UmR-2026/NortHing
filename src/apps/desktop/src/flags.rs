//! Desktop-specific const flags.
//!
//! Mirrors `src/crates/execution/agent-dispatch/src/flags.rs` but scoped
//! to the desktop UI shell. Flags here drive **presentation** behavior
//! (which UI branches render), not runtime actor behavior.
//!
//! Pattern source: `.agents/reference/actor/06-const-flag-usage.md`.
//!
//! ## Phase C.2
//!
//! `SESSION_TREE_VIEW` — when `true`, the sidebar renders subagent
//! sessions nested under their parent. When `false`, the sidebar
//! renders a flat list (legacy A6 behavior). The default is `true` per
//! `main.rs::SESSION_TREE_VIEW`; the value is duplicated here so the
//! `app_state::create_ui` path can read it without depending on the
//! binary's `main` module (which isn't accessible from `lib.rs`).

/// A6 / Phase C.2: render sessions as a nested tree when `true`.
///
/// `false` keeps the byte-identical flat list that shipped in A6.
#[allow(dead_code)]
pub const SESSION_TREE_VIEW: bool = true;

/// Default mode id used by the desktop shell's skill panel.
pub const DEFAULT_MODE_ID: &str = "agentic"; // 2026-07-18: registry has no "code" mode; agentic is the default single-agent mode

#[cfg(test)]
mod tests {
    use super::*;

    /// `SESSION_TREE_VIEW = true` is the deliberate Phase C.2 default —
    /// flipping it to `false` is a one-line UI regression test, not a
    /// silent behavior change. Lock it down here so any flip is paired
    /// with a corresponding test update.
    #[test]
    fn session_tree_view_default_phase_c2() {
        assert!(SESSION_TREE_VIEW);
    }

    /// `DEFAULT_MODE_ID = "agentic"` — the registry only has "agentic" /
    /// "Claw" / "Team" modes, no "code". Edit here when multi-mode shell
    /// is introduced.
    #[test]
    fn default_mode_id_is_agentic() {
        assert_eq!(DEFAULT_MODE_ID, "agentic");
    }
}
