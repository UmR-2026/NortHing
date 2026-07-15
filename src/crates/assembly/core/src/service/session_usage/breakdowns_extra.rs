//! R24 session_usage/breakdowns_extra sibling — extracted from service.rs L713-L1050.
//!
//! Visibility: `pub(super)` for cross-sibling use within `session_usage` module.
//! Cross-sibling function calls use `super::sibling_name::fn_name(...)`.

use super::entry::SessionUsageReportRequest;
use crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, ToolItemData, TurnStatus};
use crate::service::session_usage::redaction::{
    display_workspace_relative_path, redact_usage_input_summary, redact_usage_label,
};
use crate::service::token_usage::TokenUsageRecord;
use crate::util::errors::NortHingResult;
use northhing_services_core::session_usage::types::*;
use std::collections::{BTreeSet, HashMap, HashSet};
pub fn p95_duration_ms(durations: &[u64]) -> Option<u64> {
    if durations.len() < 2 {
        return None;
    }

    let mut sorted = durations.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64) * 0.95).ceil() as usize;
    sorted.get(index.saturating_sub(1)).copied()
}

pub fn build_file_breakdown(
    workspace_root: Option<&str>,
    turns: &[DialogTurnData],
    snapshot_facts: &UsageSnapshotFacts,
) -> UsageFileBreakdown {
    if snapshot_facts.source_available {
        return build_file_breakdown_from_snapshot_operations(workspace_root, &snapshot_facts.operations);
    }

    build_file_breakdown_from_tool_inputs(workspace_root, turns)
}

pub fn build_file_breakdown_from_snapshot_operations(
    workspace_root: Option<&str>,
    operations: &[UsageSnapshotOperationSummary],
) -> UsageFileBreakdown {
    let mut files: HashMap<String, UsageFileRow> = HashMap::new();
    let mut turn_indexes_by_path: HashMap<String, BTreeSet<usize>> = HashMap::new();
    let mut operation_ids_by_path: HashMap<String, BTreeSet<String>> = HashMap::new();

    for operation in operations {
        let label = display_workspace_relative_path(workspace_root, &operation.file_path);
        let row = files.entry(label.value.clone()).or_insert_with(|| UsageFileRow {
            path_label: label.value.clone(),
            operation_count: 0,
            added_lines: Some(0),
            deleted_lines: Some(0),
            session_id: Some(operation.session_id.clone()),
            turn_indexes: vec![],
            operation_ids: vec![],
            redacted: label.redacted,
        });
        row.operation_count += 1;
        row.added_lines = Some(row.added_lines.unwrap_or(0) + operation.lines_added);
        row.deleted_lines = Some(row.deleted_lines.unwrap_or(0) + operation.lines_removed);
        row.session_id.get_or_insert_with(|| operation.session_id.clone());
        row.redacted |= label.redacted;

        turn_indexes_by_path
            .entry(label.value.clone())
            .or_default()
            .insert(operation.turn_index);
        operation_ids_by_path
            .entry(label.value)
            .or_default()
            .insert(operation.operation_id.clone());
    }

    let mut rows: Vec<_> = files
        .into_iter()
        .map(|(path_label, mut row)| {
            row.turn_indexes = turn_indexes_by_path
                .remove(&path_label)
                .map(|values| values.into_iter().collect())
                .unwrap_or_default();
            row.operation_ids = operation_ids_by_path
                .remove(&path_label)
                .map(|values| values.into_iter().collect())
                .unwrap_or_default();
            row
        })
        .collect();
    rows.sort_by(|a, b| a.path_label.cmp(&b.path_label));

    UsageFileBreakdown {
        scope: UsageFileScope::SnapshotSummary,
        changed_files: Some(rows.len() as u64),
        added_lines: Some(rows.iter().map(|row| row.added_lines.unwrap_or(0)).sum()),
        deleted_lines: Some(rows.iter().map(|row| row.deleted_lines.unwrap_or(0)).sum()),
        files: rows,
    }
}

