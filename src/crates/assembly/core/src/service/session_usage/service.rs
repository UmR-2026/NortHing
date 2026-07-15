use crate::agentic::persistence::PersistenceManager;
use crate::service::session::{DialogTurnData, DialogTurnKind, ModelRoundData, ToolItemData, TurnStatus};
use crate::service::session_usage::types::*;
use crate::service::token_usage::{TokenUsageRecord, TokenUsageService};
use crate::util::errors::NortHingResult;

pub use super::entry::SessionUsageReportRequest;

pub async fn generate_session_usage_report(
    persistence_manager: &PersistenceManager,
    token_usage_service: Option<&TokenUsageService>,
    request: SessionUsageReportRequest,
) -> NortHingResult<SessionUsageReport> {
    super::entry::generate_session_usage_report(persistence_manager, token_usage_service, request).await
}

pub fn build_session_usage_report_from_turns(
    request: SessionUsageReportRequest,
    turns: &[DialogTurnData],
    token_records: &[TokenUsageRecord],
    generated_at: i64,
) -> SessionUsageReport {
    super::entry::build_session_usage_report_from_turns(request, turns, token_records, generated_at)
}

pub fn build_session_usage_report_from_sources(
    request: SessionUsageReportRequest,
    turns: &[DialogTurnData],
    token_records: &[TokenUsageRecord],
    snapshot_facts: &UsageSnapshotFacts,
    generated_at: i64,
) -> SessionUsageReport {
    super::entry::build_session_usage_report_from_sources(request, turns, token_records, snapshot_facts, generated_at)
}

#[cfg(test)]
pub mod test_helpers {
    use super::super::*;
    use crate::service::session::{
        DialogTurnData, DialogTurnKind, ModelRoundData, ToolCallData, ToolItemData, ToolResultData, TurnStatus,
        UserMessageData,
    };
    use crate::service::token_usage::TokenUsageRecord;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    pub fn test_request(remote_connection_id: Option<&str>) -> SessionUsageReportRequest {
        SessionUsageReportRequest {
            session_id: "session-1".to_string(),
            workspace_path: Some("D:/workspace/northhing".to_string()),
            remote_connection_id: remote_connection_id.map(ToOwned::to_owned),
            remote_ssh_host: remote_connection_id.map(|_| "host.example".to_string()),
            include_hidden_subagents: true,
        }
    }

    pub fn test_snapshot_facts(operations: Vec<UsageSnapshotOperationSummary>) -> UsageSnapshotFacts {
        UsageSnapshotFacts {
            source_available: true,
            operations,
        }
    }

    pub fn test_snapshot_operation(
        operation_id: &str,
        turn_index: usize,
        file_path: &str,
        lines_added: u64,
        lines_removed: u64,
    ) -> UsageSnapshotOperationSummary {
        UsageSnapshotOperationSummary {
            operation_id: operation_id.to_string(),
            session_id: "session-1".to_string(),
            turn_index,
            file_path: file_path.to_string(),
            lines_added,
            lines_removed,
        }
    }

    pub fn test_turn(turn_id: &str, turn_index: usize, kind: DialogTurnKind) -> DialogTurnData {
        test_turn_with_tools(
            turn_id,
            turn_index,
            kind,
            vec![test_tool_item(
                &format!("tool-{}", turn_index),
                "write_file",
                Some(true),
                100,
                "D:/workspace/northhing/src/main.rs",
            )],
        )
    }

    pub fn test_turn_with_tools(
        turn_id: &str,
        turn_index: usize,
        kind: DialogTurnKind,
        tool_items: Vec<ToolItemData>,
    ) -> DialogTurnData {
        DialogTurnData {
            turn_id: turn_id.to_string(),
            turn_index,
            session_id: "session-1".to_string(),
            timestamp: 1_000 + turn_index as u64,
            kind,
            agent_type: None,
            user_message: UserMessageData {
                id: format!("user-{}", turn_index),
                content: "hidden from report".to_string(),
                timestamp: 1_000 + turn_index as u64,
                metadata: None,
            },
            model_rounds: vec![ModelRoundData {
                id: format!("round-{}", turn_index),
                turn_id: turn_id.to_string(),
                round_index: 0,
                timestamp: 1_000 + turn_index as u64,
                text_items: vec![],
                tool_items,
                thinking_items: vec![],
                start_time: 1_000 + turn_index as u64,
                end_time: Some(1_200 + turn_index as u64),
                duration_ms: Some(200),
                provider_id: None,
                model_id: Some("model-a".to_string()),
                model_alias: Some("model-a".to_string()),
                first_chunk_ms: None,
                first_visible_output_ms: None,
                stream_duration_ms: None,
                attempt_count: None,
                failure_category: None,
                token_details: None,
                status: "completed".to_string(),
            }],
            start_time: 1_000 + turn_index as u64,
            end_time: Some(1_300 + turn_index as u64),
            duration_ms: Some(300),
            token_usage: None,
            status: TurnStatus::Completed,
        }
    }

