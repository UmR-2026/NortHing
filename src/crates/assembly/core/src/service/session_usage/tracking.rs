#[cfg(test)]
mod tests {
    use super::super::breakdowns_core::*;
    use super::super::service::build_session_usage_report_from_turns;
    use super::super::service::test_helpers::*;
    use super::super::SessionUsageReport;
    use super::super::SessionUsageReportRequest;
    use super::super::UsageCacheCoverage;
    use super::super::UsageCoverageKey;
    use super::super::UsageModelBreakdown;
    use super::super::UsageScope;
    use super::super::UsageSlowSpanKind;
    use super::super::UsageToolBreakdown;
    use super::super::UsageWorkspaceKind;
    use crate::service::session::{DialogTurnKind, ToolResultData};

    #[test]
    fn report_marks_cache_unavailable_for_zero_filled_cache_source() {
        let request = test_request(None);
        let records = vec![test_token_record("model-a", 100, 20, 0)];

        let report = build_session_usage_report_from_turns(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &records,
            1_778_347_200_000,
        );

        assert_eq!(report.tokens.total_tokens, Some(120));
        assert_eq!(report.tokens.cached_tokens, None);
        assert_eq!(report.tokens.cache_coverage, UsageCacheCoverage::Unavailable);
        assert!(report.coverage.missing.contains(&UsageCoverageKey::CachedTokens));
    }

    #[test]
    fn report_uses_cached_tokens_when_provider_reports_them() {
        let request = test_request(None);
        let mut records = vec![test_token_record("model-a", 100, 20, 12)];
        records[0].cached_tokens_available = true;

        let report = build_session_usage_report_from_turns(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &records,
            1_778_347_200_000,
        );

        assert_eq!(report.tokens.cached_tokens, Some(12));
        assert_eq!(report.tokens.cache_coverage, UsageCacheCoverage::Available);
        assert_eq!(report.models[0].cached_tokens, Some(12));
        assert!(report.coverage.available.contains(&UsageCoverageKey::CachedTokens));
    }

    #[test]
    fn report_scopes_by_workspace_identity() {
        let request = test_request(None);

        let report = build_session_usage_report_from_turns(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &[],
            1_778_347_200_000,
        );

        assert_eq!(report.session_id, "session-1");
        assert_eq!(report.workspace.kind, UsageWorkspaceKind::Local);
        assert_eq!(report.workspace.path_label.as_deref(), Some("D:/workspace/northhing"));
    }

    #[test]
    fn report_counts_failed_and_cancelled_tool_duration_when_available() {
        let request = test_request(None);
        let turn = test_turn_with_tools(
            "turn-1",
            0,
            DialogTurnKind::UserDialog,
            vec![
                test_tool_item(
                    "tool-failed",
                    "write_file",
                    Some(false),
                    120,
                    "D:/workspace/northhing/src/main.rs",
                ),
                test_tool_item(
                    "tool-cancelled",
                    "edit_file",
                    None,
                    80,
                    "D:/workspace/northhing/src/lib.rs",
                ),
            ],
        );

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        let failed = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "write_file")
            .expect("failed tool row");
        assert_eq!(failed.error_count, 1);
        assert_eq!(failed.duration_ms, Some(120));

