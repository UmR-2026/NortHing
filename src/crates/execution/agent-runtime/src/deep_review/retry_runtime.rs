//! Provider-capacity retry runtime, retry coverage/scope validation, queue
//! wait timing primitive, and shared control-snapshot decision.
//!
//! This module groups the cross-cutting retry semantics:
//! - `QueueWaitTimer` / `QueueWaitSnapshot` — pure wait timing primitive
//!   shared by `provider_capacity_queue` and `reviewer_admission_queue`
//! - `DeepReviewProviderCapacityRetryRuntime` — retry-attempt tracker used
//!   after each provider error
//! - `ensure_deep_review_retry_coverage` / `prompt_with_deep_review_retry_scope`
//!   — retry-coverage validation and prompt shaping
//! - `provider_capacity_queue_wait_seconds` / `..._for_attempt` — backoff math
//! - `decide_queue_control_step` — shared control-snapshot decision
//! - `local_reviewer_capacity_queue_decision` /
//!   `provider_capacity_wait_can_wake_on_active_reviewer_release` — small
//!   cross-sibling helpers

use super::task_completion_and_cache::manifest_packet_by_id;
use super::types::DeepReviewQueueWaitSkipReason;
use super::{
    DeepReviewCapacityQueueDecision, DeepReviewCapacityQueueReason, DeepReviewConcurrencyPolicy,
    DeepReviewPolicyViolation, DeepReviewQueueControlSnapshot,
};
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};

pub const DEEP_REVIEW_PROVIDER_CAPACITY_MAX_RETRY_ATTEMPTS: usize = 3;
const DEEP_REVIEW_PROVIDER_CAPACITY_BACKOFF_MULTIPLIER: u64 = 3;
const DEEP_REVIEW_PROVIDER_CAPACITY_MAX_BACKOFF_SECONDS: u64 = 600;

#[derive(Debug, Clone)]
pub(super) struct QueueWaitTimer {
    started_at: Instant,
    paused_since: Option<Instant>,
    paused_total: Duration,
}

impl QueueWaitTimer {
    pub(super) fn start(now: Instant) -> Self {
        Self {
            started_at: now,
            paused_since: None,
            paused_total: Duration::ZERO,
        }
    }

    pub(super) fn snapshot(&self, now: Instant) -> QueueWaitSnapshot {
        let active_pause = self
            .paused_since
            .map(|paused_at| now.saturating_duration_since(paused_at))
            .unwrap_or_default();
        let queue_elapsed = now
            .saturating_duration_since(self.started_at)
            .saturating_sub(self.paused_total)
            .saturating_sub(active_pause);

        QueueWaitSnapshot {
            queue_elapsed,
            queue_elapsed_ms: u64::try_from(queue_elapsed.as_millis()).unwrap_or(u64::MAX),
        }
    }

    pub(super) fn pause(&mut self, now: Instant) {
        if self.paused_since.is_none() {
            self.paused_since = Some(now);
        }
    }