    pub fn test_model_round(
        id: &str,
        turn_id: &str,
        round_index: usize,
        model_id: &str,
        duration_ms: u64,
    ) -> ModelRoundData {
        ModelRoundData {
            id: id.to_string(),
            turn_id: turn_id.to_string(),
            round_index,
            timestamp: 1_000 + round_index as u64,
            text_items: vec![],
            tool_items: vec![],
            thinking_items: vec![],
            start_time: 1_000 + round_index as u64,
            end_time: Some(1_000 + round_index as u64 + duration_ms),
            duration_ms: Some(duration_ms),
            provider_id: Some("test-provider".to_string()),
            model_id: Some(model_id.to_string()),
            model_alias: Some(model_id.to_string()),
            first_chunk_ms: Some(5),
            first_visible_output_ms: Some(8),
            stream_duration_ms: Some(duration_ms.saturating_sub(10)),
            attempt_count: Some(1),
            failure_category: None,
            token_details: None,
            status: "completed".to_string(),
        }
    }

    pub fn test_tool_item(
        id: &str,
        tool_name: &str,
        success: Option<bool>,
        duration_ms: u64,
        file_path: &str,
    ) -> ToolItemData {
        test_tool_item_with_input(
            id,
            tool_name,
            success,
            duration_ms,
            json!({
                "file_path": file_path
            }),
        )
    }

    pub fn test_tool_item_with_input(
        id: &str,
        tool_name: &str,
        success: Option<bool>,
        duration_ms: u64,
        input: serde_json::Value,
    ) -> ToolItemData {
        ToolItemData {
            id: id.to_string(),
            tool_name: tool_name.to_string(),
            tool_call: ToolCallData {
                input,
                id: format!("call-{}", id),
            },
            tool_result: success.map(|success| ToolResultData {
                result: json!({}),
                success,
                result_for_assistant: None,
                error: (!success).then(|| "tool failed".to_string()),
                duration_ms: Some(duration_ms),
            }),
            ai_intent: None,
            start_time: 1_000,
            end_time: Some(1_000 + duration_ms),
            duration_ms: Some(duration_ms),
            order_index: None,
            is_subagent_item: None,
            parent_task_tool_id: None,
            subagent_session_id: None,
            subagent_model_id: None,
            subagent_model_alias: None,
            status: Some(
                match success {
                    Some(true) => "completed",
                    Some(false) => "failed",
                    None => "cancelled",
                }
                .to_string(),
            ),
            interruption_reason: success.is_none().then(|| "cancelled".to_string()),
            queue_wait_ms: None,
            preflight_ms: None,
            confirmation_wait_ms: None,
            execution_ms: None,
        }
    }

    pub fn test_token_record(
        model_id: &str,
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: u32,
    ) -> TokenUsageRecord {
        TokenUsageRecord {
            model_id: model_id.to_string(),
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
            timestamp: Utc.timestamp_millis_opt(1_778_347_200_000).unwrap(),
            input_tokens,
            output_tokens,
            cached_tokens,
            cached_tokens_available: false,
            cache_write_tokens: 0,
            total_tokens: input_tokens + output_tokens,
            token_details: None,
            is_subagent: false,
        }
    }

    pub fn reported_token_record(
        model_id: &str,
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: u32,
    ) -> TokenUsageRecord {
        let mut record = test_token_record(model_id, input_tokens, output_tokens, cached_tokens);
        record.cached_tokens_available = true;
        record
    }
}