        let cancelled = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "edit_file")
            .expect("cancelled tool row");
        assert_eq!(cancelled.call_count, 1);
        assert_eq!(cancelled.duration_ms, Some(80));
    }

    #[test]
    fn report_computes_tool_p95_only_with_multiple_duration_spans() {
        let request = test_request(None);
        let turn = test_turn_with_tools(
            "turn-1",
            0,
            DialogTurnKind::UserDialog,
            vec![
                test_tool_item(
                    "tool-1",
                    "write_file",
                    Some(true),
                    10,
                    "D:/workspace/northhing/src/a.rs",
                ),
                test_tool_item(
                    "tool-2",
                    "write_file",
                    Some(true),
                    100,
                    "D:/workspace/northhing/src/b.rs",
                ),
                test_tool_item(
                    "tool-3",
                    "write_file",
                    Some(true),
                    200,
                    "D:/workspace/northhing/src/c.rs",
                ),
                test_tool_item("tool-4", "edit_file", Some(true), 60, "D:/workspace/northhing/src/d.rs"),
            ],
        );

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        let write = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "write_file")
            .expect("write tool row");
        assert_eq!(write.duration_ms, Some(310));
        assert_eq!(write.p95_duration_ms, Some(200));

        let edit = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "edit_file")
            .expect("edit tool row");
        assert_eq!(edit.p95_duration_ms, None);
    }

    #[test]
    fn report_sums_tool_phase_timings_and_marks_phase_coverage_available() {
        let request = test_request(None);
        let mut first = test_tool_item(
            "tool-1",
            "write_file",
            Some(true),
            100,
            "D:/workspace/northhing/src/a.rs",
        );
        first.queue_wait_ms = Some(7);
        first.preflight_ms = Some(11);
        first.confirmation_wait_ms = Some(13);
        first.execution_ms = Some(69);

        let mut second = test_tool_item(
            "tool-2",
            "write_file",
            Some(true),
            80,
            "D:/workspace/northhing/src/b.rs",
        );
        second.queue_wait_ms = Some(3);
        second.preflight_ms = Some(5);
        second.confirmation_wait_ms = Some(0);
        second.execution_ms = Some(72);

        let turn = test_turn_with_tools("turn-1", 0, DialogTurnKind::UserDialog, vec![first, second]);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        let write = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "write_file")
            .expect("write tool row");
        assert_eq!(write.duration_ms, Some(180));
        assert_eq!(write.queue_wait_ms, Some(10));
        assert_eq!(write.preflight_ms, Some(16));
        assert_eq!(write.confirmation_wait_ms, Some(13));
        assert_eq!(write.execution_ms, Some(141));
        assert!(report.coverage.available.contains(&UsageCoverageKey::ToolPhaseTiming));
        assert!(!report.coverage.missing.contains(&UsageCoverageKey::ToolPhaseTiming));
    }

    #[test]
    fn report_slowest_tool_spans_include_diagnostic_fields() {
        let request = test_request(None);
        let mut slow = test_tool_item_with_input(
            "tool-slow",
            "Bash",
            Some(false),
            95_000,
            serde_json::json!({
                "command": "curl https://api.example.test/slow",
                "timeout_seconds": 90
            }),
        );
        slow.queue_wait_ms = Some(5);
        slow.preflight_ms = Some(10);
        slow.confirmation_wait_ms = Some(15);
        slow.execution_ms = Some(94_970);
        slow.tool_result = Some(ToolResultData {
            result: serde_json::json!({
                "exit_code": 28,
                "timed_out": true,
                "stderr": "operation timed out"
            }),
            success: false,
            result_for_assistant: None,
            error: Some("operation timed out".to_string()),
            duration_ms: Some(95_000),
        });
        let turn = test_turn_with_tools("turn-1", 0, DialogTurnKind::UserDialog, vec![slow]);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);
        let span = report
            .slowest
            .iter()
            .find(|span| span.kind == UsageSlowSpanKind::Tool)
            .expect("slow tool span");

        assert_eq!(span.item_id.as_deref(), Some("tool-slow"));
        assert_eq!(
            span.input_summary.as_deref(),
            Some("curl https://api.example.test/slow")
        );
        assert_eq!(span.status.as_deref(), Some("failed"));
        assert_eq!(span.exit_code, Some(28));
        assert_eq!(span.timed_out, Some(true));
        assert_eq!(span.error_summary.as_deref(), Some("operation timed out"));
        assert_eq!(span.execution_ms, Some(94_970));
    }

    #[test]
    fn report_slowest_tool_spans_summarize_url_inputs() {
        let request = test_request(None);
        let slow = test_tool_item_with_input(
            "tool-slow-url",
            "web_fetch",
            Some(true),
            95_000,
            serde_json::json!({
                "method": "GET",
                "url": "https://api.example.test/slow"
            }),
        );
        let turn = test_turn_with_tools("turn-1", 0, DialogTurnKind::UserDialog, vec![slow]);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);
        let span = report
            .slowest
            .iter()
            .find(|span| span.kind == UsageSlowSpanKind::Tool)
            .expect("slow tool span");

        assert_eq!(span.input_summary.as_deref(), Some("GET https://api.example.test/slow"));
    }

    #[test]
    fn cache_hit_rate_computes_when_all_records_report_cache() {
        let records = vec![
            reported_token_record("model-a", 100, 20, 30),
            reported_token_record("model-a", 200, 40, 80),
        ];
        let breakdown = super::super::breakdowns_core::build_token_breakdown(&records);
        // (30 + 80) / (100 + 200) = 110 / 300
        let rate = breakdown.cache_hit_rate.expect("hit rate present");
        assert!((rate - (110.0 / 300.0)).abs() < 1e-9);
    }

    #[test]
    fn cache_hit_rate_is_none_when_no_record_reports_cache() {
        let records = vec![
            test_token_record("model-a", 100, 20, 0),
            test_token_record("model-a", 200, 40, 0),
        ];
        let breakdown = super::super::breakdowns_core::build_token_breakdown(&records);
        assert_eq!(breakdown.cache_hit_rate, None);
    }

    #[test]
    fn cache_hit_rate_excludes_unreported_records_from_denominator() {
        // Partial coverage: one record reports, the other does not. The
        // unreported record must be excluded from BOTH numerator and
        // denominator — otherwise hit rate is artificially deflated.
        let records = vec![
            reported_token_record("model-a", 100, 20, 80), // reports → counts
            test_token_record("model-a", 9999, 1, 0),      // unreported → excluded
        ];
        let breakdown = super::super::breakdowns_core::build_token_breakdown(&records);
        let rate = breakdown.cache_hit_rate.expect("hit rate present");
        // 80 / 100 — the 9999 input from the unreported record must NOT bloat the denominator.
        assert!((rate - 0.8).abs() < 1e-9);
    }

    #[test]
    fn cache_hit_rate_none_when_input_sum_is_zero() {
        // Edge case: reported records but their input_tokens all 0.
        // Avoid divide-by-zero; surface as None.
        let records = vec![reported_token_record("model-a", 0, 5, 0)];
        let breakdown = super::super::breakdowns_core::build_token_breakdown(&records);
        assert_eq!(breakdown.cache_hit_rate, None);
    }

    #[test]
    fn per_model_cache_hit_rate_isolated_per_model() {
        let records = vec![
            reported_token_record("model-a", 100, 10, 40), // a: 40/100
            reported_token_record("model-b", 200, 20, 50), // b: 50/200
        ];
        let models = super::super::breakdowns_core::build_model_breakdown(&[], &records);
        let a = models.iter().find(|m| m.model_id == "model-a").unwrap();
        let b = models.iter().find(|m| m.model_id == "model-b").unwrap();
        assert!((a.cache_hit_rate.unwrap() - 0.4).abs() < 1e-9);
        assert!((b.cache_hit_rate.unwrap() - 0.25).abs() < 1e-9);
    }
}
