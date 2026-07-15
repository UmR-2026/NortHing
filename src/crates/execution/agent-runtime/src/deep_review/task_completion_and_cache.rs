//! Task completion, retry-guidance presentation, incremental cache attach,
//! packet-id resolution, and launch-batch lookup.
//!
//! This module owns the pure assembly of result data + assistant message
//! pairs that callers feed back to the LLM after each reviewer completes
//! (or is cancelled / capacity-skipped). It also owns the helpers used by
//! the cache attach flow and by retry coverage validation in
//! `super::retry_runtime`.

use super::incremental_cache::DeepReviewIncrementalCache;
use super::types::{DeepReviewIncrementalCacheHit, DeepReviewLaunchBatchInfo};
use super::{DeepReviewConcurrencyPolicy, DeepReviewPolicyViolation, DeepReviewSubagentRole};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepReviewTaskCompletionResultInput<'a> {
    pub delegate_target_label: &'a str,
    pub result_text: &'a str,
    pub context_mode: &'a str,
    pub duration_ms: u128,
    pub is_partial_timeout: bool,
    pub reason: Option<&'a str>,
    pub ledger_event_id: Option<&'a str>,
    pub retry_hint: &'a str,
}

pub fn deep_review_task_completion_result(input: DeepReviewTaskCompletionResultInput<'_>) -> (Value, String) {
    let status = if input.is_partial_timeout {
        "partial_timeout"
    } else {
        "completed"
    };
    let assistant_message = if input.is_partial_timeout {
        format!(
            "{} timed out with partial result:\n<partial_result status=\"partial_timeout\">\n{}\n</partial_result>{}",
            input.delegate_target_label, input.result_text, input.retry_hint
        )
    } else {
        format!(
            "{} completed successfully with result:\n<result>\n{}\n</result>",
            input.delegate_target_label, input.result_text
        )
    };
    let mut data = json!({
        "duration": input.duration_ms,
        "context_mode": input.context_mode,
        "status": status
    });

    if input.is_partial_timeout {
        data["partial_output"] = json!(input.result_text);
        if let Some(reason) = input.reason {
            data["reason"] = json!(reason);
        }
        if let Some(event_id) = input.ledger_event_id {
            data["ledger_event_id"] = json!(event_id);
        }
    }

    (data, assistant_message)
}

pub fn deep_review_cancelled_reviewer_result(subagent_type: &str, reason: &str, duration_ms: u128) -> (Value, String) {
    let duration = u64::try_from(duration_ms).unwrap_or(u64::MAX);
    let reason = if reason.trim().is_empty() {
        "Subagent task was cancelled"
    } else {
        reason.trim()
    };
    let assistant_message = format!(
        "Subagent '{}' was cancelled by the user.\n<result status=\"cancelled\" reason=\"user_cancelled\">Treat this reviewer as cancelled coverage, continue remaining reviewers when useful, and do not relaunch it automatically.</result>",
        subagent_type
    );

    let data = json!({
        "duration": duration,
        "status": "cancelled",
        "reason": reason,
    });

    (data, assistant_message)
}

pub fn should_emit_deep_review_retry_guidance(
    is_partial_timeout: bool,
    is_retry: bool,
    deep_review_subagent_role: Option<DeepReviewSubagentRole>,
) -> bool {
    is_partial_timeout && !is_retry && matches!(deep_review_subagent_role, Some(DeepReviewSubagentRole::Reviewer))
}

pub fn deep_review_retry_guidance(retries_used: usize, max_retries: usize) -> String {
    if max_retries == 0 || retries_used >= max_retries {
        return String::new();
    }

    format!(
        "\n\n<retry_guidance>This reviewer timed out. You may retry with 'retry: true' only if you can provide retry_coverage with source_packet_id, source_status='partial_timeout', covered_files, and a smaller retry_scope_files list. Retries used: {}/{}.</retry_guidance>",
        retries_used, max_retries
    )
}

