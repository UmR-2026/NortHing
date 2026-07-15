use super::super::concurrency_policy::DeepReviewEffectiveConcurrencySnapshot;
use super::super::diagnostics::DeepReviewRuntimeDiagnostics;
use super::super::execution_policy::{DeepReviewExecutionPolicy, DeepReviewPolicyViolation, DeepReviewSubagentRole};
use super::super::queue::DeepReviewCapacityQueueReason;
use super::super::shared_context::DeepReviewSharedContextMeasurementSnapshot;
use super::budget_calc::{
    effective_concurrency_snapshot_impl, record_effective_concurrency_capacity_error_impl,
    record_effective_concurrency_success_impl, set_effective_concurrency_user_override_impl,
};
use super::budget_observability::{record_shared_context_tool_use_impl, shared_context_measurement_snapshot_from_turn};
use super::budget_types::{DeepReviewActiveReviewerGuard, DeepReviewTurnBudget, BUDGET_TTL, PRUNE_INTERVAL};
use dashmap::DashMap;
use std::sync::Mutex;
use std::time::Instant;

pub struct DeepReviewBudgetTracker {
    pub(crate) turns: DashMap<String, DeepReviewTurnBudget>,
    pub(crate) last_pruned_at: Mutex<Instant>,
}

impl Default for DeepReviewBudgetTracker {
    fn default() -> Self {
        Self {
            turns: DashMap::new(),
            last_pruned_at: Mutex::new(Instant::now()),
        }
    }
}

impl DeepReviewBudgetTracker {
    fn record_reason_count(
        counts: &mut std::collections::BTreeMap<String, usize>,
        reason: DeepReviewCapacityQueueReason,
    ) {
        *counts.entry(reason.as_snake_case().to_string()).or_insert(0) += 1;
    }

