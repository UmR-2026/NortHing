use super::super::shared_context::{
    normalize_shared_context_file_path, normalize_shared_context_tool_name,
    shared_context_measurement_snapshot_from_uses, DeepReviewSharedContextKey,
    DeepReviewSharedContextMeasurementSnapshot,
};
use super::budget_state::DeepReviewBudgetTracker;
use super::budget_types::{DeepReviewTurnBudget, BUDGET_TTL};
use std::time::Instant;

pub(super) fn record_shared_context_tool_use_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    subagent_type: &str,
    tool_name: &str,
    file_path: &str,
) -> DeepReviewSharedContextMeasurementSnapshot {
    if parent_dialog_turn_id.trim().is_empty() {
        return DeepReviewSharedContextMeasurementSnapshot::default();
    }
    let Some(tool_name) = normalize_shared_context_tool_name(tool_name) else {
        return shared_context_measurement_snapshot_from_turn(
            tracker.turns.get(parent_dialog_turn_id).map(|b| b).as_deref(),
        );
    };
    let Some(file_path) = normalize_shared_context_file_path(file_path) else {
        return shared_context_measurement_snapshot_from_turn(
            tracker.turns.get(parent_dialog_turn_id).map(|b| b).as_deref(),
        );
    };

    let now = Instant::now();
    if let Ok(last_pruned) = tracker.last_pruned_at.lock() {
        if now.saturating_duration_since(*last_pruned) >= BUDGET_TTL {
            drop(last_pruned);
            tracker.prune_stale(now);
        }
    }

    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));
    let record = budget
        .shared_context_uses
        .entry(DeepReviewSharedContextKey {
            tool_name: tool_name.to_string(),
            file_path,
        })
        .or_default();
    record.call_count = record.call_count.saturating_add(1);
    if !subagent_type.trim().is_empty() {
        record.reviewer_types.insert(subagent_type.trim().to_string());
    }
    budget.updated_at = now;

    shared_context_measurement_snapshot_from_uses(&budget.shared_context_uses)
}

pub(super) fn shared_context_measurement_snapshot_from_turn(
    turn: Option<&DeepReviewTurnBudget>,
) -> DeepReviewSharedContextMeasurementSnapshot {
    use super::super::shared_context::shared_context_measurement_snapshot_from_uses;
    turn.map(|t| shared_context_measurement_snapshot_from_uses(&t.shared_context_uses))
        .unwrap_or_default()
}