    pub(super) fn continue_now(&mut self, now: Instant) {
        if let Some(paused_at) = self.paused_since.take() {
            self.paused_total += now.saturating_duration_since(paused_at);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct QueueWaitSnapshot {
    pub(super) queue_elapsed: Duration,
    pub(super) queue_elapsed_ms: u64,
}

impl QueueWaitSnapshot {
    pub(super) fn is_expired(self, max_wait: Duration) -> bool {
        self.queue_elapsed >= max_wait
    }
}

pub fn provider_capacity_wait_can_wake_on_active_reviewer_release(reason: DeepReviewCapacityQueueReason) -> bool {
    matches!(
        reason,
        DeepReviewCapacityQueueReason::ProviderConcurrencyLimit | DeepReviewCapacityQueueReason::TemporaryOverload
    )
}

pub fn decide_queue_control_step(
    control_snapshot: &DeepReviewQueueControlSnapshot,
    is_optional_reviewer: bool,
) -> super::types::DeepReviewQueueControlStepDecision {
    if control_snapshot.cancelled || (is_optional_reviewer && control_snapshot.skip_optional) {
        return super::types::DeepReviewQueueControlStepDecision::Skipped {
            skip_reason: if control_snapshot.cancelled {
                DeepReviewQueueWaitSkipReason::UserCancelled
            } else {
                DeepReviewQueueWaitSkipReason::OptionalSkipped
            },
        };
    }

    if control_snapshot.paused {
        return super::types::DeepReviewQueueControlStepDecision::Paused;
    }

    super::types::DeepReviewQueueControlStepDecision::Continue
}

fn is_retryable_capacity_reason(reason: &str) -> bool {
    matches!(
        reason,
        "local_concurrency_cap"
            | "launch_batch_blocked"
            | "provider_rate_limit"
            | "provider_concurrency_limit"
            | "retry_after"
            | "temporary_overload"
    )
}

pub fn ensure_deep_review_retry_coverage(
    input: &Value,
    subagent_type: &str,
    run_manifest: Option<&Value>,
) -> Result<Vec<String>, DeepReviewPolicyViolation> {
    let Some(coverage) =
        super::task_completion_and_cache::value_for_any_key(input, &["retry_coverage", "retryCoverage"])
    else {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_missing_coverage",
            "DeepReview retry requires structured retry_coverage metadata",
        ));
    };
    let packet_id =
        super::task_completion_and_cache::string_for_any_key(coverage, &["source_packet_id", "sourcePacketId"])
            .ok_or_else(|| {
                DeepReviewPolicyViolation::new(
                    "deep_review_retry_missing_packet_id",
                    "DeepReview retry coverage requires source_packet_id",
                )
            })?;
    let source_status =
        super::task_completion_and_cache::string_for_any_key(coverage, &["source_status", "sourceStatus"]).ok_or_else(
            || {
                DeepReviewPolicyViolation::new(
                    "deep_review_retry_missing_status",
                    "DeepReview retry coverage requires source_status",
                )
            },
        )?;
    match source_status {
        "partial_timeout" => {}
        "capacity_skipped" => {
            let capacity_reason =
                super::task_completion_and_cache::string_for_any_key(coverage, &["capacity_reason", "capacityReason"])
                    .unwrap_or("");
            if !is_retryable_capacity_reason(capacity_reason) {
                return Err(DeepReviewPolicyViolation::new(
                    "deep_review_retry_non_retryable_status",
                    format!(
                        "DeepReview retry cannot redispatch non-transient capacity reason '{}'",
                        capacity_reason
                    ),
                ));
            }
        }
        other => {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_non_retryable_status",
                format!(
                    "DeepReview retry only supports partial_timeout or transient capacity failures, not '{}'",
                    other
                ),
            ));
        }
    }

    let packet = manifest_packet_by_id(run_manifest, packet_id, subagent_type).ok_or_else(|| {
        DeepReviewPolicyViolation::new(
            "deep_review_retry_unknown_packet",
            format!(
                "DeepReview retry source packet '{}' does not match reviewer '{}'",
                packet_id, subagent_type
            ),
        )
    })?;
    let original_files = super::task_completion_and_cache::file_paths_for_manifest_packet(packet)?;
    ensure_deep_review_retry_timeout(input, packet)?;
    let retry_scope_files = super::task_completion_and_cache::string_array_for_any_key(
        coverage,
        &["retry_scope_files", "retryScopeFiles"],
    )?;
    let covered_files =
        super::task_completion_and_cache::string_array_for_any_key(coverage, &["covered_files", "coveredFiles"])?;
    if retry_scope_files.is_empty() {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_empty_scope",
            "DeepReview retry requires at least one retry_scope_files entry",
        ));
    }

    let original_file_set: HashSet<&str> = original_files.iter().map(String::as_str).collect();
    let mut retry_file_set = HashSet::new();
    for file in &retry_scope_files {
        if !retry_file_set.insert(file.as_str()) {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_duplicate_scope_file",
                format!("DeepReview retry scope repeats file '{}'", file),
            ));
        }
        if !original_file_set.contains(file.as_str()) {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_scope_outside_packet",
                format!(
                    "DeepReview retry file '{}' is outside source packet '{}'",
                    file, packet_id
                ),
            ));
        }
    }
    if retry_scope_files.len() >= original_files.len() {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_scope_not_reduced",
            "DeepReview retry_scope_files must be smaller than the source packet scope",
        ));
    }

    for file in &covered_files {
        if !original_file_set.contains(file.as_str()) {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_coverage_outside_packet",
                format!(
                    "DeepReview retry covered file '{}' is outside source packet '{}'",
                    file, packet_id
                ),
            ));
        }
        if retry_file_set.contains(file.as_str()) {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_coverage_overlaps_scope",
                format!(
                    "DeepReview retry covered file '{}' cannot also be in retry_scope_files",
                    file
                ),
            ));
        }
    }

    Ok(retry_scope_files)
}

