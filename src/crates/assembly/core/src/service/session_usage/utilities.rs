//! R24 session_usage/utilities sibling — extracted from service.rs L1077-L1290.
//!
//! Visibility: `pub(super)` for cross-sibling use within `session_usage` module.
//! Cross-sibling function calls use `super::sibling_name::fn_name(...)`.

use super::entry::SessionUsageReportRequest;
use crate::service::session::{DialogTurnData, ModelRoundData, ToolItemData};
use crate::service::session_usage::redaction::{redact_usage_input_summary, redact_usage_label};
use crate::util::errors::NortHingResult;
use northhing_services_core::session_usage::types::*;
use std::collections::HashSet;
pub fn iter_tools(turns: &[DialogTurnData]) -> impl Iterator<Item = &ToolItemData> {
    turns.iter().flat_map(iter_turn_tools)
}

pub fn iter_turn_tools(turn: &DialogTurnData) -> impl Iterator<Item = &ToolItemData> {
    turn.model_rounds.iter().flat_map(|round| round.tool_items.iter())
}

pub fn model_round_duration_ms(round: &ModelRoundData) -> Option<u64> {
    round
        .duration_ms
        .or_else(|| round.end_time.map(|end| end.saturating_sub(round.start_time)))
}

pub fn model_round_label(round: &ModelRoundData) -> String {
    round
        .model_id
        .as_deref()
        .or(round.model_alias.as_deref())
        .map(|value| redact_usage_label(value, 80).value)
        .unwrap_or_else(|| "unknown_model".to_string())
}

pub fn has_model_timing_fact(round: &ModelRoundData) -> bool {
    model_round_duration_ms(round).is_some()
        || round.first_chunk_ms.is_some()
        || round.first_visible_output_ms.is_some()
        || round.stream_duration_ms.is_some()
        || round.attempt_count.is_some()
        || round.failure_category.is_some()
}

pub fn has_tool_phase_timing_fact(tool: &ToolItemData) -> bool {
    tool.queue_wait_ms.is_some()
        || tool.preflight_ms.is_some()
        || tool.confirmation_wait_ms.is_some()
        || tool.execution_ms.is_some()
}

pub fn tool_duration_ms(tool: &ToolItemData) -> Option<u64> {
    tool.duration_ms
        .or_else(|| tool.tool_result.as_ref().and_then(|result| result.duration_ms))
        .or_else(|| tool.end_time.map(|end| end.saturating_sub(tool.start_time)))
}

pub fn tool_input_summary(tool: &ToolItemData) -> Option<String> {
    let input = tool.tool_call.input.as_object()?;
    let command = input
        .get("command")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(command) = command {
        return Some(redact_usage_input_summary(command, 180).value);
    }

    let url = ["url", "request_url", "endpoint"]
        .into_iter()
        .find_map(|field| input.get(field).and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let method = input
        .get("method")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let summary = method
        .map(|method| format!("{method} {url}"))
        .unwrap_or_else(|| url.to_string());
    Some(redact_usage_input_summary(&summary, 180).value)
}

pub fn tool_timeout_seconds(tool: &ToolItemData) -> Option<u64> {
    let input = tool.tool_call.input.as_object()?;
    input
        .get("timeout_seconds")
        .and_then(|value| value.as_u64())
        .or_else(|| {
            input
                .get("timeout_ms")
                .and_then(|value| value.as_u64())
                .map(|ms| ms.div_ceil(1000))
        })
}

pub fn tool_status_summary(tool: &ToolItemData) -> Option<String> {
    if let Some(success) = tool.tool_result.as_ref().map(|result| result.success) {
        return Some(if success { "succeeded" } else { "failed" }.to_string());
    }

    tool.status.as_deref().map(|status| match status {
        "completed" | "success" | "succeeded" => "succeeded".to_string(),
        "failed" | "error" => "failed".to_string(),
        "cancelled" | "canceled" => "cancelled".to_string(),
        other => redact_usage_label(other, 80).value,
    })
}

pub fn tool_exit_code(tool: &ToolItemData) -> Option<i64> {
    tool.tool_result
        .as_ref()
        .and_then(|result| result.result.get("exit_code"))
        .and_then(|value| value.as_i64())
}

pub fn tool_timed_out(tool: &ToolItemData) -> Option<bool> {
    tool.tool_result
        .as_ref()
        .and_then(|result| result.result.get("timed_out"))
        .and_then(|value| value.as_bool())
}

pub fn tool_error_summary(tool: &ToolItemData) -> Option<String> {
    let error = tool
        .tool_result
        .as_ref()
        .and_then(|result| result.error.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(redact_usage_label(error, 180).value)
}

pub fn add_optional_duration(total: &mut Option<u64>, value: Option<u64>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0) + value);
    }
}

pub fn set_turn_anchor_if_missing(
    sample_turn_id: &mut Option<String>,
    sample_turn_index: &mut Option<usize>,
    turn_id: &str,
    turn_index: Option<usize>,
) {
    if sample_turn_id.is_none() {
        *sample_turn_id = Some(turn_id.to_string());
    }
    if sample_turn_index.is_none() {
        *sample_turn_index = turn_index;
    }
}

pub fn set_item_anchor_if_missing(
    sample_turn_id: &mut Option<String>,
    sample_turn_index: &mut Option<usize>,
    sample_item_id: &mut Option<String>,
    turn_id: &str,
    turn_index: usize,
    item_id: &str,
) {
    set_turn_anchor_if_missing(sample_turn_id, sample_turn_index, turn_id, Some(turn_index));
    if sample_item_id.is_none() {
        *sample_item_id = Some(item_id.to_string());
    }
}

pub fn duration_union_ms(intervals: &[(u64, u64)]) -> u64 {
    let mut normalized = intervals
        .iter()
        .filter_map(|(start, end)| (end > start).then_some((*start, *end)))
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return 0;
    }

    normalized.sort_unstable_by_key(|(start, end)| (*start, *end));
    let mut total = 0;
    let (mut current_start, mut current_end) = normalized[0];

    for (start, end) in normalized.into_iter().skip(1) {
        if start <= current_end {
            current_end = current_end.max(end);
        } else {
            total += current_end.saturating_sub(current_start);
            current_start = start;
            current_end = end;
        }
    }

    total + current_end.saturating_sub(current_start)
}

pub fn is_file_modification_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Write"
            | "Edit"
            | "Delete"
            | "write_file"
            | "edit_file"
            | "create_file"
            | "delete_file"
            | "rename_file"
            | "move_file"
            | "search_replace"
    )
}

pub fn extract_file_path(tool: &ToolItemData) -> Option<String> {
    let input = tool.tool_call.input.as_object()?;
    ["file_path", "path", "filePath", "target_file", "filename"]
        .into_iter()
        .find_map(|key| input.get(key).and_then(|value| value.as_str()))
        .map(ToOwned::to_owned)
}
