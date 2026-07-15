//! Sub-domain: subagent execution outcome monitoring.
//! Spec step-3.7 — extracted from so_lifecycle.rs (R54a refactor).

/// Outcome of monitoring a subagent execution.
pub(crate) enum SubagentExecutionOutcome<T> {
    Completed(T),
    Cancelled,
    TimedOut,
}
