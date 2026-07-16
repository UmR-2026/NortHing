//! Deep Review score/signal enrichment.
//!
//! Owns reliability signal shaping for the score/rating section of a Deep
//! Review report.

use super::report_summary::DeepReviewCacheUpdate;
use crate::deep_review::manifest::{DeepReviewEvidencePack, DeepReviewScopeProfile};
use serde_json::{json, Value};
use std::collections::HashSet;

pub fn push_reliability_signal_if_missing(input: &mut Value, signal: Value) {
    let Some(kind) = signal.get("kind").and_then(Value::as_str) else {
        return;
    };
    if has_reliability_signal(input, kind) {
        return;
    }
    if !input.get("reliability_signals").is_some_and(Value::is_array) {
        input["reliability_signals"] = json!([]);
    }
    if let Some(signals) = input.get_mut("reliability_signals").and_then(Value::as_array_mut) {
        signals.push(signal);
    }
}

pub fn fill_deep_review_runtime_tracker_signal(input: &mut Value, concurrency_limited_count: usize) {
    if concurrency_limited_count == 0 {
        return;
    }

    push_reliability_signal_if_missing(
        input,
        json!({
            "kind": "concurrency_limited",
            "severity": "warning",
            "count": concurrency_limited_count,
            "source": "runtime"
        }),
    );
}

pub fn fill_deep_review_cache_update_signals(input: &mut Value, cache_update: &DeepReviewCacheUpdate) {
    if cache_update.hit_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "cache_hit",
                "severity": "info",
                "count": cache_update.hit_count,
                "source": "runtime"
            }),
        );
    }
    if cache_update.miss_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "cache_miss",
                "severity": "info",
                "count": cache_update.miss_count,
                "source": "runtime"
            }),
        );
    }
}

pub fn fill_deep_review_reliability_signals(
    input: &mut Value,
    run_manifest: Option<&Value>,
    compression_preserved_signal_count: Option<usize>,
) {
    if let Some(scope_profile) = run_manifest.and_then(DeepReviewScopeProfile::from_manifest) {
        if scope_profile.is_reduced_depth() {
            let mut signal = json!({
                "kind": "reduced_scope",
                "severity": "info",
                "source": "manifest"
            });
            if let Some(detail) = scope_profile.coverage_expectation() {
                signal["detail"] = json!(detail);
            }
            push_reliability_signal_if_missing(input, signal);
        }
    }

    if let Some(manifest) = run_manifest {
        if let Err(error) = DeepReviewEvidencePack::from_manifest(manifest) {
            push_reliability_signal_if_missing(
                input,
                json!({
                    "kind": "context_pressure",
                    "severity": "warning",
                    "source": "manifest",
                    "detail": format!("Evidence pack ignored: {}", error)
                }),
            );
        }
    }

    if let Some(token_budget) =
        run_manifest.and_then(|manifest| value_for_any_key(manifest, &["tokenBudget", "token_budget"]))
    {
        let has_context_pressure =
            bool_for_any_key(token_budget, &["largeDiffSummaryFirst", "large_diff_summary_first"])
                || has_non_empty_array_for_any_key(token_budget, &["warnings"]);
        if has_context_pressure {
            let count =
                u64_for_any_key(token_budget, &["estimatedReviewerCalls", "estimated_reviewer_calls"]).unwrap_or(0);
            push_reliability_signal_if_missing(
                input,
                json!({
                    "kind": "context_pressure",
                    "severity": "info",
                    "count": count,
                    "source": "runtime"
                }),
            );
        }
    }

    let skipped_reviewer_count = count_manifest_skipped_reviewers(run_manifest);
    if skipped_reviewer_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "skipped_reviewers",
                "severity": "info",
                "count": skipped_reviewer_count,
                "source": "manifest"
            }),
        );
    }

    let token_budget_limited_reviewer_count = count_token_budget_limited_reviewers(run_manifest);
    if token_budget_limited_reviewer_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "token_budget_limited",
                "severity": "warning",
                "count": token_budget_limited_reviewer_count,
                "source": "manifest"
            }),
        );
    }

    if let Some(count) = compression_preserved_signal_count.filter(|count| *count > 0) {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "compression_preserved",
                "severity": "info",
                "count": count,
                "source": "runtime"
            }),
        );
    }

    let partial_reviewer_count = count_partial_reviewers(input);
    if partial_reviewer_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "partial_reviewer",
                "severity": "warning",
                "count": partial_reviewer_count,
                "source": "runtime"
            }),
        );
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "retry_guidance",
                "severity": "warning",
                "count": partial_reviewer_count,
                "source": "runtime"
            }),
        );
    }

    let decision_item_count = count_decision_items(input);
    if decision_item_count > 0 {
        push_reliability_signal_if_missing(
            input,
            json!({
                "kind": "user_decision",
                "severity": "action",
                "count": decision_item_count,
                "source": "report"
            }),
        );
    }
}

