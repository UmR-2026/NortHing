//! Task tool — DeepReview runtime tests (Round 12b split: async tokio + retry + provider tests)
//!
//! Owns 16 async tokio tests + 4 retry tests + 1 quota test + 2 final tests.
//! Sync + reviewer-queue async tests live in sibling
//! `task_tool_deep_review_tests`.
//!
//! Production code lives in sibling `task_tool_deep_review_policy`.
//!
//! Spec: `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md` (e4261ff)

#[cfg(test)]
mod tests {
    use super::super::task_tool_deep_review_policy::{
        auto_retry_suppression_reason, deep_review_capacity_decision_for_provider_error,
        deep_review_capacity_skip_result_for_provider_queue_outcome, ensure_deep_review_auto_retry_allowed,
        ensure_deep_review_retry_coverage, prompt_with_deep_review_retry_scope,
    };
    use crate::agentic::deep_review::task_adapter::DeepReviewProviderQueueWaitOutcome;
    use crate::agentic::deep_review_policy::{
        apply_deep_review_queue_control, deep_review_effective_concurrency_snapshot,
        deep_review_runtime_diagnostics_snapshot, record_deep_review_effective_concurrency_success,
        try_begin_deep_review_active_reviewer, DeepReviewConcurrencyPolicy, DeepReviewQueueControlAction,
    };
    use crate::util::NortHingError;
    use serde_json::json;

    #[test]
    fn deep_review_auto_retry_requires_review_team_opt_in() {
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 4,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };

        let violation = ensure_deep_review_auto_retry_allowed(&policy, "turn-auto-retry-disabled")
            .expect_err("auto retry must be disabled by default");