fn ensure_deep_review_retry_timeout(input: &Value, packet: &Value) -> Result<(), DeepReviewPolicyViolation> {
    let retry_timeout_seconds =
        super::task_completion_and_cache::u64_for_any_key(input, &["timeout_seconds", "timeoutSeconds"]).unwrap_or(0);
    if retry_timeout_seconds == 0 {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_timeout_required",
            "DeepReview retry requires a positive timeout_seconds value",
        ));
    }

    let source_timeout_seconds =
        super::task_completion_and_cache::u64_for_any_key(packet, &["timeoutSeconds", "timeout_seconds"]).unwrap_or(0);
    if source_timeout_seconds > 0 && retry_timeout_seconds >= source_timeout_seconds {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_timeout_not_reduced",
            format!(
                "DeepReview retry timeout_seconds ({}) must be lower than source timeout ({})",
                retry_timeout_seconds, source_timeout_seconds
            ),
        ));
    }

    Ok(())
}

pub fn prompt_with_deep_review_retry_scope(prompt: &str, retry_scope_files: &[String]) -> String {
    let mut scoped_prompt = String::new();
    scoped_prompt.push_str("<deep_review_retry_scope>\n");
    scoped_prompt.push_str(
        "This is a bounded DeepReview retry. Review only the following retry_scope_files and treat any other files as background context only:\n",
    );
    for file in retry_scope_files {
        scoped_prompt.push_str("- ");
        scoped_prompt.push_str(file);
        scoped_prompt.push('\n');
    }
    scoped_prompt.push_str("</deep_review_retry_scope>\n\n");
    scoped_prompt.push_str(prompt);
    scoped_prompt
}

pub fn provider_capacity_queue_wait_seconds(
    decision: &DeepReviewCapacityQueueDecision,
    conc_policy: &DeepReviewConcurrencyPolicy,
) -> Option<u64> {
    if !decision.queueable || conc_policy.max_queue_wait_seconds == 0 {
        return None;
    }

    match decision.reason? {
        DeepReviewCapacityQueueReason::ProviderRateLimit
        | DeepReviewCapacityQueueReason::ProviderConcurrencyLimit
        | DeepReviewCapacityQueueReason::RetryAfter
        | DeepReviewCapacityQueueReason::TemporaryOverload => {}
        DeepReviewCapacityQueueReason::LocalConcurrencyCap | DeepReviewCapacityQueueReason::LaunchBatchBlocked => {
            return None
        }
    }

    Some(
        decision
            .retry_after_seconds
            .unwrap_or(conc_policy.max_queue_wait_seconds)
            .min(conc_policy.max_queue_wait_seconds),
    )
    .filter(|seconds| *seconds > 0)
}

pub fn provider_capacity_queue_wait_seconds_for_attempt(
    decision: &DeepReviewCapacityQueueDecision,
    conc_policy: &DeepReviewConcurrencyPolicy,
    retry_attempt_index: usize,
) -> Option<u64> {
    let base_wait_seconds = provider_capacity_queue_wait_seconds(decision, conc_policy)?;
    if decision.retry_after_seconds.is_some() {
        return Some(base_wait_seconds);
    }

    let multiplier = DEEP_REVIEW_PROVIDER_CAPACITY_BACKOFF_MULTIPLIER
        .saturating_pow(u32::try_from(retry_attempt_index).unwrap_or(u32::MAX).min(8));
    Some(
        base_wait_seconds
            .saturating_mul(multiplier)
            .min(DEEP_REVIEW_PROVIDER_CAPACITY_MAX_BACKOFF_SECONDS),
    )
    .filter(|seconds| *seconds > 0)
}

