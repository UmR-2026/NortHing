//! Task tool — DeepReview sibling facade (Round 12b thin re-export)
//!
//! Production code lives in `task_tool_deep_review_policy`.
//! Tests live in `task_tool_deep_review_tests`.
//!
//! This file exists only to preserve the `super::task_tool_deep_review::*`
//! import paths used by facade (`task_tool.rs`) and sibling
//! (`task_tool_subagent.rs`) callers. Without this re-export facade, every
//! caller would need to migrate to `super::task_tool_deep_review_policy::*`.
//!
//! Spec: `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md` (e4261ff)
//! Pattern: QClaw R12 review §10 (code + tests + thin facade).

pub use super::task_tool_deep_review_policy::*;
