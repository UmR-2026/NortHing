#[cfg(test)]
mod tests {
    use super::super::breakdowns_core::*;
    use super::super::breakdowns_extra::*;
    use super::super::service::test_helpers::*;
    use super::super::service::{build_session_usage_report_from_sources, build_session_usage_report_from_turns};
    use super::super::snapshot::*;
    use super::super::utilities::*;
    use super::super::SessionUsageReport;
    use super::super::SessionUsageReportRequest;
    use super::super::UsageCoverageKey;
    use super::super::UsageErrorBreakdown;
    use super::super::UsageErrorExample;
    use super::super::UsageModelBreakdown;
    use super::super::UsageScope;
    use super::super::UsageSlowSpan;
    use super::super::UsageSlowSpanKind;
    use super::super::UsageTimeAccounting;
    use super::super::UsageTimeDenominator;
    use super::super::UsageToolBreakdown;
    use super::super::UsageWorkspaceKind;
    use crate::service::session::{DialogTurnKind, TurnStatus};
    use crate::service::session_usage::redaction::{
        display_workspace_relative_path, redact_usage_input_summary, redact_usage_label,
    };

    #[test]
    fn report_active_runtime_uses_active_span_union() {
        let request = test_request(None);
        let mut first = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        first.start_time = 1_000;
        first.end_time = Some(1_300);
        first.duration_ms = Some(300);
        first.model_rounds[0].start_time = 1_010;
        first.model_rounds[0].end_time = Some(1_110);
        first.model_rounds[0].duration_ms = Some(100);

        let mut second = test_turn("turn-2", 1, DialogTurnKind::ManualCompaction);
        second.start_time = 1_200;
        second.end_time = Some(1_500);
        second.duration_ms = Some(300);
        second.model_rounds[0].start_time = 1_220;
        second.model_rounds[0].end_time = Some(1_340);
        second.model_rounds[0].duration_ms = Some(120);

        let report = build_session_usage_report_from_turns(request, &[first, second], &[], 1_778_347_200_000);

        assert_eq!(report.time.accounting, UsageTimeAccounting::Exact);
        assert_eq!(report.time.denominator, UsageTimeDenominator::ActiveTurnTime);
        assert_eq!(report.time.wall_time_ms, Some(500));
        assert_eq!(report.time.active_turn_ms, Some(500));
        assert_eq!(report.time.model_ms, Some(220));
        assert_eq!(report.time.idle_gap_ms, Some(0));
        assert_eq!(report.compression.manual_compaction_count, 1);
    }

    #[test]
    fn report_active_runtime_includes_incomplete_turn_child_spans() {
        let request = test_request(None);
        let mut completed = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        completed.start_time = 1_000;
        completed.end_time = Some(61_000);
        completed.duration_ms = Some(60_000);
        completed.model_rounds[0].start_time = 2_000;
        completed.model_rounds[0].end_time = Some(12_000);
        completed.model_rounds[0].duration_ms = Some(10_000);
        completed.model_rounds[0].tool_items.clear();

        let mut incomplete = test_turn("turn-2", 1, DialogTurnKind::UserDialog);
        incomplete.start_time = 121_000;
        incomplete.end_time = None;
        incomplete.duration_ms = None;
        incomplete.model_rounds[0].start_time = 122_000;
        incomplete.model_rounds[0].end_time = Some(181_000);
        incomplete.model_rounds[0].duration_ms = Some(59_000);
        incomplete.model_rounds[0].tool_items = vec![test_tool_item_with_input(
            "slow-bash",
            "Bash",
            Some(true),
            120_000,
            serde_json::json!({
                "command": "pnpm install",
                "timeout_seconds": 300
            }),
        )];
        incomplete.model_rounds[0].tool_items[0].start_time = 181_000;
        incomplete.model_rounds[0].tool_items[0].end_time = Some(301_000);

        let report = build_session_usage_report_from_turns(request, &[completed, incomplete], &[], 1_778_347_200_000);

        assert_eq!(report.time.accounting, UsageTimeAccounting::Approximate);
        assert_eq!(report.time.wall_time_ms, Some(300_000));
        assert_eq!(report.time.active_turn_ms, Some(240_000));
        assert_eq!(report.time.tool_ms, Some(120_000));
    }

    #[test]
    fn report_excludes_local_command_turns_from_usage_metrics() {
        let request = test_request(None);
        let mut user_turn = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        user_turn.start_time = 1_000;
        user_turn.end_time = Some(1_300);
        user_turn.duration_ms = Some(300);
        user_turn.model_rounds[0].duration_ms = Some(200);

        let mut local_usage_turn = test_turn("local-usage-1", 1, DialogTurnKind::LocalCommand);
        local_usage_turn.start_time = 50_000;
        local_usage_turn.end_time = Some(50_000);
        local_usage_turn.duration_ms = Some(0);
        local_usage_turn.model_rounds[0].duration_ms = Some(9_000);

        let report =
            build_session_usage_report_from_turns(request, &[user_turn, local_usage_turn], &[], 1_778_347_200_000);

        assert_eq!(report.scope.turn_count, 1);
        assert_eq!(report.scope.from_turn_id.as_deref(), Some("turn-1"));
        assert_eq!(report.scope.to_turn_id.as_deref(), Some("turn-1"));
        assert_eq!(report.time.wall_time_ms, Some(300));
        assert_eq!(report.time.active_turn_ms, Some(300));
        assert_eq!(report.time.model_ms, Some(200));
        assert_eq!(report.models[0].duration_ms, Some(200));
        assert_eq!(report.tools[0].call_count, 1);
        assert_eq!(report.files.files[0].operation_count, 1);
    }

