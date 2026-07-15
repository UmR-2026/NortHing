//! R24 session_usage/breakdowns_core sibling — extracted from service.rs L294-L712.
//!
//! Visibility: `pub(super)` for cross-sibling use within `session_usage` module.
//! Cross-sibling function calls use `super::sibling_name::fn_name(...)`.

use super::entry::SessionUsageReportRequest;
use crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, ToolItemData, TurnStatus};
use crate::service::session_usage::classifier::classify_tool_usage;
use crate::service::session_usage::redaction::redact_usage_label;
use crate::service::token_usage::TokenUsageRecord;
use crate::util::errors::NortHingResult;
use northhing_services_core::session_usage::types::*;
use std::collections::{BTreeSet, HashMap};
pub fn build_time_breakdown(turns: &[DialogTurnData], generated_at: i64) -> UsageTimeBreakdown {
    if turns.is_empty() {
        return UsageTimeBreakdown {
            accounting: UsageTimeAccounting::Unavailable,
            denominator: UsageTimeDenominator::Unavailable,
            wall_time_ms: None,
            active_turn_ms: None,
            model_ms: None,
            tool_ms: None,
            idle_gap_ms: None,
        };
    }

    // These are persisted lifecycle spans. They intentionally describe recorded
    // session/turn/model-round boundaries, not pure provider streaming
    // throughput such as first-token latency or tokens per second.
    let start = turns.iter().map(|turn| turn.start_time).min().unwrap_or(0);
    let generated_at_ms = u64::try_from(generated_at).ok();
    let end = turns
        .iter()
        .filter_map(|turn| effective_turn_end_time(turn, generated_at_ms))
        .max()
        .unwrap_or(start);
    let wall_time_ms = end.saturating_sub(start);
    let active_intervals = turns
        .iter()
        .filter_map(|turn| {
            effective_turn_end_time(turn, generated_at_ms)
                .filter(|end| *end > turn.start_time)
                .map(|end| (turn.start_time, end))
        })
        .collect::<Vec<_>>();
    let active_turn_ms = (!active_intervals.is_empty())
        .then(|| super::utilities::duration_union_ms(&active_intervals))
        .or_else(|| {
            let summed: u64 = turns.iter().filter_map(|turn| turn.duration_ms).sum();
            (summed > 0).then_some(summed)
        });
    let tool_durations = turns
        .iter()
        .flat_map(|turn| turn.model_rounds.iter())
        .flat_map(|round| round.tool_items.iter())
        .filter_map(super::utilities::tool_duration_ms)
        .collect::<Vec<_>>();
    let tool_ms = Some(tool_durations.iter().sum());
    let model_round_durations: Vec<u64> = turns
        .iter()
        .flat_map(|turn| turn.model_rounds.iter())
        .filter_map(super::utilities::model_round_duration_ms)
        .collect();
    let model_ms = (!model_round_durations.is_empty()).then(|| model_round_durations.iter().sum());
    let has_incomplete_turn_span = turns.iter().any(|turn| turn.end_time.is_none());
    let has_legacy_model_span = turns
        .iter()
        .flat_map(|turn| turn.model_rounds.iter())
        .any(|round| round.duration_ms.is_none() && round.end_time.is_some());

    UsageTimeBreakdown {
        accounting: if has_incomplete_turn_span || has_legacy_model_span {
            UsageTimeAccounting::Approximate
        } else {
            UsageTimeAccounting::Exact
        },
        denominator: if active_turn_ms.is_some() {
            UsageTimeDenominator::ActiveTurnTime
        } else {
            UsageTimeDenominator::SessionWallTime
        },
        wall_time_ms: Some(wall_time_ms),
        active_turn_ms,
        model_ms,
        tool_ms,
        idle_gap_ms: active_turn_ms.map(|active| wall_time_ms.saturating_sub(active)),
    }
}

/// Compute `cache hit rate = cached / input` over records whose provider
/// reported cached tokens. Records without `cached_tokens_available` are
/// excluded from BOTH numerator and denominator — never punish a partially
/// reporting provider by inflating the denominator with un-reported input.
///
/// Returns `None` when no record reports cached tokens, or when the filtered
/// input sum is zero (avoids dividing by zero on edge cases like a tool-only
/// turn). Range: 0.0..=1.0 in normal cases; values >1.0 are theoretically
/// possible on broken providers and left as-is for diagnostic visibility.
fn compute_cache_hit_rate<'a, I>(records: I) -> Option<f64>
where
    I: IntoIterator<Item = &'a TokenUsageRecord>,
{
    let mut cached_sum: u64 = 0;
    let mut input_sum: u64 = 0;
    let mut any_reported = false;
    for record in records {
        if !record.cached_tokens_available {
            continue;
        }
        any_reported = true;
        cached_sum += record.cached_tokens as u64;
        input_sum += record.input_tokens as u64;
    }
    if !any_reported || input_sum == 0 {
        return None;
    }
    Some(cached_sum as f64 / input_sum as f64)
}

