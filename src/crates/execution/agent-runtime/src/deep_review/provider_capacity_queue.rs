//! Provider-capacity queue runtime and step decisions.
//!
//! `DeepReviewProviderCapacityQueueRuntime` walks a single reviewer through
//! provider-side capacity back-pressure (rate limits, concurrency limits,
//! retry-after, temporary overload). Pure helper decisions
//! (`decide_provider_capacity_queue_step`,
//! `capacity_decision_for_provider_error_facts`,
//! `capacity_skip_result_for_provider_queue_outcome`) live here too.
//!
//! The wait timer primitive lives in `super::retry_runtime`; this file only
//! owns the runtime that uses it.

use super::retry_runtime::{
    decide_queue_control_step, provider_capacity_wait_can_wake_on_active_reviewer_release, QueueWaitTimer,
};
use super::types::{
    DeepReviewProviderCapacityErrorCategory, DeepReviewProviderCapacityErrorFacts,
    DeepReviewProviderCapacityQueueStepDecision, DeepReviewProviderCapacityQueueStepFacts,
    DeepReviewQueueControlStepDecision, DeepReviewQueueWaitSkipReason,
};
use super::{
    classify_deep_review_capacity_error, DeepReviewCapacityFailFastReason, DeepReviewCapacityQueueDecision,
    DeepReviewCapacityQueueReason, DeepReviewConcurrencyPolicy, DeepReviewQueueControlSnapshot,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DeepReviewProviderCapacityQueueRuntime {
    reason: DeepReviewCapacityQueueReason,
    timer: QueueWaitTimer,
    max_wait: Duration,
    initial_active_reviewer_count: usize,
    is_optional_reviewer: bool,
}

impl DeepReviewProviderCapacityQueueRuntime {
    pub fn start(
        now: Instant,
        reason: DeepReviewCapacityQueueReason,
        max_wait: Duration,
        initial_active_reviewer_count: usize,
        is_optional_reviewer: bool,
    ) -> Self {
        Self {
            reason,
            timer: QueueWaitTimer::start(now),
            max_wait,
            initial_active_reviewer_count,
            is_optional_reviewer,
        }
    }

    pub fn step(
        &mut self,
        input: DeepReviewProviderCapacityQueueRuntimeInput,
    ) -> DeepReviewProviderCapacityQueueRuntimeStep {
        let queue_snapshot = self.timer.snapshot(input.now);
        let queue_elapsed = queue_snapshot.queue_elapsed;
        let queue_elapsed_ms = queue_snapshot.queue_elapsed_ms;
        let queue_decision = decide_provider_capacity_queue_step(DeepReviewProviderCapacityQueueStepFacts {
            reason: self.reason,
            queue_expired: queue_snapshot.is_expired(self.max_wait),
            initial_active_reviewer_count: self.initial_active_reviewer_count,
            active_reviewer_count: input.active_reviewer_count,
            control_snapshot: input.control_snapshot,
            is_optional_reviewer: self.is_optional_reviewer,
        });

        match queue_decision {
            DeepReviewProviderCapacityQueueStepDecision::Skipped { skip_reason } => {
                DeepReviewProviderCapacityQueueRuntimeStep::Skipped {
                    queue_elapsed_ms,
                    skip_reason,
                }
            }
            DeepReviewProviderCapacityQueueStepDecision::Paused => {
                self.timer.pause(input.now);
                DeepReviewProviderCapacityQueueRuntimeStep::Paused {
                    queue_elapsed_ms,
                    next_sleep: input.poll_interval,
                }
            }
            DeepReviewProviderCapacityQueueStepDecision::ReadyToRetry { early_capacity_probe } => {
                self.timer.continue_now(input.now);
                DeepReviewProviderCapacityQueueRuntimeStep::ReadyToRetry {
                    queue_elapsed_ms,
                    early_capacity_probe,
                }
            }
            DeepReviewProviderCapacityQueueStepDecision::Queued => {
                self.timer.continue_now(input.now);
                DeepReviewProviderCapacityQueueRuntimeStep::Queued {
                    queue_elapsed_ms,
                    next_sleep: input.poll_interval.min(self.max_wait.saturating_sub(queue_elapsed)),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewProviderCapacityQueueRuntimeInput {
    pub now: Instant,
    pub active_reviewer_count: usize,
    pub control_snapshot: DeepReviewQueueControlSnapshot,
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewProviderCapacityQueueRuntimeStep {
    Skipped {
        queue_elapsed_ms: u64,
        skip_reason: DeepReviewQueueWaitSkipReason,
    },
    Paused {
        queue_elapsed_ms: u64,
        next_sleep: Duration,
    },
    ReadyToRetry {
        queue_elapsed_ms: u64,
        early_capacity_probe: bool,
    },
    Queued {
        queue_elapsed_ms: u64,
        next_sleep: Duration,
    },
}

pub fn capacity_decision_for_provider_error_facts(
    facts: DeepReviewProviderCapacityErrorFacts<'_>,
) -> DeepReviewCapacityQueueDecision {
    let decision =
        classify_deep_review_capacity_error(facts.provider_code, facts.provider_message, facts.retry_after_seconds);
    if decision.queueable
        || decision.fail_fast_reason != Some(DeepReviewCapacityFailFastReason::DeterministicProviderError)
    {
        return decision;
    }

    match facts.category {
        DeepReviewProviderCapacityErrorCategory::RateLimit => DeepReviewCapacityQueueDecision::queueable(
            DeepReviewCapacityQueueReason::ProviderRateLimit,
            decision.retry_after_seconds,
        ),
        DeepReviewProviderCapacityErrorCategory::ProviderUnavailable => DeepReviewCapacityQueueDecision::queueable(
            DeepReviewCapacityQueueReason::TemporaryOverload,
            decision.retry_after_seconds,
        ),
        DeepReviewProviderCapacityErrorCategory::Other => decision,
    }
}

pub fn decide_provider_capacity_queue_step(
    facts: DeepReviewProviderCapacityQueueStepFacts,
) -> DeepReviewProviderCapacityQueueStepDecision {
    match decide_queue_control_step(&facts.control_snapshot, facts.is_optional_reviewer) {
        DeepReviewQueueControlStepDecision::Skipped { skip_reason } => {
            return DeepReviewProviderCapacityQueueStepDecision::Skipped { skip_reason };
        }
        DeepReviewQueueControlStepDecision::Paused => {
            return DeepReviewProviderCapacityQueueStepDecision::Paused;
        }
        DeepReviewQueueControlStepDecision::Continue => {}
    }

    if facts.queue_expired {
        return DeepReviewProviderCapacityQueueStepDecision::ReadyToRetry {
            early_capacity_probe: false,
        };
    }

    if provider_capacity_wait_can_wake_on_active_reviewer_release(facts.reason)
        && facts.initial_active_reviewer_count > 0
        && facts.active_reviewer_count < facts.initial_active_reviewer_count
    {
        return DeepReviewProviderCapacityQueueStepDecision::ReadyToRetry {
            early_capacity_probe: true,
        };
    }

    DeepReviewProviderCapacityQueueStepDecision::Queued
}

pub fn capacity_skip_result_for_provider_queue_outcome(
    reason: DeepReviewCapacityQueueReason,
    subagent_type: &str,
    conc_policy: &DeepReviewConcurrencyPolicy,
    duration_ms: u128,
    queue_elapsed_ms: u64,
    terminal_skip_reason: Option<DeepReviewQueueWaitSkipReason>,
    effective_parallel_instances: usize,
) -> (Value, String) {
    let duration_ms = u64::try_from(duration_ms).unwrap_or(u64::MAX);
    let reason_code = reason.as_snake_case();
    let queue_skip_reason = match terminal_skip_reason {
        Some(DeepReviewQueueWaitSkipReason::UserCancelled) => "user_cancelled",
        Some(DeepReviewQueueWaitSkipReason::OptionalSkipped) => "optional_skipped",
        Some(DeepReviewQueueWaitSkipReason::QueueExpired) | None => reason_code,
    };
    let assistant_message = match terminal_skip_reason {
        Some(DeepReviewQueueWaitSkipReason::UserCancelled) => format!(
            "Subagent '{}' was skipped because the DeepReview provider capacity queue was cancelled by the user.\n<queue_result status=\"capacity_skipped\" reason=\"user_cancelled\" queue_elapsed_ms=\"{}\" />",
            subagent_type, queue_elapsed_ms
        ),
        Some(DeepReviewQueueWaitSkipReason::OptionalSkipped) => format!(
            "Subagent '{}' was skipped because optional DeepReview provider capacity retries were skipped by the user.\n<queue_result status=\"capacity_skipped\" reason=\"optional_skipped\" queue_elapsed_ms=\"{}\" />",
            subagent_type, queue_elapsed_ms
        ),
        Some(DeepReviewQueueWaitSkipReason::QueueExpired) | None => format!(
            "Subagent '{}' was skipped because the provider reported transient DeepReview capacity pressure.\n<queue_result status=\"capacity_skipped\" reason=\"{}\" queue_elapsed_ms=\"{}\" />",
            subagent_type, reason_code, queue_elapsed_ms
        ),
    };
    let data = json!({
        "duration": duration_ms,
        "status": "capacity_skipped",
        "queue_elapsed_ms": queue_elapsed_ms,
        "max_queue_wait_seconds": conc_policy.max_queue_wait_seconds,
        "queue_skip_reason": queue_skip_reason,
        "provider_capacity_reason": reason_code,
        "effective_parallel_instances": effective_parallel_instances
    });

    (data, assistant_message)
}

#[cfg(test)]
mod tests {
    use super::super::types::DeepReviewProviderCapacityErrorCategory;
    use super::super::{
        DeepReviewCapacityFailFastReason, DeepReviewCapacityQueueReason, DeepReviewQueueControlSnapshot,
    };
    use super::*;
    use std::time::{Duration, Instant};

    fn control_snapshot(paused: bool, cancelled: bool, skip_optional: bool) -> DeepReviewQueueControlSnapshot {
        DeepReviewQueueControlSnapshot {
            paused,
            cancelled,
            skip_optional,
        }
    }

    fn provider_queue_facts(reason: DeepReviewCapacityQueueReason) -> DeepReviewProviderCapacityQueueStepFacts {
        DeepReviewProviderCapacityQueueStepFacts {
            reason,
            queue_expired: false,
            initial_active_reviewer_count: 2,
            active_reviewer_count: 2,
            control_snapshot: control_snapshot(false, false, false),
            is_optional_reviewer: false,
        }
    }

    #[test]
    fn provider_error_decision_uses_structured_category_fallback() {
        let rate_limited = capacity_decision_for_provider_error_facts(DeepReviewProviderCapacityErrorFacts {
            provider_code: "provider_specific_code",
            provider_message: "provider returned an unmapped error",
            retry_after_seconds: None,
            category: DeepReviewProviderCapacityErrorCategory::RateLimit,
        });
        assert_eq!(
            rate_limited.reason,
            Some(DeepReviewCapacityQueueReason::ProviderRateLimit)
        );

        let unavailable = capacity_decision_for_provider_error_facts(DeepReviewProviderCapacityErrorFacts {
            provider_code: "unknown",
            provider_message: "upstream failed",
            retry_after_seconds: None,
            category: DeepReviewProviderCapacityErrorCategory::ProviderUnavailable,
        });
        assert_eq!(
            unavailable.reason,
            Some(DeepReviewCapacityQueueReason::TemporaryOverload)
        );
    }

    #[test]
    fn provider_error_decision_keeps_quota_fail_fast() {
        let decision = capacity_decision_for_provider_error_facts(DeepReviewProviderCapacityErrorFacts {
            provider_code: "1113",
            provider_message: "insufficient quota",
            retry_after_seconds: None,
            category: DeepReviewProviderCapacityErrorCategory::RateLimit,
        });

        assert!(!decision.queueable);
        assert_eq!(
            decision.fail_fast_reason,
            Some(DeepReviewCapacityFailFastReason::BillingOrQuota)
        );
    }

    #[test]
    fn provider_queue_decision_cancel_skips_before_other_states() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        facts.queue_expired = true;
        facts.active_reviewer_count = 1;
        facts.control_snapshot = control_snapshot(true, true, false);

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::Skipped {
                skip_reason: DeepReviewQueueWaitSkipReason::UserCancelled
            }
        );
    }

    #[test]
    fn provider_queue_decision_optional_skip_only_applies_to_optional_reviewers() {
        let mut mandatory = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        mandatory.control_snapshot = control_snapshot(false, false, true);
        assert_eq!(
            decide_provider_capacity_queue_step(mandatory),
            DeepReviewProviderCapacityQueueStepDecision::Queued
        );

        let mut optional = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        optional.control_snapshot = control_snapshot(false, false, true);
        optional.is_optional_reviewer = true;
        assert_eq!(
            decide_provider_capacity_queue_step(optional),
            DeepReviewProviderCapacityQueueStepDecision::Skipped {
                skip_reason: DeepReviewQueueWaitSkipReason::OptionalSkipped
            }
        );
    }

    #[test]
    fn provider_queue_decision_pause_wins_over_expiry_and_active_release() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        facts.queue_expired = true;
        facts.active_reviewer_count = 1;
        facts.control_snapshot = control_snapshot(true, false, false);

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::Paused
        );
    }

    #[test]
    fn provider_queue_decision_expiry_retries_without_early_probe() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        facts.queue_expired = true;
        facts.active_reviewer_count = 2;

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::ReadyToRetry {
                early_capacity_probe: false
            }
        );
    }

    #[test]
    fn provider_queue_decision_wakes_when_provider_capacity_can_free() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::ProviderConcurrencyLimit);
        facts.active_reviewer_count = 1;

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::ReadyToRetry {
                early_capacity_probe: true
            }
        );
    }

    #[test]
    fn provider_queue_decision_does_not_wake_retry_after_on_reviewer_release() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::RetryAfter);
        facts.active_reviewer_count = 1;

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::Queued
        );
    }

    #[test]
    fn provider_queue_decision_requires_existing_active_reviewer_before_wake() {
        let mut facts = provider_queue_facts(DeepReviewCapacityQueueReason::TemporaryOverload);
        facts.initial_active_reviewer_count = 0;
        facts.active_reviewer_count = 0;

        assert_eq!(
            decide_provider_capacity_queue_step(facts),
            DeepReviewProviderCapacityQueueStepDecision::Queued
        );
    }

    #[test]
    fn provider_capacity_queue_runtime_pauses_without_consuming_wait_budget() {
        let start = Instant::now();
        let mut runtime = DeepReviewProviderCapacityQueueRuntime::start(
            start,
            DeepReviewCapacityQueueReason::ProviderConcurrencyLimit,
            Duration::from_secs(2),
            2,
            false,
        );
        let poll_interval = Duration::from_millis(100);

        assert_eq!(
            runtime.step(DeepReviewProviderCapacityQueueRuntimeInput {
                now: start + Duration::from_millis(500),
                active_reviewer_count: 2,
                control_snapshot: control_snapshot(true, false, false),
                poll_interval,
            }),
            DeepReviewProviderCapacityQueueRuntimeStep::Paused {
                queue_elapsed_ms: 500,
                next_sleep: poll_interval,
            }
        );

        assert_eq!(
            runtime.step(DeepReviewProviderCapacityQueueRuntimeInput {
                now: start + Duration::from_millis(1_500),
                active_reviewer_count: 2,
                control_snapshot: control_snapshot(true, false, false),
                poll_interval,
            }),
            DeepReviewProviderCapacityQueueRuntimeStep::Paused {
                queue_elapsed_ms: 500,
                next_sleep: poll_interval,
            }
        );

        assert_eq!(
            runtime.step(DeepReviewProviderCapacityQueueRuntimeInput {
                now: start + Duration::from_millis(2_500),
                active_reviewer_count: 1,
                control_snapshot: control_snapshot(false, false, false),
                poll_interval,
            }),
            DeepReviewProviderCapacityQueueRuntimeStep::ReadyToRetry {
                queue_elapsed_ms: 500,
                early_capacity_probe: true,
            }
        );
    }

    #[test]
    fn provider_capacity_queue_runtime_limits_sleep_to_remaining_wait() {
        let start = Instant::now();
        let mut runtime = DeepReviewProviderCapacityQueueRuntime::start(
            start,
            DeepReviewCapacityQueueReason::RetryAfter,
            Duration::from_secs(1),
            2,
            false,
        );

        assert_eq!(
            runtime.step(DeepReviewProviderCapacityQueueRuntimeInput {
                now: start + Duration::from_millis(950),
                active_reviewer_count: 2,
                control_snapshot: control_snapshot(false, false, false),
                poll_interval: Duration::from_millis(100),
            }),
            DeepReviewProviderCapacityQueueRuntimeStep::Queued {
                queue_elapsed_ms: 950,
                next_sleep: Duration::from_millis(50),
            }
        );
    }
}