        assert_eq!(violation.code, "deep_review_auto_retry_disabled");
        assert_eq!(auto_retry_suppression_reason(violation.code), "auto_retry_disabled");
    }

    #[test]
    fn deep_review_auto_retry_opt_in_allows_guarded_admission() {
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 4,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: true,
            auto_retry_elapsed_guard_seconds: 180,
        };

        ensure_deep_review_auto_retry_allowed(&policy, "turn-auto-retry-enabled")
            .expect("opted-in auto retry should pass the admission gate before budget checks");
    }

    #[test]
    fn deep_review_retry_rejects_missing_structured_coverage() {
        let manifest = json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "timeoutSeconds": 600,
                    "assignedScope": {
                        "files": [
                            "src/crates/assembly/core/src/auth.rs",
                            "src/crates/assembly/core/src/token.rs"
                        ]
                    }
                }
            ]
        });
        let input = json!({ "retry": true });

        let violation = ensure_deep_review_retry_coverage(&input, "ReviewSecurity", Some(&manifest))
            .expect_err("missing retry coverage should be rejected");

        assert_eq!(violation.code, "deep_review_retry_missing_coverage");
    }

    #[test]
    fn deep_review_retry_rejects_broad_scope() {
        let manifest = json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "timeoutSeconds": 600,
                    "assignedScope": {
                        "files": [
                            "src/crates/assembly/core/src/auth.rs",
                            "src/crates/assembly/core/src/token.rs"
                        ]
                    }
                }
            ]
        });
        let input = json!({
            "retry": true,
            "timeout_seconds": 300,
            "retry_coverage": {
                "source_packet_id": "reviewer:ReviewSecurity:group-1-of-1",
                "source_status": "partial_timeout",
                "covered_files": [
                    "src/crates/assembly/core/src/auth.rs"
                ],
                "retry_scope_files": [
                    "src/crates/assembly/core/src/auth.rs",
                    "src/crates/assembly/core/src/token.rs"
                ]
            }
        });

        let violation = ensure_deep_review_retry_coverage(&input, "ReviewSecurity", Some(&manifest))
            .expect_err("retrying the full packet should be rejected");

        assert_eq!(violation.code, "deep_review_retry_scope_not_reduced");
    }

    #[test]
    fn deep_review_retry_rejects_timeout_that_is_not_lowered() {
        let manifest = json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "timeoutSeconds": 600,
                    "assignedScope": {
                        "files": [
                            "src/crates/assembly/core/src/auth.rs",
                            "src/crates/assembly/core/src/token.rs"
                        ]
                    }
                }
            ]
        });
        let input = json!({
            "retry": true,
            "timeout_seconds": 600,
            "retry_coverage": {
                "source_packet_id": "reviewer:ReviewSecurity:group-1-of-1",
                "source_status": "partial_timeout",
                "covered_files": [
                    "src/crates/assembly/core/src/auth.rs"
                ],
                "retry_scope_files": [
                    "src/crates/assembly/core/src/token.rs"
                ]
            }
        });

        let violation = ensure_deep_review_retry_coverage(&input, "ReviewSecurity", Some(&manifest))
            .expect_err("retry timeout must be lower than source timeout");

        assert_eq!(violation.code, "deep_review_retry_timeout_not_reduced");
    }

    #[test]
    fn deep_review_retry_rejects_non_queueable_capacity_reason() {
        let manifest = json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "timeoutSeconds": 600,
                    "assignedScope": {
                        "files": [
                            "src/crates/assembly/core/src/auth.rs",
                            "src/crates/assembly/core/src/token.rs"
                        ]
                    }
                }
            ]
        });
        let input = json!({
            "retry": true,
            "retry_coverage": {
                "source_packet_id": "reviewer:ReviewSecurity:group-1-of-1",
                "source_status": "capacity_skipped",
                "capacity_reason": "auth_error",
                "covered_files": [],
                "retry_scope_files": [
                    "src/crates/assembly/core/src/token.rs"
                ]
            }
        });

        let violation = ensure_deep_review_retry_coverage(&input, "ReviewSecurity", Some(&manifest))
            .expect_err("non-queueable capacity failures must fail fast");

        assert_eq!(violation.code, "deep_review_retry_non_retryable_status");
    }

    #[test]
    fn deep_review_provider_capacity_error_builds_capacity_skipped_payload_and_lowers_effective_cap() {
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 3,
            stagger_seconds: 0,
            max_queue_wait_seconds: 30,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id = "turn-provider-capacity-skip";
        let decision = deep_review_capacity_decision_for_provider_error(&NortHingError::ai(
            "Provider error: provider=openai, code=429, message=rate limit exceeded",
        ));
        assert!(decision.queueable);
        let reason = decision
            .reason
            .expect("provider rate limit should surface as capacity_skipped");
        let (data, assistant_message) = deep_review_capacity_skip_result_for_provider_queue_outcome(
            reason,
            turn_id,
            "ReviewSecurity",
            &policy,
            42,
            0,
            None,
        );

        assert_eq!(data["status"], "capacity_skipped");
        assert_eq!(data["queue_skip_reason"], "provider_rate_limit");
        assert_eq!(data["effective_parallel_instances"], 2);
        assert!(assistant_message.contains("status=\"capacity_skipped\""));
        assert!(assistant_message.contains("reason=\"provider_rate_limit\""));
        assert_eq!(
            deep_review_effective_concurrency_snapshot(turn_id, 3).effective_parallel_instances,
            2
        );
    }

    #[test]
    fn deep_review_provider_quota_error_is_not_capacity_skipped() {
        let decision = deep_review_capacity_decision_for_provider_error(&NortHingError::ai(
            "Provider error: provider=glm, code=1113, message=insufficient quota",
        ));

        assert!(
            !decision.queueable,
            "quota errors should remain fail-fast instead of entering capacity queue flow"
        );
    }

    #[tokio::test]
    async fn deep_review_provider_capacity_queue_retries_when_active_reviewer_frees_capacity() {
        let turn_id = "turn-provider-queue-active-release";
        let tool_id = "tool-provider-queue-active-release";
        let occupied = try_begin_deep_review_active_reviewer(turn_id, 2)
            .expect("precondition should occupy another reviewer slot");
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            super::super::task_tool_deep_review_policy::wait_for_deep_review_provider_capacity_retry(
                "session-provider-queue-active-release",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                crate::agentic::deep_review_policy::DeepReviewCapacityQueueReason::ProviderConcurrencyLimit,
                60,
                false,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "provider queue should keep waiting while no additional reviewer capacity freed"
        );
        drop(occupied);

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle)
            .await
            .expect("provider queue should wake when another active reviewer frees capacity")
            .expect("spawned wait should not panic");

        match outcome {
            DeepReviewProviderQueueWaitOutcome::ReadyToRetry {
                queue_elapsed_ms,
                early_capacity_probe,
            } => {
                assert!(
                    queue_elapsed_ms < 500,
                    "early capacity wake should not wait for the full backoff window"
                );
                assert!(
                    early_capacity_probe,
                    "active reviewer release should be marked as an early provider capacity probe"
                );
            }
            DeepReviewProviderQueueWaitOutcome::Skipped { .. } => {
                panic!("provider queue should retry after active reviewer capacity frees")
            }
        }
    }

    #[tokio::test]
    async fn deep_review_provider_retry_after_wait_ignores_active_reviewer_release() {
        let turn_id = "turn-provider-retry-after-hard-wait";
        let tool_id = "tool-provider-retry-after-hard-wait";
        let occupied = try_begin_deep_review_active_reviewer(turn_id, 2)
            .expect("precondition should occupy another reviewer slot");
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 1,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            super::super::task_tool_deep_review_policy::wait_for_deep_review_provider_capacity_retry(
                "session-provider-retry-after-hard-wait",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                crate::agentic::deep_review_policy::DeepReviewCapacityQueueReason::RetryAfter,
                1,
                false,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        drop(occupied);
        tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
        assert!(
            !handle.is_finished(),
            "retry-after waits should not be interrupted by local reviewer capacity release"
        );

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(1500), handle)
            .await
            .expect("retry-after wait should eventually finish")
            .expect("spawned wait should not panic");

        match outcome {
            DeepReviewProviderQueueWaitOutcome::ReadyToRetry {
                early_capacity_probe, ..
            } => {
                assert!(
                    !early_capacity_probe,
                    "retry-after completion should be a natural cooldown retry"
                );
            }
            DeepReviewProviderQueueWaitOutcome::Skipped { .. } => {
                panic!("retry-after wait should retry after its bounded cooldown")
            }
        }
    }

    #[tokio::test]
    async fn deep_review_provider_capacity_queue_cancel_control_skips_retry() {
        let turn_id = "turn-provider-queue-cancel";
        let tool_id = "tool-provider-queue-cancel";
        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Cancel);
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };

        let outcome = super::super::task_tool_deep_review_policy::wait_for_deep_review_provider_capacity_retry(
            "session-provider-queue-cancel",
            turn_id,
            tool_id,
            "ReviewSecurity",
            &policy,
            crate::agentic::deep_review_policy::DeepReviewCapacityQueueReason::ProviderRateLimit,
            60,
            false,
        )
        .await;

        match outcome {
            DeepReviewProviderQueueWaitOutcome::Skipped {
                queue_elapsed_ms,
                skip_reason,
            } => {
                assert!(queue_elapsed_ms < 100);
                assert_eq!(
                    skip_reason,
                    crate::agentic::deep_review::task_adapter::DeepReviewQueueWaitSkipReason::UserCancelled
                );
            }
            DeepReviewProviderQueueWaitOutcome::ReadyToRetry { .. } => {
                panic!("cancelled provider queue should not retry")
            }
        }

        let diagnostics =
            deep_review_runtime_diagnostics_snapshot(turn_id).expect("provider queue should record diagnostics");
        assert_eq!(diagnostics.provider_capacity_queue_count, 1);
        assert_eq!(
            diagnostics
                .provider_capacity_queue_reason_counts
                .get("provider_rate_limit"),
            Some(&1)
        );
    }

    #[tokio::test]
    async fn deep_review_provider_capacity_queue_pause_does_not_count_against_wait() {
        let turn_id = "turn-provider-queue-pause";
        let tool_id = "tool-provider-queue-pause";
        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Pause);
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 1,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            super::super::task_tool_deep_review_policy::wait_for_deep_review_provider_capacity_retry(
                "session-provider-queue-pause",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                crate::agentic::deep_review_policy::DeepReviewCapacityQueueReason::ProviderConcurrencyLimit,
                1,
                false,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "paused provider queue should not expire before continue"
        );

        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Continue);
        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(1500), handle)
            .await
            .expect("continued provider queue should finish")
            .expect("spawned wait should not panic");

        match outcome {
            DeepReviewProviderQueueWaitOutcome::ReadyToRetry { queue_elapsed_ms, .. } => {
                assert!(queue_elapsed_ms >= 900);
            }
            DeepReviewProviderQueueWaitOutcome::Skipped { .. } => {
                panic!("continued provider queue should retry after bounded wait")
            }
        }
    }

    #[test]
    fn deep_review_retry_accepts_reduced_partial_timeout_scope() {
        let manifest = json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "timeoutSeconds": 600,
                    "assignedScope": {
                        "files": [
                            "src/crates/assembly/core/src/auth.rs",
                            "src/crates/assembly/core/src/token.rs"
                        ]
                    }
                }
            ]
        });
        let input = json!({
            "retry": true,
            "timeout_seconds": 300,
            "retry_coverage": {
                "source_packet_id": "reviewer:ReviewSecurity:group-1-of-1",
                "source_status": "partial_timeout",
                "covered_files": [
                    "src/crates/assembly/core/src/auth.rs"
                ],
                "retry_scope_files": [
                    "src/crates/assembly/core/src/token.rs"
                ]
            }
        });

        let retry_scope = ensure_deep_review_retry_coverage(&input, "ReviewSecurity", Some(&manifest))
            .expect("reduced retry scope should be accepted");

        assert_eq!(retry_scope, vec!["src/crates/assembly/core/src/token.rs"]);
    }

    #[test]
    fn deep_review_retry_scope_prompt_prepend_bounds_review_files() {
        let prompt = prompt_with_deep_review_retry_scope(
            "Continue the security review.",
            &["src/crates/assembly/core/src/token.rs".to_string()],
        );

        assert!(prompt.starts_with("<deep_review_retry_scope>"));
        assert!(prompt.contains("Review only the following retry_scope_files"));
        assert!(prompt.contains("- src/crates/assembly/core/src/token.rs"));
        assert!(prompt.ends_with("Continue the security review."));
    }

    // Marker to silence unused-import for `record_deep_review_effective_concurrency_success`
    #[allow(dead_code)]
    fn _unused_marker() {
        let _ = record_deep_review_effective_concurrency_success;
    }
}
