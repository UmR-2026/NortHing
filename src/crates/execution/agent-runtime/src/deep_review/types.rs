//! Provider-neutral Deep Review task-execution shared data types.
//!
//! Pure data DTOs and small enums referenced by sibling modules
//! (`provider_capacity_queue`, `reviewer_admission_queue`, `retry_runtime`,
//! `task_completion_and_cache`). No runtime logic lives here — only the
//! vocabulary the siblings share.

use super::{DeepReviewCapacityQueueReason, DeepReviewQueueControlSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewQueueWaitSkipReason {
    QueueExpired,
    UserCancelled,
    OptionalSkipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewLaunchBatchInfo {
    pub packet_id: Option<String>,
    pub launch_batch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewIncrementalCacheHit {
    pub packet_id: String,
    pub cached_output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewProviderCapacityErrorCategory {
    RateLimit,
    ProviderUnavailable,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepReviewProviderCapacityErrorFacts<'a> {
    pub provider_code: &'a str,
    pub provider_message: &'a str,
    pub retry_after_seconds: Option<u64>,
    pub category: DeepReviewProviderCapacityErrorCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewProviderCapacityQueueStepFacts {
    pub reason: DeepReviewCapacityQueueReason,
    pub queue_expired: bool,
    pub initial_active_reviewer_count: usize,
    pub active_reviewer_count: usize,
    pub control_snapshot: DeepReviewQueueControlSnapshot,
    pub is_optional_reviewer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewQueueControlStepDecision {
    Skipped { skip_reason: DeepReviewQueueWaitSkipReason },
    Paused,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewProviderCapacityQueueStepDecision {
    Skipped { skip_reason: DeepReviewQueueWaitSkipReason },
    Paused,
    ReadyToRetry { early_capacity_probe: bool },
    Queued,
}
