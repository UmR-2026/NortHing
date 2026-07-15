//! Task tool — DeepReview tests (Round 12b split: sync tests + reviewer-queue async tests)
//!
//! Owns 14 test fns (7 sync + 6 reviewer-queue tokio + 1 concurrency_judge).
//! The remaining 16 async tokio tests live in sibling
//! `task_tool_deep_review_tests_runtime`.
//!
//! Production code lives in sibling `task_tool_deep_review_policy`.
//!
//! Spec: `docs/handoffs/2026-06-29-round12b-task-tool-deep-review-secondary-split-spec.md` (e4261ff)

#[cfg(test)]
mod tests {
    use super::super::task_tool_deep_review_policy::{
        deep_review_cancelled_reviewer_tool_result, deep_review_capacity_decision_for_provider_error,
        deep_review_capacity_skip_result_for_provider_queue_outcome, deep_review_retry_guidance_max_retries,
        ensure_deep_review_auto_retry_allowed, ensure_deep_review_retry_coverage, prompt_with_deep_review_retry_scope,
        should_emit_deep_review_retry_guidance, wait_for_deep_review_reviewer_admission,
    };
    use crate::agentic::deep_review::task_adapter::{
        self as deep_review_task_adapter, DeepReviewLaunchBatchInfo, DeepReviewQueueWaitOutcome,
    };
    use crate::agentic::deep_review_policy::{
        apply_deep_review_queue_control, deep_review_capacity_skip_count, deep_review_concurrency_cap_rejection_count,
        deep_review_effective_parallel_instances, deep_review_max_retries_per_role, deep_review_retries_used,
        DeepReviewBudgetTracker, DeepReviewConcurrencyPolicy, DeepReviewExecutionPolicy, DeepReviewQueueControlAction,
        DeepReviewSubagentRole,
    };
    use crate::agentic::tools::framework::ToolResult;
    use serde_json::json;

    #[test]
    fn deep_review_policy_allows_only_configured_team_members() {
        let policy = DeepReviewExecutionPolicy::from_config_value(Some(&json!({
            "extra_subagent_ids": [
                "ExtraReviewer",
                "DeepReview",
                "ReviewFixer",
                "ReviewJudge",
                "ReviewBusinessLogic"
            ]
        })));

        assert_eq!(
            policy.classify_subagent("ReviewBusinessLogic").unwrap(),
            DeepReviewSubagentRole::Reviewer
        );
        assert_eq!(
            policy.classify_subagent("ExtraReviewer").unwrap(),
            DeepReviewSubagentRole::Reviewer
        );
        assert_eq!(
            policy.classify_subagent("ReviewJudge").unwrap(),
            DeepReviewSubagentRole::Judge
        );
        assert!(policy.classify_subagent("ReviewFixer").is_err());
        assert!(policy.classify_subagent("CodeReview").is_err());
        assert!(policy.classify_subagent("DeepReview").is_err());
    }

    #[test]
    fn deep_review_policy_caps_reviewer_and_judge_timeouts() {
        let policy = DeepReviewExecutionPolicy::from_config_value(Some(&json!({
            "reviewer_timeout_seconds": 300,
            "judge_timeout_seconds": 240
        })));

        assert_eq!(
            policy.effective_timeout_seconds(DeepReviewSubagentRole::Reviewer, Some(900)),
            Some(300)
        );
        assert_eq!(
            policy.effective_timeout_seconds(DeepReviewSubagentRole::Reviewer, None),
            Some(300)
        );
        assert_eq!(
            policy.effective_timeout_seconds(DeepReviewSubagentRole::Judge, Some(900)),
            Some(240)
        );
    }

    #[test]
    fn deep_review_cancelled_reviewer_result_tells_parent_not_to_relaunch() {
        let result =
            deep_review_cancelled_reviewer_tool_result("ReviewArchitecture", "Subagent task has been cancelled", 42);

        let ToolResult::Result {
            data,
            result_for_assistant,
            image_attachments,
        } = result
        else {
            panic!("cancelled reviewer should return a structured tool result");
        };

        assert_eq!(data["status"], "cancelled");
        assert_eq!(data["reason"], "Subagent task has been cancelled");
        assert_eq!(data["duration"], 42);
        assert!(image_attachments.is_none());

        let assistant_message = result_for_assistant.expect("assistant message should be present");
        assert!(assistant_message.contains("status=\"cancelled\""));
        assert!(assistant_message.contains("do not relaunch it automatically"));
    }