pub fn auto_retry_suppression_reason(code: &str) -> &'static str {
    match code {
        "deep_review_auto_retry_disabled" => "auto_retry_disabled",
        "deep_review_auto_retry_elapsed_guard_exceeded" => "elapsed_guard_exceeded",
        "deep_review_retry_budget_exhausted" => "budget_exhausted",
        "deep_review_retry_without_initial_attempt" => "without_initial_attempt",
        "deep_review_retry_missing_coverage" => "missing_coverage",
        "deep_review_retry_missing_packet_id" => "missing_coverage",
        "deep_review_retry_missing_status" => "missing_coverage",
        "deep_review_retry_non_retryable_status" => "non_retryable_status",
        "deep_review_retry_unknown_packet" => "unknown_packet",
        "deep_review_retry_missing_packet_scope" => "unknown_packet",
        "deep_review_retry_timeout_required" => "timeout_not_reduced",
        "deep_review_retry_timeout_not_reduced" => "timeout_not_reduced",
        "deep_review_retry_empty_scope" => "empty_scope",
        "deep_review_retry_scope_not_reduced" => "scope_not_reduced",
        _ => "invalid_coverage",
    }
}

pub fn ensure_deep_review_auto_retry_allowed(
    conc_policy: &DeepReviewConcurrencyPolicy,
    elapsed_seconds: Option<u64>,
) -> Result<(), DeepReviewPolicyViolation> {
    if !conc_policy.allow_bounded_auto_retry {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_auto_retry_disabled",
            "DeepReview bounded automatic retry is disabled by Review Team settings",
        ));
    }

    if let Some(elapsed_seconds) = elapsed_seconds {
        if elapsed_seconds > conc_policy.auto_retry_elapsed_guard_seconds {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_auto_retry_elapsed_guard_exceeded",
                format!(
                    "DeepReview automatic retry elapsed guard exceeded (elapsed: {}s, guard: {}s)",
                    elapsed_seconds, conc_policy.auto_retry_elapsed_guard_seconds
                ),
            ));
        }
    }

    Ok(())
}

pub(crate) fn string_for_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

pub(crate) fn value_for_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

pub(crate) fn u64_for_any_key(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| value.get(*key).and_then(Value::as_u64))
}

pub(crate) fn string_array_for_any_key(value: &Value, keys: &[&str]) -> Result<Vec<String>, DeepReviewPolicyViolation> {
    let Some(array) = value_for_any_key(value, keys).and_then(Value::as_array) else {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_missing_coverage",
            format!("Retry coverage requires array field '{}'", keys[0]),
        ));
    };

    let mut result = Vec::with_capacity(array.len());
    for item in array {
        let Some(path) = item.as_str().map(str::trim).filter(|path| !path.is_empty()) else {
            return Err(DeepReviewPolicyViolation::new(
                "deep_review_retry_invalid_coverage",
                format!("Retry coverage field '{}' must contain non-empty strings", keys[0]),
            ));
        };
        result.push(path.to_string());
    }

    Ok(result)
}

fn work_packets_from_manifest(run_manifest: Option<&Value>) -> Option<&Vec<Value>> {
    run_manifest?
        .get("workPackets")
        .or_else(|| run_manifest?.get("work_packets"))?
        .as_array()
}

fn packet_id_from_description(description: Option<&str>) -> Option<String> {
    let description = description?;
    let start = description.find("[packet ")? + "[packet ".len();
    let packet_id = description[start..].split(']').next()?.trim();
    (!packet_id.is_empty()).then(|| packet_id.to_string())
}

fn packet_belongs_to_subagent(packet: &Value, subagent_type: &str) -> bool {
    string_for_any_key(packet, &["subagentId", "subagent_id", "subagentType", "subagent_type"])
        .is_some_and(|value| value == subagent_type)
}

fn packet_id_for_manifest_packet(packet: &Value) -> Option<&str> {
    string_for_any_key(packet, &["packetId", "packet_id"])
}

pub fn deep_review_packet_id_for_cache(
    subagent_type: &str,
    description: Option<&str>,
    run_manifest: Option<&Value>,
) -> Option<String> {
    let packets = work_packets_from_manifest(run_manifest)?;

    if let Some(description_packet_id) = packet_id_from_description(description) {
        return packets
            .iter()
            .any(|packet| {
                packet_id_for_manifest_packet(packet).is_some_and(|packet_id| packet_id == description_packet_id)
                    && packet_belongs_to_subagent(packet, subagent_type)
            })
            .then_some(description_packet_id);
    }

    let mut matches = packets.iter().filter_map(|packet| {
        if packet_belongs_to_subagent(packet, subagent_type) {
            packet_id_for_manifest_packet(packet).map(str::to_string)
        } else {
            None
        }
    });
    let packet_id = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(packet_id)
    }
}

