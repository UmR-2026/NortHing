//! Tests for `CodeReviewTool`.
//!
//! Kept in a separate `tests.rs` so `mod.rs` stays under the 600-line
//! facade budget while preserving all existing test behaviour.

use super::CodeReviewTool;
use crate::agentic::core::{CompressionContract, CompressionContractItem};
use crate::agentic::deep_review_policy::{
    deep_review_runtime_diagnostics_snapshot, record_deep_review_capacity_skip,
    record_deep_review_concurrency_cap_rejection, record_deep_review_shared_context_tool_use,
};
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use serde_json::json;
use std::collections::HashMap;

fn tool_context(agent_type: Option<&str>) -> ToolUseContext {
    ToolUseContext {
        tool_call_id: None,
        agent_type: agent_type.map(str::to_string),
        session_id: None,
        dialog_turn_id: None,
        workspace: None,
        unlocked_collapsed_tools: Vec::new(),
        custom_data: HashMap::new(),
        computer_use_host: None,
        runtime_tool_restrictions: Default::default(),
        runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
        actor_runtime: None,
    }
}

#[tokio::test]
async fn deep_review_schema_requires_deep_review_fields() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let schema = tool.input_schema_for_model_with_context(Some(&context)).await;
    let required = schema["required"].as_array().expect("required fields");

    for field in ["review_mode", "review_scope", "reviewers", "remediation_plan"] {
        assert!(
            required.iter().any(|value| value.as_str() == Some(field)),
            "DeepReview schema should require {field}"
        );
    }
}

#[tokio::test]
async fn deep_review_schema_accepts_reviewer_partial_output() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let schema = tool.input_schema_for_model_with_context(Some(&context)).await;
    let reviewer_properties = &schema["properties"]["reviewers"]["items"]["properties"];

    assert_eq!(reviewer_properties["partial_output"]["type"], "string");
}

#[tokio::test]
async fn deep_review_schema_accepts_reviewer_packet_fallback_metadata() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let schema = tool.input_schema_for_model_with_context(Some(&context)).await;
    let reviewer_properties = &schema["properties"]["reviewers"]["items"]["properties"];

    assert_eq!(reviewer_properties["packet_id"]["type"], "string");
    assert_eq!(
        reviewer_properties["packet_status_source"]["enum"],
        json!(["reported", "inferred", "missing"])
    );
}

#[tokio::test]
async fn deep_review_schema_accepts_structured_reliability_signals() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let schema = tool.input_schema_for_model_with_context(Some(&context)).await;
    let reliability_properties = &schema["properties"]["reliability_signals"]["items"]["properties"];

    assert_eq!(
        reliability_properties["kind"]["enum"],
        json!([
            "context_pressure",
            "compression_preserved",
            "cache_hit",
            "cache_miss",
            "concurrency_limited",
            "partial_reviewer",
            "reduced_scope",
            "retry_guidance",
            "skipped_reviewers",
            "token_budget_limited",
            "user_decision"
        ])
    );
    assert_eq!(
        reliability_properties["source"]["enum"],
        json!(["runtime", "manifest", "report", "inferred"])
    );
}

#[tokio::test]
async fn deep_review_submission_defaults_missing_mode_to_deep() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "No blocking issues",
                    "risk_level": "low",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": []
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert_eq!(data["review_mode"], "deep");
    assert!(data["reviewers"].as_array().is_some());
    assert!(data["remediation_plan"].as_array().is_some());
}

#[tokio::test]
async fn deep_review_submission_infers_unique_reviewer_packet_from_manifest() {
    let tool = CodeReviewTool::new();
    let mut context = tool_context(Some("DeepReview"));
    context.custom_data.insert(
        "deep_review_run_manifest".to_string(),
        json!({
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "displayName": "Security Reviewer",
                    "roleName": "Security Reviewer"
                }
            ]
        }),
    );

    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "No blocking issues",
                    "risk_level": "low",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": [],
                "reviewers": [
                    {
                        "name": "Security Reviewer",
                        "specialty": "security",
                        "status": "completed",
                        "summary": "Checked the security packet."
                    }
                ]
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert_eq!(data["reviewers"][0]["packet_id"], "reviewer:ReviewSecurity");
    assert_eq!(data["reviewers"][0]["packet_status_source"], "inferred");
}