pub fn build_file_breakdown_from_tool_inputs(
    workspace_root: Option<&str>,
    turns: &[DialogTurnData],
) -> UsageFileBreakdown {
    let mut files: HashMap<String, UsageFileRow> = HashMap::new();
    let mut turn_indexes_by_path: HashMap<String, BTreeSet<usize>> = HashMap::new();
    let mut operation_ids_by_path: HashMap<String, BTreeSet<String>> = HashMap::new();

    for turn in turns {
        for tool in super::utilities::iter_turn_tools(turn) {
            if !super::utilities::is_file_modification_tool(&tool.tool_name) {
                continue;
            }

            let Some(path) = super::utilities::extract_file_path(tool) else {
                continue;
            };
            let label = display_workspace_relative_path(workspace_root, &path);
            let row = files.entry(label.value.clone()).or_insert_with(|| UsageFileRow {
                path_label: label.value.clone(),
                operation_count: 0,
                added_lines: None,
                deleted_lines: None,
                session_id: None,
                turn_indexes: vec![],
                operation_ids: vec![],
                redacted: label.redacted,
            });
            row.operation_count += 1;
            row.redacted |= label.redacted;

            turn_indexes_by_path
                .entry(label.value.clone())
                .or_default()
                .insert(turn.turn_index);
            operation_ids_by_path
                .entry(label.value)
                .or_default()
                .insert(tool.id.clone());
        }
    }

    let mut rows: Vec<_> = files
        .into_iter()
        .map(|(path_label, mut row)| {
            row.turn_indexes = turn_indexes_by_path
                .remove(&path_label)
                .map(|values| values.into_iter().collect())
                .unwrap_or_default();
            row.operation_ids = operation_ids_by_path
                .remove(&path_label)
                .map(|values| values.into_iter().collect())
                .unwrap_or_default();
            row
        })
        .collect();
    rows.sort_by(|a, b| a.path_label.cmp(&b.path_label));
    UsageFileBreakdown {
        scope: if rows.is_empty() {
            UsageFileScope::Unavailable
        } else {
            UsageFileScope::ToolInputsOnly
        },
        changed_files: if rows.is_empty() { None } else { Some(rows.len() as u64) },
        added_lines: None,
        deleted_lines: None,
        files: rows,
    }
}

pub fn build_compression_breakdown(turns: &[DialogTurnData]) -> UsageCompressionBreakdown {
    let manual_compaction_count = turns
        .iter()
        .filter(|turn| turn.kind == DialogTurnKind::ManualCompaction)
        .count() as u64;
    let automatic_compaction_count = super::utilities::iter_tools(turns)
        .filter(|tool| tool.tool_name.to_lowercase().contains("compaction"))
        .count() as u64;

    UsageCompressionBreakdown {
        compaction_count: manual_compaction_count + automatic_compaction_count,
        manual_compaction_count,
        automatic_compaction_count,
        saved_tokens: None,
    }
}

pub fn build_error_breakdown(turns: &[DialogTurnData]) -> UsageErrorBreakdown {
    let model_errors = turns.iter().filter(|turn| turn.status == TurnStatus::Error).count() as u64;
    let tool_errors = super::utilities::iter_tools(turns)
        .filter(|tool| tool.tool_result.as_ref().is_some_and(|result| !result.success))
        .count() as u64;
    let mut examples = Vec::new();

    if model_errors > 0 {
        let sample_model_error_turn = turns.iter().find(|turn| turn.status == TurnStatus::Error);
        examples.push(UsageErrorExample {
            label: "Model/runtime turn errors".to_string(),
            count: model_errors,
            sample_turn_id: sample_model_error_turn.map(|turn| turn.turn_id.clone()),
            sample_turn_index: sample_model_error_turn.map(|turn| turn.turn_index),
            sample_item_id: None,
            redacted: false,
        });
    }

    let mut tool_error_counts: HashMap<String, UsageErrorExample> = HashMap::new();
    for turn in turns {
        for tool in super::utilities::iter_turn_tools(turn)
            .filter(|tool| tool.tool_result.as_ref().is_some_and(|result| !result.success))
        {
            let label = redact_usage_label(&tool.tool_name, 80);
            let row = tool_error_counts
                .entry(label.value.clone())
                .or_insert_with(|| UsageErrorExample {
                    label: label.value.clone(),
                    count: 0,
                    sample_turn_id: None,
                    sample_turn_index: None,
                    sample_item_id: None,
                    redacted: label.redacted,
                });
            row.count += 1;
            super::utilities::set_item_anchor_if_missing(
                &mut row.sample_turn_id,
                &mut row.sample_turn_index,
                &mut row.sample_item_id,
                &turn.turn_id,
                turn.turn_index,
                &tool.id,
            );
            row.redacted |= label.redacted;
        }
    }

    let mut tool_examples: Vec<_> = tool_error_counts.into_values().collect();
    tool_examples.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
    examples.extend(tool_examples.into_iter().take(4));

    UsageErrorBreakdown {
        total_errors: model_errors + tool_errors,
        tool_errors,
        model_errors,
        examples,
    }
}