pub fn attach_deep_review_cache(run_manifest: &mut Value, cache_value: Option<Value>) {
    if run_manifest.get("deepReviewCache").is_some() {
        return;
    }
    let Some(cache_value) = cache_value else {
        return;
    };
    if let Some(object) = run_manifest.as_object_mut() {
        object.insert("deepReviewCache".to_string(), cache_value);
    }
}

pub fn deep_review_incremental_cache_hit_for_task(
    subagent_type: &str,
    description: Option<&str>,
    run_manifest: Option<&Value>,
) -> Option<DeepReviewIncrementalCacheHit> {
    let manifest = run_manifest?;
    let cache_value = manifest.get("deepReviewCache")?;
    let cache = DeepReviewIncrementalCache::from_value(cache_value);
    if !cache.matches_manifest(manifest) {
        return None;
    }

    let packet_id = deep_review_packet_id_for_cache(subagent_type, description, Some(manifest))?;
    let cached_output = cache.get_packet(&packet_id)?.to_string();
    Some(DeepReviewIncrementalCacheHit {
        packet_id,
        cached_output,
    })
}

pub fn deep_review_incremental_cache_hit_result(
    subagent_type: &str,
    cache_hit: &DeepReviewIncrementalCacheHit,
) -> (Value, String) {
    (
        json!({ "cached": true, "packet_id": &cache_hit.packet_id }),
        format!(
            "Subagent '{}' result (from incremental review cache):\n<result source=\"cache\">\n{}\n</result>",
            subagent_type, cache_hit.cached_output
        ),
    )
}

pub(crate) fn manifest_packet_by_id<'a>(
    run_manifest: Option<&'a Value>,
    packet_id: &str,
    subagent_type: &str,
) -> Option<&'a Value> {
    work_packets_from_manifest(run_manifest)?.iter().find(|packet| {
        packet_id_for_manifest_packet(packet).is_some_and(|id| id == packet_id)
            && packet_belongs_to_subagent(packet, subagent_type)
    })
}

fn launch_batch_for_manifest_packet(packet: &Value) -> Option<u64> {
    u64_for_any_key(packet, &["launchBatch", "launch_batch"]).filter(|launch_batch| *launch_batch > 0)
}

pub fn deep_review_launch_batch_for_task(
    subagent_type: &str,
    description: Option<&str>,
    run_manifest: Option<&Value>,
) -> Option<DeepReviewLaunchBatchInfo> {
    let packet_id = deep_review_packet_id_for_cache(subagent_type, description, run_manifest)?;
    let packet = manifest_packet_by_id(run_manifest, &packet_id, subagent_type)?;
    let launch_batch = launch_batch_for_manifest_packet(packet)?;

    Some(DeepReviewLaunchBatchInfo {
        packet_id: Some(packet_id),
        launch_batch,
    })
}

pub(crate) fn file_paths_for_manifest_packet(packet: &Value) -> Result<Vec<String>, DeepReviewPolicyViolation> {
    let Some(scope) = value_for_any_key(packet, &["assignedScope", "assigned_scope"]) else {
        return Err(DeepReviewPolicyViolation::new(
            "deep_review_retry_missing_packet_scope",
            "DeepReview retry source packet is missing assigned scope",
        ));
    };
    string_array_for_any_key(scope, &["files"])
}

#[cfg(test)]
mod tests {
    use super::super::incremental_cache::DeepReviewIncrementalCache;
    use super::super::{DeepReviewConcurrencyPolicy, DeepReviewSubagentRole};
    use super::*;
    use serde_json::json;