#[tokio::test]
async fn deep_review_submission_marks_uninferable_packet_metadata_as_missing() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "No blocking issues",
                    "risk_level": "low",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": [],
                "reviewers": [
                    {
                        "name": "Unknown Reviewer",
                        "specialty": "unknown",
                        "status": "completed",
                        "summary": "Packet was omitted."
                    }
                ]
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert!(data["reviewers"][0].get("packet_id").is_none());
    assert_eq!(data["reviewers"][0]["packet_status_source"], "missing");
}

#[tokio::test]
async fn deep_review_submission_marks_existing_packet_metadata_as_reported() {
    let tool = CodeReviewTool::new();
    let context = tool_context(Some("DeepReview"));
    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "No blocking issues",
                    "risk_level": "low",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": [],
                "reviewers": [
                    {
                        "name": "Security Reviewer",
                        "specialty": "security",
                        "status": "completed",
                        "summary": "Packet was reported.",
                        "packet_id": "reviewer:ReviewSecurity"
                    }
                ]
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert_eq!(data["reviewers"][0]["packet_id"], "reviewer:ReviewSecurity");
    assert_eq!(data["reviewers"][0]["packet_status_source"], "reported");
}

#[tokio::test]
async fn deep_review_submission_fills_runtime_reliability_signals() {
    let tool = CodeReviewTool::new();
    let mut context = tool_context(Some("DeepReview"));
    context.custom_data.insert(
        "deep_review_run_manifest".to_string(),
        json!({
            "tokenBudget": {
                "largeDiffSummaryFirst": true,
                "warnings": [],
                "estimatedReviewerCalls": 7,
                "skippedReviewerIds": ["CustomPerf"]
            },
            "skippedReviewers": [
                {
                    "subagentId": "ReviewFrontend",
                    "reason": "not_applicable"
                },
                {
                    "subagentId": "CustomPerf",
                    "reason": "budget_limited"
                }
            ]
        }),
    );

    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "Review completed with reduced confidence",
                    "risk_level": "medium",
                    "recommended_action": "request_changes"
                },
                "issues": [],
                "positive_points": [],
                "reviewers": [
                    {
                        "name": "Security Reviewer",
                        "specialty": "security",
                        "status": "partial_timeout",
                        "summary": "Timed out after partial evidence.",
                        "partial_output": "Found one likely issue before timeout."
                    }
                ],
                "report_sections": {
                    "remediation_groups": {
                        "needs_decision": [
                            "Decide whether to block the release."
                        ]
                    }
                }
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert_eq!(
        data["reliability_signals"],
        json!([
            {
                "kind": "context_pressure",
                "severity": "info",
                "count": 7,
                "source": "runtime"
            },
            {
                "kind": "skipped_reviewers",
                "severity": "info",
                "count": 2,
                "source": "manifest"
            },
            {
                "kind": "token_budget_limited",
                "severity": "warning",
                "count": 1,
                "source": "manifest"
            },
            {
                "kind": "partial_reviewer",
                "severity": "warning",
                "count": 1,
                "source": "runtime"
            },
            {
                "kind": "retry_guidance",
                "severity": "warning",
                "count": 1,
                "source": "runtime"
            },
            {
                "kind": "user_decision",
                "severity": "action",
                "count": 1,
                "source": "report"
            }
        ])
    );
}

#[tokio::test]
async fn deep_review_submission_fills_concurrency_limited_from_runtime_tracker() {
    use crate::agentic::deep_review_policy::record_deep_review_concurrency_cap_rejection;

    let tool = CodeReviewTool::new();
    let mut context = tool_context(Some("DeepReview"));
    context.dialog_turn_id = Some("turn-code-review-cap-signal".to_string());
    record_deep_review_concurrency_cap_rejection("turn-code-review-cap-signal");

    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "Review completed with launch backpressure",
                    "risk_level": "medium",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": []
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert_eq!(
        data["reliability_signals"],
        json!([
            {
                "kind": "concurrency_limited",
                "severity": "warning",
                "count": 1,
                "source": "runtime"
            }
        ])
    );
}