    fn update_runtime_diagnostics(&self, parent_dialog_turn_id: &str, update: impl FnOnce(&mut DeepReviewTurnBudget)) {
        if parent_dialog_turn_id.trim().is_empty() {
            return;
        }

        let now = Instant::now();
        if let Ok(last_pruned) = self.last_pruned_at.lock() {
            if now.saturating_duration_since(*last_pruned) >= PRUNE_INTERVAL {
                drop(last_pruned);
                self.prune_stale(now);
            }
        }

        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));
        update(&mut budget);
        budget.updated_at = now;
    }

    pub fn record_runtime_queue_wait(&self, parent_dialog_turn_id: &str, queue_elapsed_ms: u64) {
        if queue_elapsed_ms == 0 {
            return;
        }
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.queue_wait_count = budget.runtime_diagnostics.queue_wait_count.saturating_add(1);
            budget.runtime_diagnostics.queue_wait_total_ms = budget
                .runtime_diagnostics
                .queue_wait_total_ms
                .saturating_add(queue_elapsed_ms);
            budget.runtime_diagnostics.queue_wait_max_ms =
                budget.runtime_diagnostics.queue_wait_max_ms.max(queue_elapsed_ms);
        });
    }

    pub fn record_runtime_provider_capacity_queue(
        &self,
        parent_dialog_turn_id: &str,
        reason: DeepReviewCapacityQueueReason,
    ) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.provider_capacity_queue_count = budget
                .runtime_diagnostics
                .provider_capacity_queue_count
                .saturating_add(1);
            Self::record_reason_count(
                &mut budget.runtime_diagnostics.provider_capacity_queue_reason_counts,
                reason,
            );
        });
    }

    pub fn record_runtime_provider_capacity_retry(
        &self,
        parent_dialog_turn_id: &str,
        reason: DeepReviewCapacityQueueReason,
    ) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.provider_capacity_retry_count = budget
                .runtime_diagnostics
                .provider_capacity_retry_count
                .saturating_add(1);
            Self::record_reason_count(
                &mut budget.runtime_diagnostics.provider_capacity_retry_reason_counts,
                reason,
            );
        });
    }

    pub fn record_runtime_provider_capacity_retry_success(
        &self,
        parent_dialog_turn_id: &str,
        reason: DeepReviewCapacityQueueReason,
    ) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.provider_capacity_retry_success_count = budget
                .runtime_diagnostics
                .provider_capacity_retry_success_count
                .saturating_add(1);
            Self::record_reason_count(
                &mut budget.runtime_diagnostics.provider_capacity_retry_success_reason_counts,
                reason,
            );
        });
    }

    pub fn record_runtime_capacity_skip(&self, parent_dialog_turn_id: &str, reason: DeepReviewCapacityQueueReason) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.capacity_skip_count =
                budget.runtime_diagnostics.capacity_skip_count.saturating_add(1);
            Self::record_reason_count(&mut budget.runtime_diagnostics.capacity_skip_reason_counts, reason);
        });
    }

    pub fn record_runtime_manual_queue_action(&self, parent_dialog_turn_id: &str) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.manual_queue_action_count =
                budget.runtime_diagnostics.manual_queue_action_count.saturating_add(1);
        });
    }

    pub fn record_runtime_manual_retry(&self, parent_dialog_turn_id: &str) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.manual_retry_count =
                budget.runtime_diagnostics.manual_retry_count.saturating_add(1);
        });
    }

    pub fn record_runtime_auto_retry(&self, parent_dialog_turn_id: &str) {
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            budget.runtime_diagnostics.auto_retry_count = budget.runtime_diagnostics.auto_retry_count.saturating_add(1);
        });
    }

    pub fn record_runtime_auto_retry_suppressed(&self, parent_dialog_turn_id: &str, reason: &str) {
        let reason = reason.trim();
        if reason.is_empty() {
            return;
        }
        self.update_runtime_diagnostics(parent_dialog_turn_id, |budget| {
            *budget
                .runtime_diagnostics
                .auto_retry_suppressed_reason_counts
                .entry(reason.to_string())
                .or_insert(0) += 1;
        });
    }

    pub fn runtime_diagnostics_snapshot(&self, parent_dialog_turn_id: &str) -> Option<DeepReviewRuntimeDiagnostics> {
        let budget = self.turns.get(parent_dialog_turn_id)?;
        let mut diagnostics = budget.runtime_diagnostics.clone();
        let shared_context_snapshot = shared_context_measurement_snapshot_from_turn(Some(&budget));
        diagnostics.merge_shared_context_counts(
            shared_context_snapshot.total_calls,
            shared_context_snapshot.duplicate_calls,
            shared_context_snapshot.duplicate_context_count,
        );
        (!diagnostics.is_empty()).then_some(diagnostics)
    }

    pub fn turn_elapsed_seconds(&self, parent_dialog_turn_id: &str) -> Option<u64> {
        let budget = self.turns.get(parent_dialog_turn_id)?;
        Some(Instant::now().saturating_duration_since(budget.created_at).as_secs())
    }

    pub fn record_shared_context_tool_use(
        &self,
        parent_dialog_turn_id: &str,
        subagent_type: &str,
        tool_name: &str,
        file_path: &str,
    ) -> DeepReviewSharedContextMeasurementSnapshot {
        record_shared_context_tool_use_impl(self, parent_dialog_turn_id, subagent_type, tool_name, file_path)
    }

    pub fn shared_context_measurement_snapshot(
        &self,
        parent_dialog_turn_id: &str,
    ) -> DeepReviewSharedContextMeasurementSnapshot {
        shared_context_measurement_snapshot_from_turn(self.turns.get(parent_dialog_turn_id).map(|b| b).as_deref())
    }

    pub fn record_task(
        &self,
        parent_dialog_turn_id: &str,
        policy: &DeepReviewExecutionPolicy,
        role: DeepReviewSubagentRole,
        subagent_type: &str,
        is_retry: bool,
    ) -> Result<(), DeepReviewPolicyViolation> {
        super::budget_enforce::record_task_impl(self, parent_dialog_turn_id, policy, role, subagent_type, is_retry)
    }

    pub fn record_concurrency_cap_rejection(&self, parent_dialog_turn_id: &str) {
        if parent_dialog_turn_id.trim().is_empty() {
            return;
        }

        let now = Instant::now();
        if let Ok(last_pruned) = self.last_pruned_at.lock() {
            if now.saturating_duration_since(*last_pruned) >= PRUNE_INTERVAL {
                drop(last_pruned);
                self.prune_stale(now);
            }
        }

        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));
        budget.concurrency_cap_rejections += 1;
        budget.updated_at = now;
    }

    fn record_capacity_skip_inner(&self, parent_dialog_turn_id: &str, reason: Option<DeepReviewCapacityQueueReason>) {
        if parent_dialog_turn_id.trim().is_empty() {
            return;
        }

        let now = Instant::now();
        if let Ok(last_pruned) = self.last_pruned_at.lock() {
            if now.saturating_duration_since(*last_pruned) >= PRUNE_INTERVAL {
                drop(last_pruned);
                self.prune_stale(now);
            }
        }

        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));
        budget.capacity_skips += 1;
        budget.runtime_diagnostics.capacity_skip_count =
            budget.runtime_diagnostics.capacity_skip_count.saturating_add(1);
        if let Some(reason) = reason {
            Self::record_reason_count(&mut budget.runtime_diagnostics.capacity_skip_reason_counts, reason);
        }
        budget.updated_at = now;
    }

    pub fn record_capacity_skip(&self, parent_dialog_turn_id: &str) {
        self.record_capacity_skip_inner(parent_dialog_turn_id, None);
    }

    pub fn record_capacity_skip_for_reason(&self, parent_dialog_turn_id: &str, reason: DeepReviewCapacityQueueReason) {
        self.record_capacity_skip_inner(parent_dialog_turn_id, Some(reason));
    }

    pub fn begin_active_reviewer<'a>(&'a self, parent_dialog_turn_id: &str) -> DeepReviewActiveReviewerGuard<'a> {
        let now = Instant::now();
        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));
        budget.active_reviewers = budget.active_reviewers.saturating_add(1);
        budget.updated_at = now;

        DeepReviewActiveReviewerGuard {
            tracker: self,
            parent_dialog_turn_id: parent_dialog_turn_id.to_string(),
            launch_batch: None,
            released: false,
        }
    }

    pub fn try_begin_active_reviewer<'a>(
        &'a self,
        parent_dialog_turn_id: &str,
        max_active_reviewers: usize,
    ) -> Option<DeepReviewActiveReviewerGuard<'a>> {
        let now = Instant::now();
        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));
        if budget.active_reviewers >= max_active_reviewers {
            return None;
        }

        budget.active_reviewers = budget.active_reviewers.saturating_add(1);
        budget.updated_at = now;
        Some(DeepReviewActiveReviewerGuard {
            tracker: self,
            parent_dialog_turn_id: parent_dialog_turn_id.to_string(),
            launch_batch: None,
            released: false,
        })
    }

    pub fn try_begin_active_reviewer_for_launch_batch<'a>(
        &'a self,
        parent_dialog_turn_id: &str,
        max_active_reviewers: usize,
        launch_batch: u64,
        _packet_id: Option<&str>,
    ) -> Result<Option<DeepReviewActiveReviewerGuard<'a>>, DeepReviewPolicyViolation> {
        let now = Instant::now();
        let mut budget = self
            .turns
            .entry(parent_dialog_turn_id.to_string())
            .or_insert_with(|| DeepReviewTurnBudget::new(now));

        if budget.active_reviewers >= max_active_reviewers {
            return Ok(None);
        }

        budget.active_reviewers = budget.active_reviewers.saturating_add(1);
        *budget.active_reviewer_launch_batches.entry(launch_batch).or_insert(0) += 1;
        budget.updated_at = now;
        Ok(Some(DeepReviewActiveReviewerGuard {
            tracker: self,
            parent_dialog_turn_id: parent_dialog_turn_id.to_string(),
            launch_batch: Some(launch_batch),
            released: false,
        }))
    }

    pub(crate) fn finish_active_reviewer(&self, parent_dialog_turn_id: &str, launch_batch: Option<u64>) {
        if let Some(mut budget) = self.turns.get_mut(parent_dialog_turn_id) {
            budget.active_reviewers = budget.active_reviewers.saturating_sub(1);
            if let Some(launch_batch) = launch_batch {
                let should_remove_batch =
                    if let Some(count) = budget.active_reviewer_launch_batches.get_mut(&launch_batch) {
                        *count = (*count).saturating_sub(1);
                        *count == 0
                    } else {
                        false
                    };
                if should_remove_batch {
                    budget.active_reviewer_launch_batches.remove(&launch_batch);
                }
            }
            budget.updated_at = Instant::now();
        }
    }

    pub(crate) fn prune_stale(&self, now: Instant) {
        self.turns
            .retain(|_, budget| now.saturating_duration_since(budget.updated_at) <= BUDGET_TTL);
        if let Ok(mut last_pruned) = self.last_pruned_at.lock() {
            *last_pruned = now;
        }
    }

    /// Explicitly clean up all budget tracking data.
    /// Call this when the application is shutting down or when the review session ends.
    pub fn cleanup(&self) {
        self.turns.clear();
        if let Ok(mut last_pruned) = self.last_pruned_at.lock() {
            *last_pruned = Instant::now();
        }
    }

    /// Returns the number of reviewer calls recorded for a given turn.
    /// Used by the concurrency enforcement to check if a new launch is allowed.
    pub fn active_reviewer_count(&self, parent_dialog_turn_id: &str) -> usize {
        self.turns
            .get(parent_dialog_turn_id)
            .map(|budget| budget.active_reviewers)
            .unwrap_or(0)
    }

    /// Returns true if a judge call has been recorded for a given turn.
    pub fn has_judge_been_launched(&self, parent_dialog_turn_id: &str) -> bool {
        self.turns
            .get(parent_dialog_turn_id)
            .map(|budget| budget.judge_calls > 0)
            .unwrap_or(false)
    }

    pub fn concurrency_cap_rejection_count(&self, parent_dialog_turn_id: &str) -> usize {
        self.turns
            .get(parent_dialog_turn_id)
            .map(|budget| budget.concurrency_cap_rejections)
            .unwrap_or(0)
    }

    pub fn capacity_skip_count(&self, parent_dialog_turn_id: &str) -> usize {
        self.turns
            .get(parent_dialog_turn_id)
            .map(|budget| budget.capacity_skips)
            .unwrap_or(0)
    }

    pub fn retries_used(&self, parent_dialog_turn_id: &str, subagent_type: &str) -> usize {
        self.turns
            .get(parent_dialog_turn_id)
            .map(|budget| budget.retries_used_by_subagent.get(subagent_type).copied().unwrap_or(0))
            .unwrap_or(0)
    }

    pub fn effective_concurrency_snapshot(
        &self,
        parent_dialog_turn_id: &str,
        configured_max_parallel_instances: usize,
    ) -> DeepReviewEffectiveConcurrencySnapshot {
        effective_concurrency_snapshot_impl(self, parent_dialog_turn_id, configured_max_parallel_instances)
    }

    pub fn effective_parallel_instances(
        &self,
        parent_dialog_turn_id: &str,
        configured_max_parallel_instances: usize,
    ) -> usize {
        self.effective_concurrency_snapshot(parent_dialog_turn_id, configured_max_parallel_instances)
            .effective_parallel_instances
    }

    pub fn record_effective_concurrency_capacity_error(
        &self,
        parent_dialog_turn_id: &str,
        configured_max_parallel_instances: usize,
        reason: DeepReviewCapacityQueueReason,
        retry_after: Option<std::time::Duration>,
    ) -> DeepReviewEffectiveConcurrencySnapshot {
        record_effective_concurrency_capacity_error_impl(
            self,
            parent_dialog_turn_id,
            configured_max_parallel_instances,
            reason,
            retry_after,
        )
    }

    pub fn record_effective_concurrency_success(
        &self,
        parent_dialog_turn_id: &str,
        configured_max_parallel_instances: usize,
    ) -> DeepReviewEffectiveConcurrencySnapshot {
        record_effective_concurrency_success_impl(self, parent_dialog_turn_id, configured_max_parallel_instances)
    }

    pub fn set_effective_concurrency_user_override(
        &self,
        parent_dialog_turn_id: &str,
        configured_max_parallel_instances: usize,
        user_override_parallel_instances: Option<usize>,
    ) -> DeepReviewEffectiveConcurrencySnapshot {
        set_effective_concurrency_user_override_impl(
            self,
            parent_dialog_turn_id,
            configured_max_parallel_instances,
            user_override_parallel_instances,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_batch_admission_allows_later_batch_when_reviewer_capacity_is_free() {
        let tracker = DeepReviewBudgetTracker::default();
        let turn_id = "turn-launch-batch-fill-free-slot";
        let _first_batch = tracker
            .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 1, Some("packet-a"))
            .expect("batch admission should not fail")
            .expect("first reviewer should start");

        let second_batch = tracker
            .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 2, Some("packet-b"))
            .expect("later batch admission should not fail when reviewer capacity is free");

        assert!(
            second_batch.is_some(),
            "later batch should fill a freed reviewer slot instead of waiting for the earlier batch to drain"
        );
    }

    #[test]
    fn launch_batch_admission_allows_same_batch_and_next_batch_after_release() {
        let tracker = DeepReviewBudgetTracker::default();
        let turn_id = "turn-launch-batch-release";
        let first = tracker
            .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 1, Some("packet-a"))
            .expect("first batch should not violate launch order")
            .expect("first reviewer should start");
        let second = tracker
            .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 1, Some("packet-b"))
            .expect("same batch should not violate launch order")
            .expect("second reviewer should start");
        assert!(
            tracker
                .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 1, Some("packet-c"))
                .expect("same batch should not violate launch order")
                .is_none(),
            "same-batch admission should still respect active reviewer capacity"
        );

        drop(first);
        drop(second);

        assert!(tracker
            .try_begin_active_reviewer_for_launch_batch(turn_id, 2, 2, Some("packet-c"))
            .expect("next batch should start after the previous batch releases")
            .is_some());
    }
}
