use super::super::concurrency_policy::{DeepReviewEffectiveConcurrencySnapshot, DeepReviewEffectiveConcurrencyState};
use super::super::queue::DeepReviewCapacityQueueReason;
use super::budget_state::DeepReviewBudgetTracker;
use super::budget_types::DeepReviewTurnBudget;
use std::time::Instant;

pub(super) fn effective_concurrency_snapshot_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    configured_max_parallel_instances: usize,
) -> DeepReviewEffectiveConcurrencySnapshot {
    if parent_dialog_turn_id.trim().is_empty() {
        return DeepReviewEffectiveConcurrencyState::new(configured_max_parallel_instances).snapshot(Instant::now());
    }

    let now = Instant::now();
    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));
    budget.updated_at = now;
    budget
        .effective_concurrency_mut(configured_max_parallel_instances)
        .snapshot(now)
}

pub(super) fn record_effective_concurrency_capacity_error_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    configured_max_parallel_instances: usize,
    reason: DeepReviewCapacityQueueReason,
    retry_after: Option<std::time::Duration>,
) -> DeepReviewEffectiveConcurrencySnapshot {
    if parent_dialog_turn_id.trim().is_empty() {
        return DeepReviewEffectiveConcurrencyState::new(configured_max_parallel_instances).snapshot(Instant::now());
    }

    let now = Instant::now();
    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));
    budget.updated_at = now;
    let snapshot = {
        let state = budget.effective_concurrency_mut(configured_max_parallel_instances);
        state.record_capacity_error(
            matches!(reason, DeepReviewCapacityQueueReason::RetryAfter),
            retry_after,
            now,
        );
        state.snapshot(now)
    };
    budget
        .runtime_diagnostics
        .observe_effective_parallel(snapshot.effective_parallel_instances);
    snapshot
}

pub(super) fn record_effective_concurrency_success_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    configured_max_parallel_instances: usize,
) -> DeepReviewEffectiveConcurrencySnapshot {
    if parent_dialog_turn_id.trim().is_empty() {
        return DeepReviewEffectiveConcurrencyState::new(configured_max_parallel_instances).snapshot(Instant::now());
    }

    let now = Instant::now();
    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));
    budget.updated_at = now;
    let snapshot = {
        let state = budget.effective_concurrency_mut(configured_max_parallel_instances);
        state.record_success(now);
        state.snapshot(now)
    };
    budget
        .runtime_diagnostics
        .observe_effective_parallel(snapshot.effective_parallel_instances);
    snapshot
}

pub(super) fn set_effective_concurrency_user_override_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    configured_max_parallel_instances: usize,
    user_override_parallel_instances: Option<usize>,
) -> DeepReviewEffectiveConcurrencySnapshot {
    if parent_dialog_turn_id.trim().is_empty() {
        return DeepReviewEffectiveConcurrencyState::new(configured_max_parallel_instances).snapshot(Instant::now());
    }

    let now = Instant::now();
    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));
    budget.updated_at = now;
    let snapshot = {
        let state = budget.effective_concurrency_mut(configured_max_parallel_instances);
        state.set_user_override(user_override_parallel_instances);
        state.snapshot(now)
    };
    budget
        .runtime_diagnostics
        .observe_effective_parallel(snapshot.effective_parallel_instances);
    snapshot
}