    #[test]
    fn deep_review_policy_saturates_oversized_numeric_limits() {
        let policy = DeepReviewExecutionPolicy::from_config_value(Some(&json!({
            "reviewer_timeout_seconds": u64::MAX,
            "judge_timeout_seconds": u64::MAX
        })));

        assert_eq!(policy.reviewer_timeout_seconds, 3600);
        assert_eq!(policy.judge_timeout_seconds, 3600);
    }

    #[test]
    fn deep_review_budget_tracker_caps_judge_per_turn() {
        let policy = DeepReviewExecutionPolicy::default();
        let tracker = DeepReviewBudgetTracker::default();

        tracker
            .record_task("turn-1", &policy, DeepReviewSubagentRole::Judge, "ReviewJudge", false)
            .unwrap();
        assert!(tracker
            .record_task("turn-1", &policy, DeepReviewSubagentRole::Judge, "ReviewJudge", false,)
            .is_err());

        tracker
            .record_task("turn-2", &policy, DeepReviewSubagentRole::Judge, "ReviewJudge", false)
            .unwrap();
    }

    #[test]
    fn deep_review_concurrency_policy_blocks_reviewer_at_cap() {
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        assert!(policy
            .check_launch_allowed(0, DeepReviewSubagentRole::Reviewer, false)
            .is_ok());
        assert!(policy
            .check_launch_allowed(1, DeepReviewSubagentRole::Reviewer, false)
            .is_ok());
        assert!(policy
            .check_launch_allowed(2, DeepReviewSubagentRole::Reviewer, false)
            .is_err());
    }

    #[test]
    fn deep_review_concurrency_policy_returns_structured_cap_rejection() {
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let violation = policy
            .check_launch_allowed(2, DeepReviewSubagentRole::Reviewer, false)
            .expect_err("reviewer launch at cap should be rejected");
        let message = format!(
            "DeepReview concurrency policy violation: {}",
            violation.to_tool_error_message()
        );

        assert!(message.contains("deep_review_concurrency_cap_reached"));
        assert!(message.contains("Maximum parallel reviewer instances reached"));
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_waits_while_active_reviewer_is_running() {
        let turn_id = "turn-queue-active-wait";
        let tool_id = "tool-queue-active-wait";
        let occupied_a = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 2)
            .expect("precondition should occupy first reviewer capacity");
        let occupied_b = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 2)
            .expect("precondition should occupy second reviewer capacity");
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 0,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            deep_review_task_adapter::wait_for_reviewer_admission(
                "session-queue-active-wait",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                false,
                None,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "active Deep Review reviewers should keep the queued reviewer alive"
        );

