//! Distill a DialogTurnData into an Episode for the growth log.

use crate::agentic::episodes::types::{Episode, EpisodeOutcome, ToolFailureRecord, ToolUseRecord};
use crate::service::session::DialogTurnData;

/// Distill a persisted DialogTurnData into an Episode record.
pub fn distill_episode(
    turn: &DialogTurnData,
    task_summary: String,
    workspace_slug: String,
    agent_type: String,
    outcome: EpisodeOutcome,
) -> Episode {
    let tools_used = extract_tool_records(turn);
    let failures = extract_failures(turn);

    let duration_ms = turn.duration_ms;

    let ts = turn.timestamp;

    Episode {
        schema_version: 1,
        turn_id: turn.turn_id.clone(),
        session_id: turn.session_id.clone(),
        workspace_slug,
        agent_type,
        task_summary,
        tools_used,
        failures,
        outcome,
        duration_ms,
        ts,
        redline_verdicts: vec![],
    }
}

/// Extract tool use records from a dialog turn.
fn extract_tool_records(turn: &DialogTurnData) -> Vec<ToolUseRecord> {
    let mut records = Vec::new();
    for round in &turn.model_rounds {
        for tool_item in &round.tool_items {
            let ok = tool_item
                .tool_result
                .as_ref()
                .map(|r| r.success)
                .unwrap_or(false);
            records.push(ToolUseRecord {
                name: tool_item.tool_name.clone(),
                ok,
            });
        }
    }
    records
}

/// Extract tool failure records with repair tracking.
/// Uses a global index across all rounds to correctly identify later calls.
fn extract_failures(turn: &DialogTurnData) -> Vec<ToolFailureRecord> {
    let mut failures = Vec::new();

    // Collect all tool calls in order (across all rounds) with a GLOBAL index
    let mut global_idx = 0usize;
    let all_tools: Vec<(usize, String, bool, Option<String>, Option<String>)> = turn
        .model_rounds
        .iter()
        .flat_map(|round| {
            let results: Vec<_> = round
                .tool_items
                .iter()
                .map(|item| {
                    let idx = global_idx;
                    global_idx += 1;
                    let ok = item
                        .tool_result
                        .as_ref()
                        .map(|r| r.success)
                        .unwrap_or(false);
                    let error_first_line = item
                        .tool_result
                        .as_ref()
                        .and_then(|r| r.error.as_ref())
                        .map(|e| first_line_truncated(e, 200));
                    let repair_content = extract_repair_content(item);
                    (idx, item.tool_name.clone(), ok, error_first_line, repair_content)
                })
                .collect();
            results
        })
        .collect();

    // Find failed tools and track subsequent successes for the same tool name
    for &(idx, ref tool_name, ok, ref error_opt, _) in &all_tools {
        if !ok {
            let error_msg = error_opt.clone().unwrap_or_else(|| "unknown error".to_string());
            // Check if there's a later successful call to the same tool
            let repair = find_later_success(&all_tools, idx, tool_name);
            failures.push(ToolFailureRecord {
                tool: tool_name.clone(),
                error: error_msg,
                repair,
            });
        }
    }

    failures
}

/// Extract repair content from a successful tool item:
/// result_for_assistant if available, else first line of tool input (max 200 chars).
fn extract_repair_content(item: &crate::service::session::ToolItemData) -> Option<String> {
    // Try result_for_assistant first
    if let Some(ref result) = item.tool_result {
        if let Some(ref rfa) = result.result_for_assistant {
            let trimmed = rfa.trim();
            if !trimmed.is_empty() {
                return Some(first_line_truncated(trimmed, 200));
            }
        }
    }
    // Fall back to first line of input
    let input_str = item.tool_call.input.to_string();
    let first_line = input_str.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        None
    } else {
        Some(first_line_truncated(first_line, 200))
    }
}

/// Find the first successful call to the same tool after the given index.
/// Returns the repair content from that successful call.
fn find_later_success(
    all_tools: &[(usize, String, bool, Option<String>, Option<String>)],
    after_idx: usize,
    tool_name: &str,
) -> Option<String> {
    for &(idx, ref name, ok, _, ref repair_content) in all_tools {
        if idx > after_idx && name == tool_name && ok {
            return repair_content.clone();
        }
    }
    None
}

