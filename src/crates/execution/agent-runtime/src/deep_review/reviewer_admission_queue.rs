//! Reviewer admission queue runtime and local capacity decisions.
//!
//! `DeepReviewReviewerAdmissionQueueRuntime` walks a single reviewer through
//! local capacity back-pressure (local concurrency caps, launch-batch
//! blocks). Pure helper decisions
//! (`decide_blocked_reviewer_admission_queue_step`,
//! `capacity_skip_result_for_local_queue_outcome`,
//! `local_reviewer_capacity_queue_decision`) live here too.
//!
//! The wait timer primitive lives in `super::retry_runtime`; this file only
//! owns the runtime that uses it.

use super::retry_runtime::{decide_queue_control_step, QueueWaitTimer};
use super::types::{DeepReviewQueueControlStepDecision, DeepReviewQueueWaitSkipReason};
use super::{
    classify_deep_review_capacity_error, DeepReviewCapacityQueueDecision, DeepReviewCapacityQueueReason,
    DeepReviewConcurrencyPolicy, DeepReviewQueueControlSnapshot,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DeepReviewReviewerAdmissionQueueRuntime {
    timer: QueueWaitTimer,
    max_wait: Duration,
    local_capacity_reason: DeepReviewCapacityQueueReason,
    retry_after_seconds: Option<u64>,
    last_wait_reason: DeepReviewCapacityQueueReason,
    last_queue_elapsed: Duration,
    is_optional_reviewer: bool,
}

impl DeepReviewReviewerAdmissionQueueRuntime {
    pub fn start(
        now: Instant,
        local_capacity_reason: DeepReviewCapacityQueueReason,
        max_wait: Duration,
        retry_after_seconds: Option<u64>,
        is_optional_reviewer: bool,
    ) -> Self {
        Self {
            timer: QueueWaitTimer::start(now),
            max_wait,
            local_capacity_reason,
            retry_after_seconds,
            last_wait_reason: local_capacity_reason,
            last_queue_elapsed: Duration::ZERO,
            is_optional_reviewer,
        }
    }

    pub fn begin_step(
        &mut self,
        input: DeepReviewReviewerAdmissionQueueRuntimeInput,
    ) -> DeepReviewReviewerAdmissionQueueRuntimeStep {
        let queue_snapshot = self.timer.snapshot(input.now);
        self.last_queue_elapsed = queue_snapshot.queue_elapsed;
        let queue_elapsed_ms = queue_snapshot.queue_elapsed_ms;
        let current_reason = self.last_wait_reason;

        match decide_queue_control_step(&input.control_snapshot, self.is_optional_reviewer) {
            DeepReviewQueueControlStepDecision::Skipped { skip_reason } => {
                DeepReviewReviewerAdmissionQueueRuntimeStep::Skipped {
                    queue_elapsed_ms,
                    skip_reason,
                    capacity_reason: current_reason,
                }
            }
            DeepReviewQueueControlStepDecision::Paused => {
                self.timer.pause(input.now);
                DeepReviewReviewerAdmissionQueueRuntimeStep::Paused {
                    queue_elapsed_ms,
                    capacity_reason: current_reason,
                    next_sleep: input.poll_interval,
                }
            }
            DeepReviewQueueControlStepDecision::Continue => {
                self.timer.continue_now(input.now);
                DeepReviewReviewerAdmissionQueueRuntimeStep::TryAdmit {
                    queue_elapsed_ms,
                    attempt: DeepReviewReviewerAdmissionTryAdmit {
                        queue_elapsed_ms,
                        queue_expired: queue_snapshot.is_expired(self.max_wait),
                    },
                    capacity_reason: current_reason,
                }
            }
        }
    }

    pub fn after_blocked_attempt(
        &mut self,
        attempt: DeepReviewReviewerAdmissionTryAdmit,
        capacity_reason: DeepReviewCapacityQueueReason,
        active_reviewer_count: usize,
        poll_interval: Duration,
    ) -> DeepReviewReviewerAdmissionQueueRuntimeBlockedStep {
        self.last_wait_reason = capacity_reason;

        match decide_blocked_reviewer_admission_queue_step(DeepReviewBlockedReviewerAdmissionQueueStepFacts {
            capacity_reason,
            queue_expired: attempt.queue_expired,
            active_reviewer_count,
        }) {
            DeepReviewBlockedReviewerAdmissionQueueStepDecision::CapacityExpired { capacity_reason } => {
                DeepReviewReviewerAdmissionQueueRuntimeBlockedStep::CapacityExpired {
                    queue_elapsed_ms: attempt.queue_elapsed_ms,
                    capacity_reason,
                    retry_after_seconds: (capacity_reason != DeepReviewCapacityQueueReason::LaunchBatchBlocked)
                        .then_some(self.retry_after_seconds)
                        .flatten(),
                }
            }
            DeepReviewBlockedReviewerAdmissionQueueStepDecision::Queued { capacity_reason } => {
                let next_sleep = if attempt.queue_expired {
                    poll_interval
                } else {
                    poll_interval.min(self.max_wait.saturating_sub(self.last_queue_elapsed))
                };
                DeepReviewReviewerAdmissionQueueRuntimeBlockedStep::Queued {
                    queue_elapsed_ms: attempt.queue_elapsed_ms,
                    capacity_reason,
                    next_sleep,
                }
            }
        }
    }

    pub fn local_capacity_reason(&self) -> DeepReviewCapacityQueueReason {
        self.local_capacity_reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewReviewerAdmissionQueueRuntimeInput {
    pub now: Instant,
    pub control_snapshot: DeepReviewQueueControlSnapshot,
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewReviewerAdmissionQueueRuntimeStep {
    Skipped {
        queue_elapsed_ms: u64,
        skip_reason: DeepReviewQueueWaitSkipReason,
        capacity_reason: DeepReviewCapacityQueueReason,
    },
    Paused {
        queue_elapsed_ms: u64,
        capacity_reason: DeepReviewCapacityQueueReason,
        next_sleep: Duration,
    },
    TryAdmit {
        queue_elapsed_ms: u64,
        attempt: DeepReviewReviewerAdmissionTryAdmit,
        capacity_reason: DeepReviewCapacityQueueReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepReviewReviewerAdmissionTryAdmit {
    pub queue_elapsed_ms: u64,
    pub queue_expired: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewReviewerAdmissionQueueRuntimeBlockedStep {
    CapacityExpired {
        queue_elapsed_ms: u64,
        capacity_reason: DeepReviewCapacityQueueReason,
        retry_after_seconds: Option<u64>,
    },
    Queued {
        queue_elapsed_ms: u64,
        capacity_reason: DeepReviewCapacityQueueReason,
        next_sleep: Duration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepReviewBlockedReviewerAdmissionQueueStepFacts {
    pub capacity_reason: DeepReviewCapacityQueueReason,
    pub queue_expired: bool,
    pub active_reviewer_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewBlockedReviewerAdmissionQueueStepDecision {
    CapacityExpired {
        capacity_reason: DeepReviewCapacityQueueReason,
    },
    Queued {
        capacity_reason: DeepReviewCapacityQueueReason,
    },
}

pub fn local_reviewer_capacity_queue_decision() -> DeepReviewCapacityQueueDecision {
    classify_deep_review_capacity_error(
        "deep_review_concurrency_cap_reached",
        "Maximum parallel reviewer instances reached",
        None,
    )
}

pub fn decide_blocked_reviewer_admission_queue_step(
    facts: DeepReviewBlockedReviewerAdmissionQueueStepFacts,
) -> DeepReviewBlockedReviewerAdmissionQueueStepDecision {
    if facts.queue_expired && facts.active_reviewer_count == 0 {
        return DeepReviewBlockedReviewerAdmissionQueueStepDecision::CapacityExpired {
            capacity_reason: facts.capacity_reason,
        };
    }

    DeepReviewBlockedReviewerAdmissionQueueStepDecision::Queued {
        capacity_reason: facts.capacity_reason,
    }
}

pub fn capacity_skip_result_for_local_queue_outcome(
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    capacity_reason: DeepReviewCapacityQueueReason,
    skip_reason: DeepReviewQueueWaitSkipReason,
    queue_elapsed_ms: u64,
    duration_ms: u128,
    effective_parallel_instances: usize,
) -> (Value, String) {
    let queue_skip_reason = match skip_reason {
        DeepReviewQueueWaitSkipReason::QueueExpired => "queue_expired",
        DeepReviewQueueWaitSkipReason::UserCancelled => "user_cancelled",
        DeepReviewQueueWaitSkipReason::OptionalSkipped => "optional_skipped",
    };
    let capacity_reason_code = capacity_reason.as_snake_case();
    let assistant_message = match skip_reason {
        DeepReviewQueueWaitSkipReason::QueueExpired => {
            let reason_message = match capacity_reason {
                DeepReviewCapacityQueueReason::LaunchBatchBlocked => {
                    "the previous launch batch did not finish before the queue wait limit"
                }
                DeepReviewCapacityQueueReason::LocalConcurrencyCap => {
                    "the local reviewer capacity queue reached its maximum wait"
                }
                _ => "the DeepReview capacity queue reached its maximum wait",
            };
            let recommended_action = match capacity_reason {
                DeepReviewCapacityQueueReason::LaunchBatchBlocked => {
                    "Wait for the earlier reviewer batch to finish or cancel stuck queued reviewers, then retry this packet with a lower max parallel reviewer setting if it repeats."
                }
                _ => {
                    "Run the review again with a lower max parallel reviewer setting or wait for active reviewers to finish."
                }
            };
            format!(
                "Subagent '{}' was skipped because {} ({}s). Recommended action: {}\n<queue_result status=\"capacity_skipped\" reason=\"{}\" queue_elapsed_ms=\"{}\" />",
                subagent_type,
                reason_message,
                conc_policy.max_queue_wait_seconds,
                recommended_action,
                capacity_reason_code,
                queue_elapsed_ms
            )
        }
        DeepReviewQueueWaitSkipReason::UserCancelled => format!(
            "Subagent '{}' was skipped because the DeepReview capacity queue was cancelled by the user.\n<queue_result status=\"capacity_skipped\" reason=\"user_cancelled\" queue_elapsed_ms=\"{}\" />",
            subagent_type, queue_elapsed_ms
        ),
        DeepReviewQueueWaitSkipReason::OptionalSkipped => format!(
            "Subagent '{}' was skipped because optional DeepReview queued reviewers were skipped by the user.\n<queue_result status=\"capacity_skipped\" reason=\"optional_skipped\" queue_elapsed_ms=\"{}\" />",
            subagent_type, queue_elapsed_ms
        ),
    };

    let data = json!({
        "duration": u64::try_from(duration_ms).unwrap_or(u64::MAX),
        "status": "capacity_skipped",
        "queue_elapsed_ms": queue_elapsed_ms,
        "max_queue_wait_seconds": conc_policy.max_queue_wait_seconds,
        "queue_skip_reason": queue_skip_reason,
        "capacity_reason": capacity_reason_code,
        "effective_parallel_instances": effective_parallel_instances
    });

    (data, assistant_message)
}

#[cfg(test)]
mod tests {
    use super::super::{DeepReviewCapacityQueueReason, DeepReviewQueueControlSnapshot};
    use super::*;
    use std::time::{Duration, Instant};

    fn control_snapshot(paused: bool, cancelled: bool, skip_optional: bool) -> DeepReviewQueueControlSnapshot {
        DeepReviewQueueControlSnapshot {
            paused,
            cancelled,
            skip_optional,
        }
    }

    #[test]
    fn local_reviewer_capacity_decision_stays_queueable() {
        let decision = local_reviewer_capacity_queue_decision();
        assert_eq!(
            decision.reason,
            Some(DeepReviewCapacityQueueReason::LocalConcurrencyCap)
        );
        assert!(decision.queueable);
    }

    #[test]
    fn reviewer_admission_queue_expires_only_without_active_reviewers() {
        assert_eq!(
            decide_blocked_reviewer_admission_queue_step(DeepReviewBlockedReviewerAdmissionQueueStepFacts {
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                queue_expired: true,
                active_reviewer_count: 0,
            },),
            DeepReviewBlockedReviewerAdmissionQueueStepDecision::CapacityExpired {
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap
            }
        );

        assert_eq!(
            decide_blocked_reviewer_admission_queue_step(DeepReviewBlockedReviewerAdmissionQueueStepFacts {
                capacity_reason: DeepReviewCapacityQueueReason::LaunchBatchBlocked,
                queue_expired: true,
                active_reviewer_count: 1,
            },),
            DeepReviewBlockedReviewerAdmissionQueueStepDecision::Queued {
                capacity_reason: DeepReviewCapacityQueueReason::LaunchBatchBlocked
            }
        );
    }

    #[test]
    fn reviewer_admission_queue_runtime_pauses_without_consuming_wait_budget() {
        let start = Instant::now();
        let mut runtime = DeepReviewReviewerAdmissionQueueRuntime::start(
            start,
            DeepReviewCapacityQueueReason::LocalConcurrencyCap,
            Duration::from_secs(2),
            None,
            false,
        );
        let poll_interval = Duration::from_millis(100);

        assert_eq!(
            runtime.begin_step(DeepReviewReviewerAdmissionQueueRuntimeInput {
                now: start + Duration::from_millis(500),
                control_snapshot: control_snapshot(true, false, false),
                poll_interval,
            }),
            DeepReviewReviewerAdmissionQueueRuntimeStep::Paused {
                queue_elapsed_ms: 500,
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                next_sleep: poll_interval,
            }
        );

        assert_eq!(
            runtime.begin_step(DeepReviewReviewerAdmissionQueueRuntimeInput {
                now: start + Duration::from_millis(1_500),
                control_snapshot: control_snapshot(true, false, false),
                poll_interval,
            }),
            DeepReviewReviewerAdmissionQueueRuntimeStep::Paused {
                queue_elapsed_ms: 500,
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                next_sleep: poll_interval,
            }
        );
    }

    #[test]
    fn reviewer_admission_queue_runtime_limits_sleep_to_remaining_wait() {
        let start = Instant::now();
        let mut runtime = DeepReviewReviewerAdmissionQueueRuntime::start(
            start,
            DeepReviewCapacityQueueReason::LocalConcurrencyCap,
            Duration::from_secs(1),
            Some(3),
            false,
        );
        let poll_interval = Duration::from_millis(100);

        let step = runtime.begin_step(DeepReviewReviewerAdmissionQueueRuntimeInput {
            now: start + Duration::from_millis(950),
            control_snapshot: control_snapshot(false, false, false),
            poll_interval,
        });
        let expected_attempt = DeepReviewReviewerAdmissionTryAdmit {
            queue_elapsed_ms: 950,
            queue_expired: false,
        };
        assert_eq!(
            step,
            DeepReviewReviewerAdmissionQueueRuntimeStep::TryAdmit {
                queue_elapsed_ms: 950,
                attempt: expected_attempt,
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
            }
        );

        assert_eq!(
            runtime.after_blocked_attempt(
                expected_attempt,
                DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                1,
                poll_interval,
            ),
            DeepReviewReviewerAdmissionQueueRuntimeBlockedStep::Queued {
                queue_elapsed_ms: 950,
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                next_sleep: Duration::from_millis(50),
            }
        );
    }

    #[test]
    fn reviewer_admission_queue_runtime_expires_with_retry_after_hint() {
        let start = Instant::now();
        let mut runtime = DeepReviewReviewerAdmissionQueueRuntime::start(
            start,
            DeepReviewCapacityQueueReason::LocalConcurrencyCap,
            Duration::from_secs(1),
            Some(3),
            false,
        );

        let step = runtime.begin_step(DeepReviewReviewerAdmissionQueueRuntimeInput {
            now: start + Duration::from_millis(1_000),
            control_snapshot: control_snapshot(false, false, false),
            poll_interval: Duration::from_millis(100),
        });
        let attempt = match step {
            DeepReviewReviewerAdmissionQueueRuntimeStep::TryAdmit { attempt, .. } => attempt,
            other => panic!("expected reviewer admission attempt, got {other:?}"),
        };

        assert_eq!(
            runtime.after_blocked_attempt(
                attempt,
                DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                0,
                Duration::from_millis(100),
            ),
            DeepReviewReviewerAdmissionQueueRuntimeBlockedStep::CapacityExpired {
                queue_elapsed_ms: 1_000,
                capacity_reason: DeepReviewCapacityQueueReason::LocalConcurrencyCap,
                retry_after_seconds: Some(3),
            }
        );
    }
}