        drop(occupied_a);
        drop(occupied_b);

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle)
            .await
            .expect("queue should become ready after active reviewers finish")
            .expect("spawned wait should not panic")
            .expect("queue wait should resolve");

        match outcome {
            DeepReviewQueueWaitOutcome::Ready { .. } => {}
            DeepReviewQueueWaitOutcome::Skipped { .. } => {
                panic!("active Deep Review reviewers should not cause a queue-expired skip");
            }
        }
        assert_eq!(deep_review_capacity_skip_count(turn_id), 0);
        assert_eq!(deep_review_concurrency_cap_rejection_count(turn_id), 0);
        assert_eq!(deep_review_effective_parallel_instances(turn_id, 2), 2);
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_starts_later_batch_when_reviewer_capacity_frees() {
        let turn_id = "turn-launch-batch-fill-free-slot";
        let tool_id = "tool-launch-batch-fill-free-slot";
        let occupied_a = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer_for_launch_batch(
            turn_id,
            2,
            1,
            Some("packet-a"),
        )
        .expect("launch batch admission should not fail")
        .expect("first batch reviewer should start");
        let occupied_b = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer_for_launch_batch(
            turn_id,
            2,
            1,
            Some("packet-b"),
        )
        .expect("launch batch admission should not fail")
        .expect("second first-batch reviewer should start");
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 2,
            stagger_seconds: 0,
            max_queue_wait_seconds: 0,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let launch_batch_info = DeepReviewLaunchBatchInfo {
            packet_id: Some("packet-b".to_string()),
            launch_batch: 2,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            wait_for_deep_review_reviewer_admission(
                "session-launch-batch-queue-wait",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                false,
                Some(&launch_batch_info),
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "later launch batch should wait while reviewer capacity is full"
        );
        drop(occupied_a);

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle)
            .await
            .expect("later launch batch should become ready as soon as reviewer capacity frees")
            .expect("spawned wait should not panic")
            .expect("queue wait should resolve");

        match outcome {
            DeepReviewQueueWaitOutcome::Ready { .. } => {}
            DeepReviewQueueWaitOutcome::Skipped { .. } => {
                panic!("later launch batch should not expire after reviewer capacity frees.");
            }
        }
        drop(occupied_b);
        assert_eq!(deep_review_capacity_skip_count(turn_id), 0);
        assert_eq!(deep_review_effective_parallel_instances(turn_id, 2), 2);
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_cancel_control_skips_waiting_reviewer() {
        let turn_id = "turn-queue-cancel";
        let tool_id = "tool-queue-cancel";
        let _occupied = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 1)
            .expect("precondition should occupy reviewer capacity");
        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Cancel);
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 1,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };

        let outcome = deep_review_task_adapter::wait_for_reviewer_admission(
            "session-queue-cancel",
            turn_id,
            tool_id,
            "ReviewSecurity",
            &policy,
            false,
            None,
        )
        .await
        .expect("queue wait should resolve");

        match outcome {
            DeepReviewQueueWaitOutcome::Skipped { queue_elapsed_ms, .. } => {
                assert!(queue_elapsed_ms < 100);
            }
            DeepReviewQueueWaitOutcome::Ready { .. } => {
                panic!("cancelled queue control should skip the waiting reviewer");
            }
        }
        assert_eq!(deep_review_capacity_skip_count(turn_id), 1);
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_records_one_runtime_wait_when_ready() {
        let turn_id = "turn-queue-ready-diagnostics";
        let tool_id = "tool-queue-ready-diagnostics";
        let occupied = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 1)
            .expect("precondition should occupy reviewer capacity");
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 1,
            stagger_seconds: 0,
            max_queue_wait_seconds: 1,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            deep_review_task_adapter::wait_for_reviewer_admission(
                "session-queue-ready-diagnostics",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                false,
                None,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        drop(occupied);

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle)
            .await
            .expect("queue should become ready after capacity frees")
            .expect("spawned wait should not panic")
            .expect("queue wait should resolve");
        match outcome {
            DeepReviewQueueWaitOutcome::Ready { .. } => {}
            DeepReviewQueueWaitOutcome::Skipped { .. } => {
                panic!("freed capacity should allow the queued reviewer to run");
            }
        }

        let diagnostics = crate::agentic::deep_review_policy::deep_review_runtime_diagnostics_snapshot(turn_id)
            .expect("runtime diagnostics should record terminal queue wait");
        assert_eq!(diagnostics.queue_wait_count, 1);
        assert_eq!(diagnostics.queue_wait_total_ms, diagnostics.queue_wait_max_ms);
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_pause_does_not_expire_until_continued() {
        let turn_id = "turn-queue-pause";
        let tool_id = "tool-queue-pause";
        let occupied = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 1)
            .expect("precondition should occupy reviewer capacity");
        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Pause);
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 1,
            stagger_seconds: 0,
            max_queue_wait_seconds: 0,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };
        let turn_id_owned = turn_id.to_string();
        let tool_id_owned = tool_id.to_string();

        let handle = tokio::spawn(async move {
            deep_review_task_adapter::wait_for_reviewer_admission(
                "session-queue-pause",
                &turn_id_owned,
                &tool_id_owned,
                "ReviewSecurity",
                &policy,
                false,
                None,
            )
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "paused queue wait should not expire while user pause is active"
        );

        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::Continue);
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        assert!(
            !handle.is_finished(),
            "continued queue wait should stay alive while reviewer capacity is still active"
        );
        drop(occupied);

        let outcome = tokio::time::timeout(tokio::time::Duration::from_millis(500), handle)
            .await
            .expect("continued queue wait should finish")
            .expect("spawned wait should not panic")
            .expect("queue wait should resolve");
        match outcome {
            DeepReviewQueueWaitOutcome::Ready { .. } => {}
            DeepReviewQueueWaitOutcome::Skipped { .. } => {
                panic!("continued queue wait should run after reviewer capacity frees");
            }
        }
    }

    #[tokio::test]
    async fn deep_review_capacity_queue_skip_optional_skips_optional_waiter() {
        let turn_id = "turn-queue-skip-optional";
        let tool_id = "tool-queue-skip-optional";
        let _occupied = crate::agentic::deep_review_policy::try_begin_deep_review_active_reviewer(turn_id, 1)
            .expect("precondition should occupy reviewer capacity");
        apply_deep_review_queue_control(turn_id, tool_id, DeepReviewQueueControlAction::SkipOptional);
        let policy = DeepReviewConcurrencyPolicy {
            max_parallel_instances: 1,
            stagger_seconds: 0,
            max_queue_wait_seconds: 60,
            batch_extras_separately: true,
            allow_bounded_auto_retry: false,
            auto_retry_elapsed_guard_seconds: 180,
        };

        let outcome = deep_review_task_adapter::wait_for_reviewer_admission(
            "session-queue-skip-optional",
            turn_id,
            tool_id,
            "ReviewCustom",
            &policy,
            true,
            None,
        )
        .await
        .expect("queue wait should resolve");

        match outcome {
            DeepReviewQueueWaitOutcome::Skipped { queue_elapsed_ms, .. } => {
                assert!(queue_elapsed_ms < 100);
            }
            DeepReviewQueueWaitOutcome::Ready { .. } => {
                panic!("optional queue control should skip optional reviewer");
            }
        }
    }

    #[test]
    fn deep_review_concurrency_policy_blocks_judge_with_active_reviewers() {
        let policy = DeepReviewConcurrencyPolicy::default();
        assert!(policy
            .check_launch_allowed(1, DeepReviewSubagentRole::Judge, false)
            .is_err());
        assert!(policy
            .check_launch_allowed(0, DeepReviewSubagentRole::Judge, false)
            .is_ok());
        assert!(policy
            .check_launch_allowed(0, DeepReviewSubagentRole::Judge, true)
            .is_err());
    }

    #[test]
    fn deep_review_retry_guidance_includes_budget_info() {
        assert_eq!(deep_review_max_retries_per_role("nonexistent-turn"), 1);
        assert_eq!(deep_review_retries_used("nonexistent-turn", "ReviewSecurity"), 0);
    }

    #[test]
    fn deep_review_retry_guidance_uses_manifest_policy_limit() {
        let manifest = json!({
            "reviewMode": "deep",
            "executionPolicy": {
                "maxRetriesPerRole": 2
            }
        });
        let policy = DeepReviewExecutionPolicy::default().with_run_manifest_execution_policy(&manifest);

        assert_eq!(
            deep_review_retry_guidance_max_retries(Some(&policy), "nonexistent-turn"),
            2
        );
    }

    #[test]
    fn deep_review_retry_guidance_only_applies_to_initial_reviewer_timeout() {
        assert!(should_emit_deep_review_retry_guidance(
            true,
            false,
            Some(DeepReviewSubagentRole::Reviewer)
        ));
        assert!(!should_emit_deep_review_retry_guidance(true, false, None));
        assert!(!should_emit_deep_review_retry_guidance(
            true,
            false,
            Some(DeepReviewSubagentRole::Judge)
        ));
        assert!(!should_emit_deep_review_retry_guidance(
            true,
            true,
            Some(DeepReviewSubagentRole::Reviewer)
        ));
        assert!(!should_emit_deep_review_retry_guidance(
            false,
            false,
            Some(DeepReviewSubagentRole::Reviewer)
        ));
    }
}
