//! Deep Review reviewer budget, retry admission, and runtime accounting.
//!
//! This tracker is deliberately Deep Review-specific. It combines per-turn
//! reviewer/judge budgets, retry budgets, active reviewer counts, effective
//! concurrency learning, capacity diagnostics, and shared-context measurement.
//! Do not move it wholesale to `subagent_runtime`: only isolated mechanics with
//! no Deep Review policy, report, or diagnostic semantics should become generic.

pub mod budget_calc;
pub mod budget_enforce;
pub mod budget_observability;
pub mod budget_state;
pub mod budget_types;

pub use budget_state::DeepReviewBudgetTracker;
pub use budget_types::DeepReviewActiveReviewerGuard;