#[derive(Debug, Clone, Default)]
pub struct DeepReviewProviderCapacityRetryRuntime {
    retry_attempts: usize,
    queue_elapsed_ms: u64,
    last_retry_reason: Option<DeepReviewCapacityQueueReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepReviewProviderCapacityRetryDecision {
    NotQueueable,
    WaitForCapacity {
        reason: DeepReviewCapacityQueueReason,
        max_wait_seconds: u64,
    },
    CapacitySkipped {
        reason: DeepReviewCapacityQueueReason,
        queue_elapsed_ms: u64,
    },
}

impl DeepReviewProviderCapacityRetryRuntime {
    pub fn decide_after_error(
        &self,
        decision: &DeepReviewCapacityQueueDecision,
        conc_policy: &DeepReviewConcurrencyPolicy,
    ) -> DeepReviewProviderCapacityRetryDecision {
        let Some(reason) = decision.queueable.then_some(decision.reason).flatten() else {
            return DeepReviewProviderCapacityRetryDecision::NotQueueable;
        };

        if self.retry_attempts >= DEEP_REVIEW_PROVIDER_CAPACITY_MAX_RETRY_ATTEMPTS {
            return DeepReviewProviderCapacityRetryDecision::CapacitySkipped {
                reason,
                queue_elapsed_ms: self.queue_elapsed_ms,
            };
        }

        match provider_capacity_queue_wait_seconds_for_attempt(decision, conc_policy, self.retry_attempts) {
            Some(max_wait_seconds) => DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason,
                max_wait_seconds,
            },
            None => DeepReviewProviderCapacityRetryDecision::CapacitySkipped {
                reason,
                queue_elapsed_ms: 0,
            },
        }
    }

    pub fn record_ready_to_retry(
        &mut self,
        reason: DeepReviewCapacityQueueReason,
        queue_elapsed_ms: u64,
        early_capacity_probe: bool,
    ) -> u64 {
        self.queue_elapsed_ms = self.queue_elapsed_ms.saturating_add(queue_elapsed_ms);
        self.last_retry_reason = Some(reason);
        if !early_capacity_probe {
            self.retry_attempts = self.retry_attempts.saturating_add(1);
        }
        self.queue_elapsed_ms
    }

    pub fn record_queue_skipped(&mut self, queue_elapsed_ms: u64) -> u64 {
        self.queue_elapsed_ms = self.queue_elapsed_ms.saturating_add(queue_elapsed_ms);
        self.queue_elapsed_ms
    }

