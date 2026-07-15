use super::super::execution_policy::{DeepReviewExecutionPolicy, DeepReviewPolicyViolation, DeepReviewSubagentRole};
use super::budget_state::DeepReviewBudgetTracker;
use super::budget_types::{DeepReviewTurnBudget, PRUNE_INTERVAL};
use std::time::Instant;

pub(super) fn normalize_budget_subagent_type(subagent_type: &str) -> Result<String, DeepReviewPolicyViolation> {
    let normalized = subagent_type.trim();
    if normalized.is_empty() {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_subagent_type_missing",
            "DeepReview task budget requires a non-empty subagent type",
        ));
    }

    Ok(normalized.to_string())
}

pub(super) fn record_task_impl(
    tracker: &DeepReviewBudgetTracker,
    parent_dialog_turn_id: &str,
    policy: &DeepReviewExecutionPolicy,
    role: DeepReviewSubagentRole,
    subagent_type: &str,
    is_retry: bool,
) -> Result<(), DeepReviewPolicyViolation> {
    let now = Instant::now();
    if let Ok(last_pruned) = tracker.last_pruned_at.lock() {
        if now.saturating_duration_since(*last_pruned) >= PRUNE_INTERVAL {
            drop(last_pruned);
            tracker.prune_stale(now);
        }
    }

    let mut budget = tracker
        .turns
        .entry(parent_dialog_turn_id.to_string())
        .or_insert_with(|| DeepReviewTurnBudget::new(now));

    match role {
        DeepReviewSubagentRole::Reviewer => {
            let subagent_type = normalize_budget_subagent_type(subagent_type)?;
            if is_retry {
                if policy.max_retries_per_role == 0 {
                    return Err(DeepReviewPolicyViolation::new(
                        "deep_review_retry_budget_exhausted",
                        format!("Retry budget is disabled for DeepReview reviewer '{}'", subagent_type),
                    ));
                }
                if !budget.reviewer_calls_by_subagent.contains_key(subagent_type.as_str()) {
                    return Err(DeepReviewPolicyViolation::new(
                        "deep_review_retry_without_initial_attempt",
                        format!(
                            "Cannot retry DeepReview reviewer '{}' before an initial attempt in this turn",
                            subagent_type
                        ),
                    ));
                }
                let retry_count = budget
                    .retries_used_by_subagent
                    .entry(subagent_type.clone())
                    .or_insert(0);
                if *retry_count >= policy.max_retries_per_role {
                    return Err(DeepReviewPolicyViolation::new(
                        "deep_review_retry_budget_exhausted",
                        format!(
                            "Retry budget exhausted for DeepReview reviewer '{}' (max retries: {})",
                            subagent_type, policy.max_retries_per_role
                        ),
                    ));
                }
                *retry_count += 1;
                budget.updated_at = now;
                return Ok(());
            }

            let max_reviewer_calls = policy.max_same_role_instances
                * (super::super::execution_policy::reviewer_agent_type_count() + policy.extra_subagent_ids.len());
            if budget.reviewer_calls >= max_reviewer_calls {
                return Err(DeepReviewPolicyViolation::new(
                    "deep_review_reviewer_budget_exhausted",
                    format!(
                        "Reviewer launch budget exhausted for this DeepReview turn (max calls: {})",
                        max_reviewer_calls
                    ),
                ));
            }
            budget.reviewer_calls += 1;
            *budget.reviewer_calls_by_subagent.entry(subagent_type).or_insert(0) += 1;
        }
        DeepReviewSubagentRole::Judge => {
            if is_retry {
                return Err(DeepReviewPolicyViolation::new(
                    "deep_review_judge_retry_disallowed",
                    "ReviewJudge retry is not covered by the reviewer retry budget",
                ));
            }
            let max_judge_calls = 1;
            if budget.judge_calls >= max_judge_calls {
                return Err(DeepReviewPolicyViolation::new(
                    "deep_review_judge_budget_exhausted",
                    format!(
                        "ReviewJudge launch budget exhausted for this DeepReview turn (max calls: {})",
                        max_judge_calls
                    ),
                ));
            }

            budget.judge_calls += 1;
        }
    }

    budget.updated_at = now;
    Ok(())
}
