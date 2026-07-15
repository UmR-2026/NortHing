//! Task tool implementations (Round 12 + Round 12b split)
//!
//! Round 12 split: TaskTool + impl + 5 sub-handler siblings per fn domain.
//! Round 12b split: `task_tool_deep_review` further split into policy (production
//! code) + tests (#[cfg(test)]) + thin re-export facade (backward compat).
//!
//! - `task_tool` (facade): `TaskTool` struct + Tool trait impl + tool_core fns + call_impl orchestrator
//! - `task_tool_deep_review` (thin facade): pub use re-exports
//! - `task_tool_deep_review_policy` (~870): 20 deep_review_* production fns + setup helper
//! - `task_tool_deep_review_tests` (~600): sync tests + reviewer-queue async tests
//! - `task_tool_deep_review_tests_runtime` (~550): remaining async tokio tests + retry + provider tests
//! - `task_tool_subagent` (~450): 10 subagent fns + 2 tests + call_impl subagent dispatch/loop
//! - `task_tool_agents` (~300): 6 agent fns + 2 tests + call_impl completion result + PromptOrderTestAgent
//! - `task_tool_input` (~400): 5 input validation fns + 2 tests + call_impl input prep phase
//!
//! Spec: `docs/handoffs/2026-06-29-round12-task-tool-split-spec.md` (f0f9bc0)
//!       `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md` (e4261ff)

pub mod task_tool;
pub mod task_tool_agents;
pub mod task_tool_deep_review;
pub mod task_tool_deep_review_policy;
pub mod task_tool_deep_review_tests;
pub mod task_tool_deep_review_tests_runtime;
pub mod task_tool_input;
pub mod task_tool_subagent;

// Re-export public API (preserves caller compatibility: `crate::...::task_tool::TaskTool`)
pub use task_tool::TaskTool;
