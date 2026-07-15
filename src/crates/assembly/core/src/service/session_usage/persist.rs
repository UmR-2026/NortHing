#[cfg(test)]
mod tests {
    use super::super::breakdowns_extra::*;
    use super::super::service::test_helpers::*;
    use super::super::service::{build_session_usage_report_from_sources, build_session_usage_report_from_turns};
    use super::super::snapshot::*;
    use super::super::SessionUsageReport;
    use super::super::SessionUsageReportRequest;
    use super::super::UsageCoverageKey;
    use super::super::UsageFileBreakdown;
    use super::super::UsageFileRow;
    use super::super::UsageFileScope;
    use super::super::UsageSlowSpanKind;
    use super::super::UsageSnapshotFacts;
    use super::super::UsageWorkspaceKind;
    use crate::service::session::DialogTurnKind;
    use crate::service::session_usage::redaction::{redact_usage_input_summary, redact_usage_label};

    #[test]
    fn report_marks_remote_snapshot_stats_partial() {
        let request = test_request(Some("ssh-1"));

        let report = build_session_usage_report_from_turns(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &[],
            1_778_347_200_000,
        );

        assert_eq!(report.workspace.kind, UsageWorkspaceKind::RemoteSsh);
        assert!(report.coverage.missing.contains(&UsageCoverageKey::RemoteSnapshotStats));
    }

    #[test]
    fn report_uses_persisted_model_span_facts_without_token_records() {
        let request = test_request(None);
        let mut turn = test_turn("turn-1", 0, DialogTurnKind::UserDialog);
        turn.model_rounds = vec![
            test_model_round("round-a", "turn-1", 0, "model-a", 90),
            test_model_round("round-b", "turn-1", 1, "model-b", 140),
        ];

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);

        assert!(report.coverage.available.contains(&UsageCoverageKey::ModelRoundTiming));
        assert!(!report.coverage.missing.contains(&UsageCoverageKey::ModelRoundTiming));
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
            vec![("model-a", 1, Some(90), None), ("model-b", 1, Some(140), None),]
        );
        assert!(report.slowest.iter().any(|span| {
            span.kind == UsageSlowSpanKind::Model && span.label == "model-b" && span.duration_ms == 140
        }));
    }

    #[test]
    fn aggregates_operation_summary_file_stats_without_reading_file_bodies() {
        let request = test_request(None);
        let snapshot_facts = test_snapshot_facts(vec![
            test_snapshot_operation("op-1", 0, "D:/workspace/northhing/src/main.rs", 10, 2),
            test_snapshot_operation("op-2", 1, "D:/workspace/northhing/src/main.rs", 5, 1),
            test_snapshot_operation("op-3", 1, "D:/workspace/northhing/src/lib.rs", 4, 0),
        ]);

        let report = build_session_usage_report_from_sources(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &[],
            &snapshot_facts,
            1_778_347_200_000,
        );

        assert_eq!(report.files.scope, UsageFileScope::SnapshotSummary);
        assert_eq!(report.files.changed_files, Some(2));
        assert_eq!(report.files.added_lines, Some(19));
        assert_eq!(report.files.deleted_lines, Some(3));
        assert!(report.coverage.available.contains(&UsageCoverageKey::FileLineStats));
        assert!(!report.coverage.missing.contains(&UsageCoverageKey::FileLineStats));

        let main_row = report
            .files
            .files
            .iter()
            .find(|row| row.path_label == "src/main.rs")
            .expect("main.rs row");
        assert_eq!(main_row.operation_count, 2);
        assert_eq!(main_row.added_lines, Some(15));
        assert_eq!(main_row.deleted_lines, Some(3));
    }

    #[test]
    fn remote_workspace_without_snapshot_marks_file_stats_partial() {
        let request = test_request(Some("ssh-1"));

        let report = build_session_usage_report_from_sources(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &[],
            &UsageSnapshotFacts::default(),
            1_778_347_200_000,
        );

        assert_eq!(report.workspace.kind, UsageWorkspaceKind::RemoteSsh);
        assert_eq!(report.files.scope, UsageFileScope::ToolInputsOnly);
        assert_eq!(report.files.changed_files, Some(1));
        assert_eq!(report.files.added_lines, None);
        assert!(report.coverage.missing.contains(&UsageCoverageKey::FileLineStats));
        assert!(report.coverage.missing.contains(&UsageCoverageKey::RemoteSnapshotStats));
    }

    #[test]
    fn remote_workspace_uses_wrapped_tool_inputs_for_file_rows() {
        let request = test_request(Some("ssh-1"));
        let turn = test_turn_with_tools(
            "turn-1",
            0,
            DialogTurnKind::UserDialog,
            vec![
                test_tool_item_with_input(
                    "tool-1",
                    "Write",
                    Some(true),
                    100,
                    serde_json::json!({ "file_path": "D:/workspace/northhing/src/main.rs" }),
                ),
                test_tool_item_with_input(
                    "tool-2",
                    "Edit",
                    Some(true),
                    80,
                    serde_json::json!({ "target_file": "D:/workspace/northhing/src/lib.rs" }),
                ),
            ],
        );

        let report = build_session_usage_report_from_sources(
            request,
            &[turn],
            &[],
            &UsageSnapshotFacts::default(),
            1_778_347_200_000,
        );

        assert_eq!(report.workspace.kind, UsageWorkspaceKind::RemoteSsh);
        assert_eq!(report.files.scope, UsageFileScope::ToolInputsOnly);
        assert_eq!(report.files.changed_files, Some(2));
        assert_eq!(
            report
                .files
                .files
                .iter()
                .map(|row| row.path_label.as_str())
                .collect::<Vec<_>>(),
            vec!["src/lib.rs", "src/main.rs"]
        );
    }

    #[test]
    fn file_rows_preserve_operation_turn_and_session_scopes() {
        let request = test_request(None);
        let snapshot_facts = test_snapshot_facts(vec![
            test_snapshot_operation("op-9", 2, "D:/workspace/northhing/src/main.rs", 1, 0),
            test_snapshot_operation("op-1", 0, "D:/workspace/northhing/src/main.rs", 2, 1),
        ]);

        let report = build_session_usage_report_from_sources(
            request,
            &[test_turn("turn-1", 0, DialogTurnKind::UserDialog)],
            &[],
            &snapshot_facts,
            1_778_347_200_000,
        );

        let row = report
            .files
            .files
            .iter()
            .find(|row| row.path_label == "src/main.rs")
            .expect("main.rs row");

        assert_eq!(row.session_id.as_deref(), Some("session-1"));
        assert_eq!(row.turn_indexes, vec![0, 2]);
        assert_eq!(row.operation_ids, vec!["op-1", "op-9"]);
    }
}
