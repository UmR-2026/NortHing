//! Desktop-specific const flags.
//!
//! Mirrors `src/crates/execution/agent-dispatch/src/flags.rs` but scoped
//! to the desktop UI shell. Flags here drive **presentation** behavior
//! (which UI branches render), not runtime actor behavior.
//!
//! Pattern source: `.agents/reference/actor/06-const-flag-usage.md`.

/// Default mode id used by the desktop shell's skill panel.
pub const DEFAULT_MODE_ID: &str = "agentic"; // 2026-07-18: registry has no "code" mode; agentic is the default single-agent mode

#[cfg(test)]
mod tests {
    use super::*;

    /// `DEFAULT_MODE_ID = "agentic"` - the registry only has "agentic" /
    /// "Claw" / "Team" modes, no "code". Edit here when multi-mode shell
    /// is introduced.
    #[test]
    fn default_mode_id_is_agentic() {
        assert_eq!(DEFAULT_MODE_ID, "agentic");
    }
}