    #[test]
    fn report_merges_legacy_model_timing_into_token_model_row_for_same_turn() {
        let request = test_request(None);
        let mut turn = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        turn.model_rounds[0].model_id = None;
        turn.model_rounds[0].model_alias = None;
        turn.model_rounds[0].duration_ms = Some(180);
        let token_record = test_token_record("gpt-5.4", 120, 30, 0);

        let report = build_session_usage_report_from_turns(request, &[turn], &[token_record], 1_778_347_200_000);

        assert_eq!(
            report
                .models
                .iter()
                .map(|model| (
                    model.model_id.as_str(),
                    model.call_count,
                    model.duration_ms,
                    model.total_tokens
                ))
                .collect::<Vec<_>>(),
            vec![("gpt-5.4", 1, Some(180), Some(150))]
        );
        assert!(report.slowest.iter().any(|span| {
            span.kind == UsageSlowSpanKind::Model && span.label == "gpt-5.4" && span.duration_ms == 180
        }));
    }

    #[test]
    fn report_uses_clear_label_when_model_identity_is_missing() {
        let request = test_request(None);
        let mut turn = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        turn.model_rounds[0].model_id = None;
        turn.model_rounds[0].model_alias = None;
        turn.model_rounds[0].duration_ms = Some(180);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        assert_eq!(report.models[0].model_id, "unknown_model");
        assert!(report.slowest.iter().any(|span| {
            span.kind == UsageSlowSpanKind::Model && span.label == "unknown_model" && span.duration_ms == 180
        }));
    }

    #[test]
    fn report_adds_turn_anchors_to_slowest_spans() {
        let request = test_request(None);
        let mut turn = test_turn_with_tools(
            "turn-7",
            7,
            DialogTurnKind::UserDialog,
            vec![test_tool_item(
                "tool-7",
                "write_file",
                Some(true),
                500,
                "D:/workspace/northhing/src/main.rs",
            )],
        );
        turn.duration_ms = Some(900);
        turn.model_rounds[0].duration_ms = Some(700);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        for kind in [
            UsageSlowSpanKind::Turn,
            UsageSlowSpanKind::Model,
            UsageSlowSpanKind::Tool,
        ] {
            let span = report
                .slowest
                .iter()
                .find(|span| span.kind == kind)
                .expect("anchored slow span");
            assert_eq!(span.turn_id.as_deref(), Some("turn-7"));
            assert_eq!(span.turn_index, Some(7));
        }
    }

    #[test]
    fn report_adds_representative_anchors_to_model_tool_and_error_rows() {
        let request = test_request(None);
        let mut failed_turn = test_turn_with_tools(
            "turn-2",
            2,
            DialogTurnKind::UserDialog,
            vec![test_tool_item(
                "tool-failed",
                "write_file",
                Some(false),
                120,
                "D:/workspace/northhing/src/main.rs",
            )],
        );
        failed_turn.model_rounds[0].model_id = Some("model-a".to_string());
        failed_turn.model_rounds[0].model_alias = Some("model-a".to_string());
        failed_turn.model_rounds[0].duration_ms = Some(220);
        let mut model_error_turn = test_turn_with_tools("turn-4", 4, DialogTurnKind::UserDialog, vec![]);
        model_error_turn.status = TurnStatus::Error;

        let report =
            build_session_usage_report_from_turns(request, &[failed_turn, model_error_turn], &[], 1_778_347_200_000);

        let model = report
            .models
            .iter()
            .find(|model| model.model_id == "model-a")
            .expect("model row");
        assert_eq!(model.sample_turn_id.as_deref(), Some("turn-2"));
        assert_eq!(model.sample_turn_index, Some(2));

        let tool = report
            .tools
            .iter()
            .find(|tool| tool.tool_name == "write_file")
            .expect("tool row");
        assert_eq!(tool.sample_turn_id.as_deref(), Some("turn-2"));
        assert_eq!(tool.sample_turn_index, Some(2));
        assert_eq!(tool.sample_item_id.as_deref(), Some("tool-failed"));

        let tool_error = report
            .errors
            .examples
            .iter()
            .find(|example| example.label == "write_file")
            .expect("tool error example");
        assert_eq!(tool_error.sample_turn_id.as_deref(), Some("turn-2"));
        assert_eq!(tool_error.sample_turn_index, Some(2));
        assert_eq!(tool_error.sample_item_id.as_deref(), Some("tool-failed"));

        let model_error = report
            .errors
            .examples
            .iter()
            .find(|example| example.label == "Model/runtime turn errors")
            .expect("model error example");
        assert_eq!(model_error.sample_turn_id.as_deref(), Some("turn-4"));
        assert_eq!(model_error.sample_turn_index, Some(4));
        assert_eq!(model_error.sample_item_id, None);
    }

    #[test]
    fn report_includes_error_examples_for_failed_turns_and_tools() {
        let request = test_request(None);
        let mut failed_turn = test_turn_with_tools(
            "turn-1",
            0,
            DialogTurnKind::UserDialog,
            vec![
                test_tool_item(
                    "tool-1",
                    "Write",
                    Some(false),
                    100,
                    "D:/workspace/northhing/src/main.rs",
                ),
                test_tool_item("tool-2", "Bash", Some(false), 120, "D:/workspace/northhing"),
            ],
        );
        failed_turn.status = TurnStatus::Error;

        let report = build_session_usage_report_from_turns(request, &[failed_turn], &[], 1_778_347_200_000);

        assert_eq!(report.errors.total_errors, 3);
        assert_eq!(report.errors.tool_errors, 2);
        assert_eq!(report.errors.model_errors, 1);
        assert_eq!(
            report
                .errors
                .examples
                .iter()
                .map(|example| (example.label.as_str(), example.count))
                .collect::<Vec<_>>(),
            vec![("Model/runtime turn errors", 1), ("Bash", 1), ("Write", 1),]
        );
    }
}