    #[test]
    fn incremental_cache_hit_prefers_description_packet() {
        let mut cache = DeepReviewIncrementalCache::new("fp-test-123");
        cache.store_packet("reviewer:ReviewSecurity:group-2-of-2", "Found 2 security issues");
        let manifest = json!({
            "incrementalReviewCache": { "fingerprint": "fp-test-123" },
            "deepReviewCache": cache.to_value(),
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-2",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity"
                },
                {
                    "packetId": "reviewer:ReviewSecurity:group-2-of-2",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity"
                }
            ]
        });

        let cache_hit = deep_review_incremental_cache_hit_for_task(
            "ReviewSecurity",
            Some("Security review [packet reviewer:ReviewSecurity:group-2-of-2]"),
            Some(&manifest),
        )
        .expect("description packet should select the matching cache entry");

        assert_eq!(cache_hit.packet_id, "reviewer:ReviewSecurity:group-2-of-2");
        assert_eq!(cache_hit.cached_output, "Found 2 security issues");
        let (data, assistant_message) = deep_review_incremental_cache_hit_result("ReviewSecurity", &cache_hit);
        assert_eq!(
            data,
            json!({ "cached": true, "packet_id": "reviewer:ReviewSecurity:group-2-of-2" })
        );
        assert_eq!(
            assistant_message,
            "Subagent 'ReviewSecurity' result (from incremental review cache):\n<result source=\"cache\">\nFound 2 security issues\n</result>"
        );
    }

    #[test]
    fn incremental_cache_hit_uses_unique_manifest_packet_without_description() {
        let mut cache = DeepReviewIncrementalCache::new("fp-test-123");
        cache.store_packet("reviewer:ReviewBusinessLogic", "Logic finding");
        let manifest = json!({
            "incrementalReviewCache": { "fingerprint": "fp-test-123" },
            "deepReviewCache": cache.to_value(),
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewBusinessLogic",
                    "phase": "reviewer",
                    "subagentId": "ReviewBusinessLogic"
                }
            ]
        });

        let cache_hit =
            deep_review_incremental_cache_hit_for_task("ReviewBusinessLogic", Some("Logic review"), Some(&manifest))
                .expect("unique packet should be selected without a packet marker");

        assert_eq!(cache_hit.packet_id, "reviewer:ReviewBusinessLogic");
        assert_eq!(cache_hit.cached_output, "Logic finding");
    }

    #[test]
    fn incremental_cache_hit_skips_mismatches_and_ambiguous_packets() {
        let mut cache = DeepReviewIncrementalCache::new("fp-old");
        cache.store_packet("reviewer:ReviewPerformance:group-1-of-2", "Perf finding");
        let fingerprint_mismatch_manifest = json!({
            "incrementalReviewCache": { "fingerprint": "fp-new" },
            "deepReviewCache": cache.to_value(),
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewPerformance:group-1-of-2",
                    "phase": "reviewer",
                    "subagentId": "ReviewPerformance"
                }
            ]
        });
        assert_eq!(
            deep_review_incremental_cache_hit_for_task(
                "ReviewPerformance",
                Some("Performance review"),
                Some(&fingerprint_mismatch_manifest),
            ),
            None
        );

        let mut cache = DeepReviewIncrementalCache::new("fp-test-123");
        cache.store_packet("reviewer:ReviewPerformance:group-1-of-2", "Perf finding");
        let split_packet_manifest = json!({
            "incrementalReviewCache": { "fingerprint": "fp-test-123" },
            "deepReviewCache": cache.to_value(),
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewPerformance:group-1-of-2",
                    "phase": "reviewer",
                    "subagentId": "ReviewPerformance"
                },
                {
                    "packetId": "reviewer:ReviewPerformance:group-2-of-2",
                    "phase": "reviewer",
                    "subagentId": "ReviewPerformance"
                }
            ]
        });
        assert_eq!(
            deep_review_incremental_cache_hit_for_task(
                "ReviewPerformance",
                Some("Performance review"),
                Some(&split_packet_manifest),
            ),
            None
        );
        assert_eq!(
            deep_review_incremental_cache_hit_for_task(
                "ReviewPerformance",
                Some("Performance review [packet reviewer:ReviewSecurity:group-1-of-1]"),
                Some(&split_packet_manifest),
            ),
            None
        );
    }

    #[test]
    fn task_completion_result_preserves_completed_message_and_data_shape() {
        let (data, assistant_message) = deep_review_task_completion_result(DeepReviewTaskCompletionResultInput {
            delegate_target_label: "ReviewSecurity",
            result_text: "No issues found",
            context_mode: "fresh",
            duration_ms: 42,
            is_partial_timeout: false,
            reason: None,
            ledger_event_id: None,
            retry_hint: "",
        });

        assert_eq!(data["duration"], json!(42));
        assert_eq!(data["context_mode"], "fresh");
        assert_eq!(data["status"], "completed");
        assert!(data.get("partial_output").is_none());
        assert_eq!(
            assistant_message,
            "ReviewSecurity completed successfully with result:\n<result>\nNo issues found\n</result>"
        );
    }

    #[test]
    fn task_completion_result_preserves_partial_timeout_payload() {
        let (data, assistant_message) = deep_review_task_completion_result(DeepReviewTaskCompletionResultInput {
            delegate_target_label: "ReviewPerformance",
            result_text: "Partial findings",
            context_mode: "reuse",
            duration_ms: 120,
            is_partial_timeout: true,
            reason: Some("timeout"),
            ledger_event_id: Some("event-1"),
            retry_hint: "\n\n<retry_guidance>retry</retry_guidance>",
        });

        assert_eq!(data["status"], "partial_timeout");
        assert_eq!(data["partial_output"], "Partial findings");
        assert_eq!(data["reason"], "timeout");
        assert_eq!(data["ledger_event_id"], "event-1");
        assert_eq!(
            assistant_message,
            "ReviewPerformance timed out with partial result:\n<partial_result status=\"partial_timeout\">\nPartial findings\n</partial_result>\n\n<retry_guidance>retry</retry_guidance>"
        );
    }

    #[test]
    fn cancelled_reviewer_result_preserves_parent_guidance_and_data_shape() {
        let (data, assistant_message) =
            deep_review_cancelled_reviewer_result("ReviewArchitecture", " Subagent task has been cancelled ", 42);

        assert_eq!(data["status"], "cancelled");
        assert_eq!(data["reason"], "Subagent task has been cancelled");
        assert_eq!(data["duration"], 42);
        assert!(assistant_message.contains("status=\"cancelled\""));
        assert!(assistant_message.contains("reason=\"user_cancelled\""));
        assert!(assistant_message.contains("do not relaunch it automatically"));
    }

    #[test]
    fn cancelled_reviewer_result_defaults_empty_reason_and_caps_duration() {
        let (data, _assistant_message) = deep_review_cancelled_reviewer_result("ReviewSecurity", "  ", u128::MAX);

        assert_eq!(data["status"], "cancelled");
        assert_eq!(data["reason"], "Subagent task was cancelled");
        assert_eq!(data["duration"], u64::MAX);
    }

    #[test]
    fn retry_guidance_policy_applies_only_to_initial_reviewer_timeout() {
        assert!(should_emit_deep_review_retry_guidance(
            true,
            false,
            Some(DeepReviewSubagentRole::Reviewer),
        ));
        assert!(!should_emit_deep_review_retry_guidance(
            false,
            false,
            Some(DeepReviewSubagentRole::Reviewer),
        ));
        assert!(!should_emit_deep_review_retry_guidance(
            true,
            true,
            Some(DeepReviewSubagentRole::Reviewer),
        ));
        assert!(!should_emit_deep_review_retry_guidance(
            true,
            false,
            Some(DeepReviewSubagentRole::Judge),
        ));
    }

    #[test]
    fn retry_guidance_message_preserves_budget_text() {
        assert_eq!(
            deep_review_retry_guidance(1, 3),
            "\n\n<retry_guidance>This reviewer timed out. You may retry with 'retry: true' only if you can provide retry_coverage with source_packet_id, source_status='partial_timeout', covered_files, and a smaller retry_scope_files list. Retries used: 1/3.</retry_guidance>"
        );
        assert!(deep_review_retry_guidance(3, 3).is_empty());
        assert!(deep_review_retry_guidance(0, 0).is_empty());
    }

    #[test]
    fn auto_retry_admission_uses_opt_in_and_elapsed_guard() {
        let mut policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 1,
            stagger_seconds: 0,
            max_queue_wait_seconds: 1,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };

        let disabled =
            ensure_deep_review_auto_retry_allowed(&policy, None).expect_err("disabled auto retry should be rejected");
        assert_eq!(disabled.code, "deep_review_auto_retry_disabled");

        policy.allow_bounded_auto_retry = true;
        assert!(ensure_deep_review_auto_retry_allowed(&policy, Some(180)).is_ok());
        let elapsed = ensure_deep_review_auto_retry_allowed(&policy, Some(181))
            .expect_err("elapsed guard should reject late auto retry");
        assert_eq!(elapsed.code, "deep_review_auto_retry_elapsed_guard_exceeded");
    }

    #[test]
    fn auto_retry_suppression_reason_stays_stable() {
        assert_eq!(
            auto_retry_suppression_reason("deep_review_retry_missing_packet_scope"),
            "unknown_packet"
        );
        assert_eq!(
            auto_retry_suppression_reason("deep_review_retry_timeout_not_reduced"),
            "timeout_not_reduced"
        );
        assert_eq!(auto_retry_suppression_reason("unexpected"), "invalid_coverage");
    }
}
