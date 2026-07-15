use crate::agentic::core::{Message, MessageContent, MessageRole, ToolCall, ToolResult};
use crate::agentic::insights::types::SessionTranscript;
use crate::agentic::persistence::PersistenceManager;
use crate::service::session::{DialogTurnData, TurnStatus};
use crate::util::errors::NortHingResult;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::coll_stats::compute_active_duration;

const MAX_TRANSCRIPT_CHARS: usize = 16000;
const MAX_TEXT_PER_MESSAGE: usize = 800;
const TAIL_RESERVE_CHARS: usize = 4000;

/// Load messages for a session, trying sources in priority order:
/// 1. Latest context snapshot (most complete, includes compression)
/// 2. Rebuild from pre-loaded turn data
pub(super) async fn load_session_messages_with_turns(
    pm: &PersistenceManager,
    workspace_path: &Path,
    session_id: &str,
    turns: &[DialogTurnData],
) -> NortHingResult<Vec<Message>> {
    if let Ok(Some((_turn_index, messages))) = pm.load_latest_turn_context_snapshot(workspace_path, session_id).await {
        if !messages.is_empty() {
            return Ok(messages);
        }
    }

    if !turns.is_empty() {
        return Ok(rebuild_messages_from_turns(turns));
    }

    Ok(vec![])
}

pub(super) fn build_transcript(
    session_id: &str,
    session: &crate::agentic::core::Session,
    messages: &[Message],
) -> SessionTranscript {
    let mut all_parts: Vec<String> = Vec::new();
    let mut tool_names: Vec<String> = Vec::new();
    let mut has_errors = false;

    for msg in messages {
        match &msg.content {
            MessageContent::Text(text) => {
                let role_tag = match msg.role {
                    MessageRole::User => "[User]",
                    MessageRole::Assistant => "[Assistant]",
                    MessageRole::System => continue,
                    MessageRole::Tool => continue,
                };
                let truncated = truncate_text(text, MAX_TEXT_PER_MESSAGE);
                all_parts.push(format!("{}: {}", role_tag, truncated));
            }
            MessageContent::Mixed { text, tool_calls, .. } => {
                if !text.is_empty() {
                    let truncated = truncate_text(text, MAX_TEXT_PER_MESSAGE);
                    all_parts.push(format!("[Assistant]: {}", truncated));
                }
                for tc in tool_calls {
                    if !tool_names.contains(&tc.tool_name) {
                        tool_names.push(tc.tool_name.clone());
                    }
                    all_parts.push(format!("[Tool: {}]", tc.tool_name));
                }
            }
            MessageContent::ToolResult {
                tool_name, is_error, ..
            } => {
                if *is_error {
                    has_errors = true;
                    all_parts.push(format!("[Tool Error: {}]", tool_name));
                }
            }
            MessageContent::Multimodal { text, .. } => {
                if !text.is_empty() {
                    let truncated = truncate_text(text, MAX_TEXT_PER_MESSAGE);
                    all_parts.push(format!("[User]: {} [+images]", truncated));
                }
            }
        }
    }

    let transcript = smart_truncate_parts(&all_parts, MAX_TRANSCRIPT_CHARS, TAIL_RESERVE_CHARS);

    let duration_minutes = compute_active_duration(messages) / 60;

    let created_at = system_time_to_iso(session.created_at);

    SessionTranscript {
        session_id: session_id.to_string(),
        agent_type: session.agent_type.clone(),
        session_name: session.session_name.clone(),
        workspace_path: None,
        last_activity_unix_secs: 0,
        duration_minutes,
        message_count: messages.len() as u32,
        turn_count: session.dialog_turn_ids.len() as u32,
        created_at,
        transcript,
        tool_names,
        has_errors,
    }
}