pub fn effective_turn_end_time(turn: &DialogTurnData, generated_at_ms: Option<u64>) -> Option<u64> {
    let mut end = span_end_time(turn.start_time, turn.end_time, turn.duration_ms);

    for round in &turn.model_rounds {
        end = max_optional_end(end, span_end_time(round.start_time, round.end_time, round.duration_ms));
        for tool in &round.tool_items {
            end = max_optional_end(
                end,
                span_end_time(tool.start_time, tool.end_time, super::utilities::tool_duration_ms(tool)),
            );
        }
    }

    if end.is_none() && turn.status == TurnStatus::InProgress {
        end = generated_at_ms.filter(|generated_at| *generated_at > turn.start_time);
    }

    end.filter(|end| *end >= turn.start_time)
}

pub fn span_end_time(start_time: u64, end_time: Option<u64>, duration_ms: Option<u64>) -> Option<u64> {
    max_optional_end(
        end_time,
        duration_ms.map(|duration| start_time.saturating_add(duration)),
    )
}

pub fn max_optional_end(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

pub fn build_token_breakdown(token_records: &[TokenUsageRecord]) -> UsageTokenBreakdown {
    if token_records.is_empty() {
        return UsageTokenBreakdown {
            source: UsageTokenSource::Unavailable,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cached_tokens: None,
            cache_coverage: UsageCacheCoverage::Unavailable,
            cache_hit_rate: None,
        };
    }

    UsageTokenBreakdown {
        source: UsageTokenSource::TokenUsageRecords,
        input_tokens: Some(token_records.iter().map(|record| record.input_tokens as u64).sum()),
        output_tokens: Some(token_records.iter().map(|record| record.output_tokens as u64).sum()),
        total_tokens: Some(token_records.iter().map(|record| record.total_tokens as u64).sum()),
        cached_tokens: token_records
            .iter()
            .any(|record| record.cached_tokens_available)
            .then(|| {
                token_records
                    .iter()
                    .filter(|record| record.cached_tokens_available)
                    .map(|record| record.cached_tokens as u64)
                    .sum()
            }),
        cache_coverage: if token_records.iter().all(|record| record.cached_tokens_available) {
            UsageCacheCoverage::Available
        } else if token_records.iter().any(|record| record.cached_tokens_available) {
            UsageCacheCoverage::Partial
        } else {
            UsageCacheCoverage::Unavailable
        },
        cache_hit_rate: compute_cache_hit_rate(token_records.iter()),
    }
}

pub fn build_model_breakdown(turns: &[DialogTurnData], token_records: &[TokenUsageRecord]) -> Vec<UsageModelBreakdown> {
    let mut by_model: HashMap<String, UsageModelBreakdown> = HashMap::new();
    let mut span_counts_by_model: HashMap<String, u64> = HashMap::new();
    let turn_indexes_by_id: HashMap<&str, usize> = turns
        .iter()
        .map(|turn| (turn.turn_id.as_str(), turn.turn_index))
        .collect();
    let token_model_ids_by_turn = build_token_model_ids_by_turn(token_records);
    for record in token_records {
        let row = by_model
            .entry(record.model_id.clone())
            .or_insert_with(|| UsageModelBreakdown {
                model_id: record.model_id.clone(),
                call_count: 0,
                input_tokens: Some(0),
                output_tokens: Some(0),
                total_tokens: Some(0),
                cached_tokens: None,
                // Filled in by P2-2.
                cache_hit_rate: None,
                duration_ms: None,
                sample_turn_id: None,
                sample_turn_index: None,
            });

        row.call_count += 1;
        row.input_tokens = Some(row.input_tokens.unwrap_or(0) + record.input_tokens as u64);
        row.output_tokens = Some(row.output_tokens.unwrap_or(0) + record.output_tokens as u64);
        row.total_tokens = Some(row.total_tokens.unwrap_or(0) + record.total_tokens as u64);
        if record.cached_tokens_available {
            row.cached_tokens = Some(row.cached_tokens.unwrap_or(0) + record.cached_tokens as u64);
        }
        super::utilities::set_turn_anchor_if_missing(
            &mut row.sample_turn_id,
            &mut row.sample_turn_index,
            &record.turn_id,
            turn_indexes_by_id.get(record.turn_id.as_str()).copied(),
        );
    }

    for turn in turns {
        for round in &turn.model_rounds {
            let Some(duration_ms) = super::utilities::model_round_duration_ms(round) else {
                continue;
            };
            let model_id = report_model_id_for_round(round, &token_model_ids_by_turn);
            let row = by_model.entry(model_id.clone()).or_insert_with(|| UsageModelBreakdown {
                model_id: model_id.clone(),
                call_count: 0,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                cached_tokens: None,
                // Filled in by P2-2.
                cache_hit_rate: None,
                duration_ms: Some(0),
                sample_turn_id: None,
                sample_turn_index: None,
            });

            row.duration_ms = Some(row.duration_ms.unwrap_or(0) + duration_ms);
            super::utilities::set_turn_anchor_if_missing(
                &mut row.sample_turn_id,
                &mut row.sample_turn_index,
                &turn.turn_id,
                Some(turn.turn_index),
            );
            *span_counts_by_model.entry(model_id).or_default() += 1;
        }
    }

    for (model_id, span_count) in span_counts_by_model {
        if let Some(row) = by_model.get_mut(&model_id) {
            row.call_count = row.call_count.max(span_count);
        }
    }

    // Per-model hit rate: group records by model_id, then apply the same
    // numerator/denominator policy as the session-level rate.
    let mut records_by_model: HashMap<&str, Vec<&TokenUsageRecord>> = HashMap::new();
    for record in token_records {
        records_by_model
            .entry(record.model_id.as_str())
            .or_default()
            .push(record);
    }
    for (model_id, model_records) in &records_by_model {
        if let Some(row) = by_model.get_mut(*model_id) {
            row.cache_hit_rate = compute_cache_hit_rate(model_records.iter().copied());
        }
    }

    let mut rows: Vec<_> = by_model.into_values().collect();
    rows.sort_by(|a, b| a.model_id.cmp(&b.model_id));
    rows
}

pub fn build_token_model_ids_by_turn(token_records: &[TokenUsageRecord]) -> HashMap<String, BTreeSet<String>> {
    let mut by_turn: HashMap<String, BTreeSet<String>> = HashMap::new();
    for record in token_records {
        by_turn
            .entry(record.turn_id.clone())
            .or_default()
            .insert(record.model_id.clone());
    }
    by_turn
}

pub fn report_model_id_for_round(
    round: &ModelRoundData,
    token_model_ids_by_turn: &HashMap<String, BTreeSet<String>>,
) -> String {
    let label = super::utilities::model_round_label(round);
    if is_legacy_model_identity(&label) {
        if let Some(token_models) = token_model_ids_by_turn.get(&round.turn_id) {
            if token_models.len() == 1 {
                if let Some(model_id) = token_models.iter().next() {
                    return model_id.clone();
                }
            }
        }
    }
    label
}

pub fn is_legacy_model_identity(model_id: &str) -> bool {
    let normalized = model_id.trim().to_ascii_lowercase();
    normalized == "unknown_model"
        || (normalized.starts_with("model round ")
            && normalized["model round ".len()..]
                .chars()
                .all(|value| value.is_ascii_digit()))
}

pub fn build_tool_breakdown(turns: &[DialogTurnData]) -> Vec<UsageToolBreakdown> {
    let mut by_tool: HashMap<String, UsageToolBreakdown> = HashMap::new();
    let mut durations_by_tool: HashMap<String, Vec<u64>> = HashMap::new();

    for turn in turns {
        for tool in super::utilities::iter_turn_tools(turn) {
            let label = redact_usage_label(&tool.tool_name, 80);
            let row = by_tool
                .entry(label.value.clone())
                .or_insert_with(|| UsageToolBreakdown {
                    tool_name: label.value.clone(),
                    category: classify_tool_usage(&tool.tool_name, Some(&tool.tool_call.input)),
                    call_count: 0,
                    success_count: 0,
                    error_count: 0,
                    duration_ms: Some(0),
                    p95_duration_ms: None,
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                    sample_turn_id: None,
                    sample_turn_index: None,
                    sample_item_id: None,
                    redacted: label.redacted,
                });
            row.call_count += 1;
            match tool.tool_result.as_ref().map(|result| result.success) {
                Some(true) => row.success_count += 1,
                Some(false) => row.error_count += 1,
                None => {}
            }
            let duration_ms = super::utilities::tool_duration_ms(tool).unwrap_or(0);
            row.duration_ms = Some(row.duration_ms.unwrap_or(0) + duration_ms);
            if duration_ms > 0 {
                durations_by_tool
                    .entry(label.value.clone())
                    .or_default()
                    .push(duration_ms);
            }
            super::utilities::add_optional_duration(&mut row.queue_wait_ms, tool.queue_wait_ms);
            super::utilities::add_optional_duration(&mut row.preflight_ms, tool.preflight_ms);
            super::utilities::add_optional_duration(&mut row.confirmation_wait_ms, tool.confirmation_wait_ms);
            super::utilities::add_optional_duration(&mut row.execution_ms, tool.execution_ms);
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

    let mut rows: Vec<_> = by_tool
        .into_values()
        .map(|mut row| {
            row.p95_duration_ms = durations_by_tool
                .get(&row.tool_name)
                .and_then(|durations| super::breakdowns_extra::p95_duration_ms(durations));
            row
        })
        .collect();
    rows.sort_by(|a, b| {
        b.call_count
            .cmp(&a.call_count)
            .then_with(|| a.tool_name.cmp(&b.tool_name))
    });
    rows
}