fn value_for_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn bool_for_any_key(value: &Value, keys: &[&str]) -> bool {
    value_for_any_key(value, keys).and_then(Value::as_bool).unwrap_or(false)
}

fn u64_for_any_key(value: &Value, keys: &[&str]) -> Option<u64> {
    value_for_any_key(value, keys).and_then(Value::as_u64)
}

fn has_non_empty_array_for_any_key(value: &Value, keys: &[&str]) -> bool {
    value_for_any_key(value, keys)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn count_partial_reviewers(input: &Value) -> usize {
    input
        .get("reviewers")
        .and_then(Value::as_array)
        .map(|reviewers| {
            reviewers
                .iter()
                .filter(|reviewer| {
                    let status = reviewer
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    let has_partial_output = reviewer
                        .get("partial_output")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .is_some_and(|output| !output.is_empty());
                    status == "partial_timeout"
                        || (matches!(status, "timed_out" | "cancelled_by_user") && has_partial_output)
                })
                .count()
        })
        .unwrap_or(0)
}

fn count_manifest_skipped_reviewers(run_manifest: Option<&Value>) -> usize {
    run_manifest
        .and_then(|manifest| value_for_any_key(manifest, &["skippedReviewers", "skipped_reviewers"]))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn count_token_budget_limited_reviewers(run_manifest: Option<&Value>) -> usize {
    let Some(manifest) = run_manifest else {
        return 0;
    };
    let mut skipped_by_budget = HashSet::new();

    if let Some(skipped_ids) = value_for_any_key(manifest, &["tokenBudget", "token_budget"])
        .and_then(|token_budget| value_for_any_key(token_budget, &["skippedReviewerIds", "skipped_reviewer_ids"]))
        .and_then(Value::as_array)
    {
        for value in skipped_ids {
            if let Some(id) = value.as_str().map(str::trim).filter(|id| !id.is_empty()) {
                skipped_by_budget.insert(id.to_string());
            }
        }
    }

    if let Some(skipped_reviewers) =
        value_for_any_key(manifest, &["skippedReviewers", "skipped_reviewers"]).and_then(Value::as_array)
    {
        for reviewer in skipped_reviewers {
            let reason = packet_string_field(reviewer, &["reason"]);
            if reason != Some("budget_limited") {
                continue;
            }
            if let Some(id) = packet_string_field(reviewer, &["subagentId", "subagent_id"]) {
                skipped_by_budget.insert(id.to_string());
            }
        }
    }

    skipped_by_budget.len()
}

fn count_decision_items(input: &Value) -> usize {
    let needs_decision_count = input
        .pointer("/report_sections/remediation_groups/needs_decision")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .count()
        })
        .unwrap_or(0);
    if needs_decision_count > 0 {
        return needs_decision_count;
    }

    let recommended_action = input
        .pointer("/summary/recommended_action")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    usize::from(recommended_action == "block")
}

fn has_reliability_signal(input: &Value, kind: &str) -> bool {
    input
        .get("reliability_signals")
        .and_then(Value::as_array)
        .is_some_and(|signals| {
            signals.iter().any(|signal| {
                signal
                    .get("kind")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == kind)
            })
        })
}

fn packet_string_field<'a>(packet: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| packet.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
