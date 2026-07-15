//! Provider-neutral Deep Review task execution decisions.
//!
//! This module owns manifest packet matching, bounded retry validation,
//! provider-capacity retry timing, provider queue step decisions, and
//! TaskTool presentation facts. Product assembly/core keeps concrete
//! task launch, event emission, queue sleeping, and runtime state mutation.
//!
//! The implementation lives in sibling files. This file is the wildcard
//! re-export facade (per R25-R31 pattern) plus the test module.
//!
//! Sub-domain split:
//! - `types` — shared data DTOs and small enums
//! - `provider_capacity_queue` — `DeepReviewProviderCapacityQueueRuntime`
//!   + step / skip-result decisions
//! - `reviewer_admission_queue` — `DeepReviewReviewerAdmissionQueueRuntime`
//!   + step / skip-result decisions
//! - `retry_runtime` — `DeepReviewProviderCapacityRetryRuntime`, retry
//!   coverage/scope validation, `QueueWaitTimer` / `QueueWaitSnapshot`
//!   primitive, shared `decide_queue_control_step`
//! - `task_completion_and_cache` — task completion result, cancelled
//!   reviewer result, retry-guidance presentation, incremental cache
//!   attach, packet-id resolution, launch-batch lookup
//!
//! Tests live alongside the production code in each sibling file
//! (`#[cfg(test)] mod tests` per sibling). This facade re-exports the
//! public API but owns no test code itself (R45d split).

pub use super::provider_capacity_queue::*;
pub use super::retry_runtime::*;
pub use super::reviewer_admission_queue::*;
pub use super::task_completion_and_cache::*;
pub use super::types::*;