/// Truncate a string to at most `max_chars` characters, taking only the first line.
fn first_line_truncated(s: &str, max_chars: usize) -> String {
    let first_line = s.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= max_chars {
        first_line.to_string()
    } else {
        first_line.chars().take(max_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::session::{
        DialogTurnData, DialogTurnKind, ModelRoundData, ToolCallData, ToolItemData,
        ToolResultData, TurnStatus, UserMessageData,
    };

    fn make_test_turn(turn_id: &str) -> DialogTurnData {
        DialogTurnData {
            turn_id: turn_id.to_string(),
            turn_index: 0,
            session_id: "session-1".to_string(),
            timestamp: 1000,
            kind: DialogTurnKind::UserDialog,
            agent_type: Some("agentic".to_string()),
            user_message: UserMessageData {
                id: "msg-1".to_string(),
                content: "test input".to_string(),
                timestamp: 1000,
                metadata: None,
            },
            model_rounds: vec![],
            start_time: 1000,
            end_time: Some(2000),
            duration_ms: Some(1000),
            token_usage: None,
            status: TurnStatus::Completed,
        }
    }

    fn make_tool_item_with_result(
        name: &str,
        success: bool,
        error_msg: Option<&str>,
        result_for_assistant: Option<&str>,
    ) -> ToolItemData {
        ToolItemData {
            id: format!("tool-{}", name),
            tool_name: name.to_string(),
            tool_call: ToolCallData {
                input: serde_json::json!({"cmd": name}),
                id: format!("call-{}", name),
            },
            tool_result: Some(ToolResultData {
                result: serde_json::json!({}),
                success,
                result_for_assistant: result_for_assistant.map(|s| s.to_string()),
                error: error_msg.map(|e| e.to_string()),
                duration_ms: None,
            }),
            ai_intent: None,
            start_time: 1000,
            end_time: Some(1100),
            duration_ms: Some(100),
            queue_wait_ms: None,
            preflight_ms: None,
            confirmation_wait_ms: None,
            execution_ms: None,
            order_index: None,
            is_subagent_item: None,
            parent_task_tool_id: None,
            subagent_session_id: None,
            subagent_model_id: None,
            subagent_model_alias: None,
            status: None,
            interruption_reason: None,
        }
    }

    #[test]
    fn distill_with_no_tools() {
        let turn = make_test_turn("turn-1");
        let episode = distill_episode(
            &turn,
            "test task".to_string(),
            "workspace-abc".to_string(),
            "agentic".to_string(),
            EpisodeOutcome::Completed,
        );
        assert_eq!(episode.turn_id, "turn-1");
        assert!(episode.tools_used.is_empty());
        assert!(episode.failures.is_empty());
        assert_eq!(episode.outcome, EpisodeOutcome::Completed);
    }

    #[test]
    fn distill_with_tools() {
        let mut turn = make_test_turn("turn-1");
        turn.model_rounds.push(ModelRoundData {
            id: "round-1".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 0,
            timestamp: 1000,
            text_items: vec![],
            tool_items: vec![
                make_tool_item_with_result("Bash", true, None, Some("success output")),
                make_tool_item_with_result("Read", false, Some("file not found\nsecond line"), None),
                make_tool_item_with_result("Bash", true, None, Some("second success")),
                make_tool_item_with_result("Read", true, None, Some("read success")),
            ],
            thinking_items: vec![],
            start_time: 1000,
            end_time: Some(1500),
            duration_ms: Some(500),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        let episode = distill_episode(
            &turn,
            "test task".to_string(),
            "workspace-abc".to_string(),
            "agentic".to_string(),
            EpisodeOutcome::Completed,
        );

        // Should have 4 tool records
        assert_eq!(episode.tools_used.len(), 4);
        assert_eq!(episode.tools_used[0].name, "Bash");
        assert!(episode.tools_used[0].ok);
        assert_eq!(episode.tools_used[1].name, "Read");
        assert!(!episode.tools_used[1].ok);

        // Should have 1 failure (Read fails first, then Read succeeds)
        assert_eq!(episode.failures.len(), 1);
        assert_eq!(episode.failures[0].tool, "Read");
        assert!(episode.failures[0].error.contains("file not found"));
        // Repair should be set since there's a later successful Read
        assert!(episode.failures[0].repair.is_some());
        // Repair content should be the result_for_assistant of the successful call
        assert_eq!(episode.failures[0].repair.as_ref().unwrap(), "read success");
    }

    #[test]
    fn distill_with_failure_no_repair() {
        let mut turn = make_test_turn("turn-1");
        turn.model_rounds.push(ModelRoundData {
            id: "round-1".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 0,
            timestamp: 1000,
            text_items: vec![],
            tool_items: vec![make_tool_item_with_result(
                "Bash",
                false,
                Some("connection refused"),
                None,
            )],
            thinking_items: vec![],
            start_time: 1000,
            end_time: Some(1100),
            duration_ms: Some(100),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        let episode = distill_episode(
            &turn,
            "test task".to_string(),
            "workspace-abc".to_string(),
            "agentic".to_string(),
            EpisodeOutcome::Failed,
        );

        assert_eq!(episode.failures.len(), 1);
        assert_eq!(episode.failures[0].tool, "Bash");
        assert_eq!(episode.failures[0].error, "connection refused");
        assert!(episode.failures[0].repair.is_none());
    }

    #[test]
    fn distill_with_repair_across_rounds() {
        // Test that repair detection works across different rounds
        // round-0: Bash fails
        // round-1: Bash succeeds → repair should be populated
        let mut turn = make_test_turn("turn-1");

        // round 0 - Bash fails
        turn.model_rounds.push(ModelRoundData {
            id: "round-0".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 0,
            timestamp: 1000,
            text_items: vec![],
            tool_items: vec![make_tool_item_with_result(
                "Bash",
                false,
                Some("connection refused"),
                None,
            )],
            thinking_items: vec![],
            start_time: 1000,
            end_time: Some(1100),
            duration_ms: Some(100),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        // round 1 - Bash succeeds
        turn.model_rounds.push(ModelRoundData {
            id: "round-1".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 1,
            timestamp: 1200,
            text_items: vec![],
            tool_items: vec![make_tool_item_with_result(
                "Bash",
                true,
                None,
                Some("connection restored after retry"),
            )],
            thinking_items: vec![],
            start_time: 1200,
            end_time: Some(1300),
            duration_ms: Some(100),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        let episode = distill_episode(
            &turn,
            "test task".to_string(),
            "workspace-abc".to_string(),
            "agentic".to_string(),
            EpisodeOutcome::Completed,
        );

        assert_eq!(episode.failures.len(), 1);
        assert_eq!(episode.failures[0].tool, "Bash");
        assert_eq!(episode.failures[0].error, "connection refused");
        // Repair should be set with content from the later successful Bash
        assert!(
            episode.failures[0].repair.is_some(),
            "repair should be populated when same tool succeeds later"
        );
        assert_eq!(
            episode.failures[0].repair.as_ref().unwrap(),
            "connection restored after retry"
        );
    }

    #[test]
    fn repair_content_from_input_when_no_result_for_assistant() {
        let mut turn = make_test_turn("turn-1");
        // No result_for_assistant, so repair should come from input first line
        turn.model_rounds.push(ModelRoundData {
            id: "round-1".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 0,
            timestamp: 1000,
            text_items: vec![],
            tool_items: vec![ToolItemData {
                id: "tool-fail".to_string(),
                tool_name: "Bash".to_string(),
                tool_call: ToolCallData {
                    input: serde_json::json!({"command": "ls -la /tmp"}),
                    id: "call-fail".to_string(),
                },
                tool_result: Some(ToolResultData {
                    result: serde_json::json!({}),
                    success: false,
                    result_for_assistant: None,
                    error: Some("not found".to_string()),
                    duration_ms: None,
                }),
                ai_intent: None,
                start_time: 1000,
                end_time: Some(1100),
                duration_ms: Some(100),
                queue_wait_ms: None,
                preflight_ms: None,
                confirmation_wait_ms: None,
                execution_ms: None,
                order_index: None,
                is_subagent_item: None,
                parent_task_tool_id: None,
                subagent_session_id: None,
                subagent_model_id: None,
                subagent_model_alias: None,
                status: None,
                interruption_reason: None,
            }],
            thinking_items: vec![],
            start_time: 1000,
            end_time: Some(1100),
            duration_ms: Some(100),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        // round 1 - Bash succeeds with result_for_assistant
        turn.model_rounds.push(ModelRoundData {
            id: "round-2".to_string(),
            turn_id: "turn-1".to_string(),
            round_index: 1,
            timestamp: 1200,
            text_items: vec![],
            tool_items: vec![make_tool_item_with_result(
                "Bash",
                true,
                None,
                Some("ls succeeded"),
            )],
            thinking_items: vec![],
            start_time: 1200,
            end_time: Some(1300),
            duration_ms: Some(100),
            provider_id: None,
            model_id: None,
            model_alias: None,
            first_chunk_ms: None,
            first_visible_output_ms: None,
            stream_duration_ms: None,
            attempt_count: None,
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        });

        let episode = distill_episode(
            &turn,
            "test task".to_string(),
            "workspace-abc".to_string(),
            "agentic".to_string(),
            EpisodeOutcome::Completed,
        );

        assert_eq!(episode.failures.len(), 1);
        assert_eq!(episode.failures[0].repair.as_ref().unwrap(), "ls succeeded");
    }

    #[test]
    fn error_message_truncation() {
        let long_error = "error line 1\nline 2\nline 3".to_string();
        let truncated = first_line_truncated(&long_error, 200);
        assert_eq!(truncated, "error line 1");
        assert!(truncated.chars().count() <= 200);
    }
}