#[tokio::test]
async fn deep_review_shared_context_diagnostics_stays_out_of_report() {
    let turn_id = "turn-code-review-shared-context-diagnostics";
    record_deep_review_shared_context_tool_use(turn_id, "ReviewSecurity", "Read", "src/lib.rs");
    record_deep_review_shared_context_tool_use(turn_id, "ReviewPerformance", "Read", "src/lib.rs");
    record_deep_review_shared_context_tool_use(turn_id, "ReviewArchitecture", "GetFileDiff", "src/lib.rs");

    let diagnostics =
        deep_review_runtime_diagnostics_snapshot(turn_id).expect("diagnostics should be available for measured turn");
    assert_eq!(diagnostics.shared_context_total_calls, 3);
    assert_eq!(diagnostics.shared_context_duplicate_calls, 1);
    assert_eq!(diagnostics.shared_context_duplicate_context_count, 1);
    assert_eq!(diagnostics.shared_context_duplicate_savings_candidate_count, 1);

    let tool = CodeReviewTool::new();
    let mut context = tool_context(Some("DeepReview"));
    context.dialog_turn_id = Some(turn_id.to_string());

    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "Review completed",
                    "risk_level": "low",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": []
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };
    assert!(data.get("shared_context_measurement").is_none());
    assert!(data.get("runtime_diagnostics").is_none());
    assert!(data.get("reliability_signals").is_none());
}

#[tokio::test]
async fn deep_review_submission_folds_capacity_skips_into_concurrency_limited_signal() {
    record_deep_review_capacity_skip("turn-code-review-capacity-skip");

    let tool = CodeReviewTool::new();
    let mut context = tool_context(Some("DeepReview"));
    context.dialog_turn_id = Some("turn-code-review-capacity-skip".to_string());

    let result = tool
        .call_impl(
            &json!({
                "summary": {
                    "overall_assessment": "Review completed after queue skip",
                    "risk_level": "medium",
                    "recommended_action": "approve"
                },
                "issues": [],
                "positive_points": []
            }),
            &context,
        )
        .await
        .expect("submit review result");

    let ToolResult::Result { data, .. } = &result[0] else {
        panic!("expected tool result");
    };

    assert_eq!(
        data["reliability_signals"],
        json!([
            {
                "kind": "concurrency_limited",
                "severity": "warning",
                "count": 1,
                "source": "runtime"
            }
        ])
    );
}

#[test]
fn deep_review_defaults_include_compression_contract_reliability_signal() {
    let contract = CompressionContract {
        touched_files: vec!["src/web-ui/src/flow_chat/utils/codeReviewReport.ts".to_string()],
        verification_commands: vec![CompressionContractItem {
            target: "pnpm --dir src/web-ui run test:run".to_string(),
            status: "succeeded".to_string(),
            summary: "Frontend report tests passed.".to_string(),
            error_kind: None,
        }],
        blocking_failures: vec![],
        subagent_statuses: vec![],
    };
    let mut input = json!({
        "summary": {
            "overall_assessment": "No blocking issues",
            "risk_level": "low",
            "recommended_action": "approve"
        },
        "issues": [],
        "positive_points": []
    });

    CodeReviewTool::validate_and_fill_defaults(&mut input, true, None, Some(&contract));

    assert_eq!(
        input["reliability_signals"],
        json!([
            {
                "kind": "compression_preserved",
                "severity": "info",
                "count": 2,
                "source": "runtime"
            }
        ])
    );
}

#[test]
fn deep_review_reliability_contract_limit_uses_context_profile_policy() {
    assert_eq!(
        CodeReviewTool::reliability_contract_limit(Some("DeepReview"), Some("gpt-5")),
        8
    );
    assert_eq!(
        CodeReviewTool::reliability_contract_limit(Some("DeepReview"), Some("gpt-5-mini")),
        4
    );
}

#[test]
fn deep_review_defaults_include_reduced_scope_reliability_signal() {
    let manifest = json!({
        "reviewMode": "deep",
        "scopeProfile": {
            "reviewDepth": "high_risk_only",
            "riskFocusTags": ["security"],
            "maxDependencyHops": 0,
            "optionalReviewerPolicy": "risk_matched_only",
            "allowBroadToolExploration": false,
            "coverageExpectation": "High-risk-only pass; changed files stay visible."
        }
    });
    let mut input = json!({
        "summary": {
            "overall_assessment": "No blocking issues",
            "risk_level": "low",
            "recommended_action": "approve"
        },
        "issues": [],
        "positive_points": []
    });

    CodeReviewTool::validate_and_fill_defaults(&mut input, true, Some(&manifest), None);

    assert_eq!(
        input["reliability_signals"],
        json!([
            {
                "kind": "reduced_scope",
                "severity": "info",
                "source": "manifest",
                "detail": "High-risk-only pass; changed files stay visible."
            }
        ])
    );
}