/// Rebuild `Vec<Message>` from turn data, including call and result information
/// needed by `build_transcript` and `accumulate_stats`.
/// Preserves timestamps from turn data and marks cancelled turns with `[Cancelled]`.
pub(super) fn rebuild_messages_from_turns(turns: &[DialogTurnData]) -> Vec<Message> {
    let mut messages = Vec::new();

    for turn in turns {
        if !turn.kind.is_model_visible() {
            continue;
        }

        let user_ts = UNIX_EPOCH + Duration::from_millis(turn.start_time);
        let mut user_msg = Message::user(turn.user_message.content.clone());
        user_msg.timestamp = user_ts;
        messages.push(user_msg);

        for (round_idx, round) in turn.model_rounds.iter().enumerate() {
            let assistant_text = round
                .text_items
                .iter()
                .map(|item| item.content.clone())
                .filter(|c| !c.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            let tool_calls: Vec<ToolCall> = round
                .tool_items
                .iter()
                .map(|ti| ToolCall {
                    tool_id: ti.tool_call.id.clone(),
                    tool_name: ti.tool_name.clone(),
                    arguments: ti.tool_call.input.clone(),
                    raw_arguments: None,
                    is_error: false,
                    recovered_from_truncation: false,
                })
                .collect();

            let round_ts = if let Some(end_time) = turn.end_time {
                let start = turn.start_time;
                let total_rounds = turn.model_rounds.len().max(1) as u64;
                let step = (end_time.saturating_sub(start)) / (total_rounds + 1);
                UNIX_EPOCH + Duration::from_millis(start + step * (round_idx as u64 + 1))
            } else {
                UNIX_EPOCH + Duration::from_millis(turn.start_time + (round_idx as u64 + 1) * 1000)
            };

            if !tool_calls.is_empty() {
                let mut msg = Message::assistant_with_tools(assistant_text.clone(), tool_calls);
                msg.timestamp = round_ts;
                messages.push(msg);
            } else if !assistant_text.trim().is_empty() {
                let mut msg = Message::assistant(assistant_text);
                msg.timestamp = round_ts;
                messages.push(msg);
            }

            for ti in &round.tool_items {
                if let Some(result_data) = &ti.tool_result {
                    let mut msg = Message::tool_result(ToolResult {
                        tool_id: ti.tool_call.id.clone(),
                        tool_name: ti.tool_name.clone(),
                        result: result_data.result.clone(),
                        result_for_assistant: None,
                        is_error: !result_data.success,
                        duration_ms: result_data.duration_ms,
                        image_attachments: None,
                    });
                    msg.timestamp = round_ts;
                    messages.push(msg);
                }
            }
        }

        if turn.status == TurnStatus::Cancelled {
            let cancel_ts = turn
                .end_time
                .map(|t| UNIX_EPOCH + Duration::from_millis(t))
                .unwrap_or(user_ts);
            let mut cancel_msg = Message::assistant("[Cancelled by user]".to_string());
            cancel_msg.timestamp = cancel_ts;
            messages.push(cancel_msg);
        }
    }

    messages
}

/// Keep head + tail of transcript parts, inserting an omission marker in the middle
/// when total length exceeds `max_chars`. Preserves the beginning (context/goals)
/// and end (final outcome) of a session.
pub(super) fn smart_truncate_parts(parts: &[String], max_chars: usize, tail_reserve: usize) -> String {
    let total: usize = parts.iter().map(|p| p.len() + 1).sum();
    if total <= max_chars {
        return parts.join("\n");
    }

    let head_budget = max_chars.saturating_sub(tail_reserve);
    let mut head_parts = Vec::new();
    let mut head_used = 0;
    let mut head_end_idx = 0;

    for (i, part) in parts.iter().enumerate() {
        let cost = part.len() + 1;
        if head_used + cost > head_budget {
            break;
        }
        head_parts.push(part.as_str());
        head_used += cost;
        head_end_idx = i + 1;
    }

    let mut tail_parts = Vec::new();
    let mut tail_used = 0;
    let mut tail_start_idx = parts.len();

    for (i, part) in parts.iter().enumerate().rev() {
        if i < head_end_idx {
            break;
        }
        let cost = part.len() + 1;
        if tail_used + cost > tail_reserve {
            break;
        }
        tail_parts.push(part.as_str());
        tail_used += cost;
        tail_start_idx = i;
    }
    tail_parts.reverse();

    let omitted = tail_start_idx.saturating_sub(head_end_idx);

    let mut result = head_parts.join("\n");
    if omitted > 0 {
        result.push_str(&format!("\n\n[... {} messages omitted ...]\n\n", omitted));
    }
    result.push_str(&tail_parts.join("\n"));
    result
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        let mut end = max_len.min(trimmed.len());
        while end > 0 && !trimmed.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &trimmed[..end])
    }
}

fn system_time_to_iso(t: SystemTime) -> String {
    match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => {
            let dt = DateTime::<Utc>::from(UNIX_EPOCH + dur);
            dt.to_rfc3339()
        }
        Err(_) => "unknown".to_string(),
    }
}