pub fn build_slowest_spans(turns: &[DialogTurnData], token_records: &[TokenUsageRecord]) -> Vec<UsageSlowSpan> {
    let mut spans = Vec::new();
    let token_model_ids_by_turn = super::breakdowns_core::build_token_model_ids_by_turn(token_records);

    for turn in turns {
        if let Some(duration_ms) =
            super::breakdowns_core::effective_turn_end_time(turn, None).map(|end| end.saturating_sub(turn.start_time))
        {
            spans.push(UsageSlowSpan {
                label: format!("turn {}", turn.turn_index),
                kind: UsageSlowSpanKind::Turn,
                duration_ms,
                redacted: false,
                turn_id: Some(turn.turn_id.clone()),
                turn_index: Some(turn.turn_index),
                item_id: None,
                input_summary: None,
                status: None,
                timeout_seconds: None,
                exit_code: None,
                timed_out: None,
                error_summary: None,
                queue_wait_ms: None,
                preflight_ms: None,
                confirmation_wait_ms: None,
                execution_ms: None,
            });
        }

        for round in &turn.model_rounds {
            if let Some(duration_ms) = super::utilities::model_round_duration_ms(round) {
                spans.push(UsageSlowSpan {
                    label: super::breakdowns_core::report_model_id_for_round(round, &token_model_ids_by_turn),
                    kind: UsageSlowSpanKind::Model,
                    duration_ms,
                    redacted: false,
                    turn_id: Some(turn.turn_id.clone()),
                    turn_index: Some(turn.turn_index),
                    item_id: None,
                    input_summary: None,
                    status: None,
                    timeout_seconds: None,
                    exit_code: None,
                    timed_out: None,
                    error_summary: None,
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                });
            }
        }

        for tool in super::utilities::iter_turn_tools(turn) {
            let label = redact_usage_label(&tool.tool_name, 80);
            if let Some(duration_ms) = super::utilities::tool_duration_ms(tool) {
                spans.push(UsageSlowSpan {
                    label: label.value,
                    kind: UsageSlowSpanKind::Tool,
                    duration_ms,
                    redacted: label.redacted,
                    turn_id: Some(turn.turn_id.clone()),
                    turn_index: Some(turn.turn_index),
                    item_id: Some(tool.id.clone()),
                    input_summary: super::utilities::tool_input_summary(tool),
                    status: super::utilities::tool_status_summary(tool),
                    timeout_seconds: super::utilities::tool_timeout_seconds(tool),
                    exit_code: super::utilities::tool_exit_code(tool),
                    timed_out: super::utilities::tool_timed_out(tool),
                    error_summary: super::utilities::tool_error_summary(tool),
                    queue_wait_ms: tool.queue_wait_ms,
                    preflight_ms: tool.preflight_ms,
                    confirmation_wait_ms: tool.confirmation_wait_ms,
                    execution_ms: tool.execution_ms,
                });
            }
        }
    }

    spans.sort_by_key(|b| std::cmp::Reverse(b.duration_ms));
    spans.truncate(5);
    spans
}

pub fn collect_redacted_fields(report: &SessionUsageReport) -> Vec<String> {
    let mut fields = HashSet::new();
    if report.tools.iter().any(|tool| tool.redacted) {
        fields.insert("tools.toolName".to_string());
    }
    if report.files.files.iter().any(|file| file.redacted) {
        fields.insert("files.pathLabel".to_string());
    }
    if report.slowest.iter().any(|span| span.redacted) {
        fields.insert("slowest.label".to_string());
    }
    if report
        .slowest
        .iter()
        .filter_map(|span| span.input_summary.as_deref())
        .any(|summary| summary.contains("[redacted]"))
    {
        fields.insert("slowest.inputSummary".to_string());
    }

    let mut fields: Vec<_> = fields.into_iter().collect();
    fields.sort();
    fields
}