#[test]
fn deep_review_legacy_manifest_without_scope_profile_has_no_reduced_scope_signal() {
    let manifest = json!({
        "reviewMode": "deep",
        "workPackets": []
    });
    let mut input = json!({
        "summary": {
            "overall_assessment": "No blocking issues",
            "risk_level": "low",
            "recommended_action": "approve"
        },
        "issues": [],
        "positive_points": []
    });

    CodeReviewTool::validate_and_fill_defaults(&mut input, true, Some(&manifest), None);

    assert!(input.get("reliability_signals").is_none());
}

#[test]
fn deep_review_invalid_evidence_pack_becomes_manifest_reliability_signal() {
    let manifest = json!({
        "reviewMode": "deep",
        "evidencePack": {
            "version": 1,
            "source": "target_manifest",
            "changedFiles": ["src/lib.rs"],
            "diffStat": {
                "fileCount": 1,
                "lineCountSource": "diff_stat"
            },
            "domainTags": ["core"],
            "riskFocusTags": ["security"],
            "packetIds": ["reviewer:ReviewSecurity"],
            "hunkHints": [],
            "contractHints": [],
            "budget": {
                "maxChangedFiles": 80,
                "maxHunkHints": 80,
                "maxContractHints": 40,
                "omittedChangedFileCount": 0,
                "omittedHunkHintCount": 0,
                "omittedContractHintCount": 0
            },
            "privacy": {
                "content": "full_diff",
                "excludes": [
                    "source_text",
                    "full_diff",
                    "model_output",
                    "provider_raw_body",
                    "full_file_contents"
                ]
            }
        }
    });
    let mut input = json!({
        "summary": {
            "overall_assessment": "No blocking issues",
            "risk_level": "low",
            "recommended_action": "approve"
        },
        "issues": [],
        "positive_points": []
    });

    CodeReviewTool::validate_and_fill_defaults(&mut input, true, Some(&manifest), None);

    let signals = input["reliability_signals"]
        .as_array()
        .expect("invalid evidence pack should emit a reliability signal");
    assert_eq!(signals[0]["kind"], "context_pressure");
    assert_eq!(signals[0]["severity"], "warning");
    assert_eq!(signals[0]["source"], "manifest");
    assert!(signals[0]["detail"]
        .as_str()
        .expect("signal should include detail")
        .contains("privacy.content"));
}

#[test]
fn deep_review_full_depth_manifest_has_no_reduced_scope_signal() {
    let manifest = json!({
        "reviewMode": "deep",
        "scopeProfile": {
            "reviewDepth": "full_depth",
            "riskFocusTags": ["security"],
            "maxDependencyHops": "policy_limited",
            "optionalReviewerPolicy": "full",
            "allowBroadToolExploration": true,
            "coverageExpectation": "Full-depth pass."
        }
    });
    let mut input = json!({
        "summary": {
            "overall_assessment": "No blocking issues",
            "risk_level": "low",
            "recommended_action": "approve"
        },
        "issues": [],
        "positive_points": []
    });

    CodeReviewTool::validate_and_fill_defaults(&mut input, true, Some(&manifest), None);

    assert!(input.get("reliability_signals").is_none());
}

#[test]
fn deep_review_compression_signal_requires_completed_compression() {
    let contract = CompressionContract {
        touched_files: vec!["src/main.rs".to_string()],
        verification_commands: vec![],
        blocking_failures: vec![],
        subagent_statuses: vec![],
    };

    assert!(!CodeReviewTool::should_report_compression_preserved(0, Some(&contract)));
    assert!(CodeReviewTool::should_report_compression_preserved(1, Some(&contract)));
    assert!(!CodeReviewTool::should_report_compression_preserved(
        1,
        Some(&CompressionContract::default())
    ));
}