    pub fn last_retry_reason(&self) -> Option<DeepReviewCapacityQueueReason> {
        self.last_retry_reason
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::DeepReviewQueueControlStepDecision;
    use super::super::{
        classify_deep_review_capacity_error, DeepReviewCapacityFailFastReason, DeepReviewCapacityQueueDecision,
        DeepReviewCapacityQueueReason, DeepReviewConcurrencyPolicy, DeepReviewQueueControlSnapshot,
        DeepReviewQueueWaitSkipReason,
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

    fn provider_retry_policy(max_queue_wait_seconds: u64) -> DeepReviewConcurrencyPolicy {
        DeepReviewConcurrencyPolicy {
            max_parallel_instances: 3,
            stagger_seconds: 0,
            max_queue_wait_seconds,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        }
    }

    #[test]
    fn provider_capacity_retry_runtime_owns_backoff_and_attempt_limit() {
        let policy = provider_retry_policy(60);
        let decision = classify_deep_review_capacity_error("429", "too many concurrent requests", None);
        let mut runtime = DeepReviewProviderCapacityRetryRuntime::default();

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason: DeepReviewCapacityQueueReason::ProviderRateLimit,
                max_wait_seconds: 60,
            }
        );
        assert_eq!(
            runtime.record_ready_to_retry(DeepReviewCapacityQueueReason::ProviderRateLimit, 10, false,),
            10
        );

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason: DeepReviewCapacityQueueReason::ProviderRateLimit,
                max_wait_seconds: 180,
            }
        );
        runtime.record_ready_to_retry(DeepReviewCapacityQueueReason::ProviderRateLimit, 20, false);
        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason: DeepReviewCapacityQueueReason::ProviderRateLimit,
                max_wait_seconds: 540,
            }
        );
        runtime.record_ready_to_retry(DeepReviewCapacityQueueReason::ProviderRateLimit, 30, false);

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::CapacitySkipped {
                reason: DeepReviewCapacityQueueReason::ProviderRateLimit,
                queue_elapsed_ms: 60,
            }
        );
        assert_eq!(
            runtime.last_retry_reason(),
            Some(DeepReviewCapacityQueueReason::ProviderRateLimit)
        );
    }

    #[test]
    fn provider_capacity_retry_runtime_keeps_early_probe_attempt_free() {
        let policy = provider_retry_policy(60);
        let decision = classify_deep_review_capacity_error("overloaded", "temporary overload", None);
        let mut runtime = DeepReviewProviderCapacityRetryRuntime::default();

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason: DeepReviewCapacityQueueReason::TemporaryOverload,
                max_wait_seconds: 60,
            }
        );
        runtime.record_ready_to_retry(DeepReviewCapacityQueueReason::TemporaryOverload, 15, true);

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::WaitForCapacity {
                reason: DeepReviewCapacityQueueReason::TemporaryOverload,
                max_wait_seconds: 60,
            }
        );
    }

    #[test]
    fn provider_capacity_retry_runtime_accumulates_skipped_queue_elapsed() {
        let mut runtime = DeepReviewProviderCapacityRetryRuntime::default();

        assert_eq!(runtime.record_queue_skipped(25), 25);
        assert_eq!(runtime.record_queue_skipped(u64::MAX), u64::MAX);
    }

    #[test]
    fn provider_capacity_retry_runtime_rejects_fail_fast_decisions() {
        let policy = provider_retry_policy(60);
        let decision = DeepReviewCapacityQueueDecision::fail_fast(DeepReviewCapacityFailFastReason::InvalidModel);
        let runtime = DeepReviewProviderCapacityRetryRuntime::default();

        assert_eq!(
            runtime.decide_after_error(&decision, &policy),
            DeepReviewProviderCapacityRetryDecision::NotQueueable
        );
    }

    #[test]
    fn queue_control_decision_prefers_cancel_before_pause() {
        assert_eq!(
            decide_queue_control_step(&control_snapshot(true, true, true), true),
            DeepReviewQueueControlStepDecision::Skipped {
                skip_reason: DeepReviewQueueWaitSkipReason::UserCancelled
            }
        );
    }

    #[test]
    fn queue_control_decision_pause_applies_after_skip_checks() {
        assert_eq!(
            decide_queue_control_step(&control_snapshot(true, false, true), false),
            DeepReviewQueueControlStepDecision::Paused
        );
    }

    #[test]
    fn queue_wait_timer_excludes_paused_duration() {
        let start = Instant::now();
        let mut timer = QueueWaitTimer::start(start);

        let before_pause = start + Duration::from_millis(1_200);
        assert_eq!(timer.snapshot(before_pause).queue_elapsed, Duration::from_millis(1_200));

        timer.pause(before_pause);
        let during_pause = start + Duration::from_millis(5_200);
        assert_eq!(timer.snapshot(during_pause).queue_elapsed, Duration::from_millis(1_200));

        timer.continue_now(during_pause);
        let after_resume = start + Duration::from_millis(6_200);
        let snapshot = timer.snapshot(after_resume);
        assert_eq!(snapshot.queue_elapsed, Duration::from_millis(2_200));
        assert_eq!(snapshot.queue_elapsed_ms, 2_200);
    }

    #[test]
    fn queue_wait_timer_pause_and_continue_are_idempotent() {
        let start = Instant::now();
        let mut timer = QueueWaitTimer::start(start);

        let first_pause = start + Duration::from_millis(500);
        let second_pause = start + Duration::from_millis(900);
        timer.pause(first_pause);
        timer.pause(second_pause);

        let resume = start + Duration::from_millis(1_500);
        timer.continue_now(resume);
        timer.continue_now(resume + Duration::from_millis(300));

        let snapshot = timer.snapshot(start + Duration::from_millis(2_000));
        assert_eq!(snapshot.queue_elapsed, Duration::from_millis(1_000));
        assert!(!snapshot.is_expired(Duration::from_millis(1_001)));
        assert!(snapshot.is_expired(Duration::from_millis(1_000)));
    }
}
