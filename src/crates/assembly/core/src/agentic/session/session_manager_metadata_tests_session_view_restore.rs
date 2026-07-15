#![cfg(test)]
#![allow(unused_imports)]

use super::super::super::session_manager::{SessionManager, SessionManagerConfig};
use super::super::{test_manager, test_manager_with_config, TestWorkspace};
use super::*;
use crate::agentic::core::{InternalReminderKind, Message, MessageSemanticKind, Session, SessionConfig, SessionKind};
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::{
    PromptCachePolicy, PromptCacheScope, SystemPromptCacheIdentity, UserContextCacheIdentity,
};
use crate::agentic::skill_agent_snapshot::{SkillSnapshotEntry, TurnSkillAgentSnapshot};
use crate::infrastructure::PathManager;
use crate::service::session::{
    DialogTurnData, ModelRoundData, SessionMetadata, SessionRelationship, SessionRelationshipKind, ToolCallData,
    ToolItemData, ToolResultData, UserMessageData,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn restore_session_view_loads_turns_without_restoring_runtime_context() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(
        PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager"),
    );
    let manager = test_manager(persistence_manager.clone());
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Large history".to_string(),
        "agent".to_string(),
        SessionConfig {
            workspace_path: Some(workspace.path().to_string_lossy().to_string()),
            ..Default::default()
        },
    );
    session.dialog_turn_ids = vec!["turn-1".to_string()];

    persistence_manager
        .save_session(workspace.path(), &session)
        .await
        .expect("session should save");
    let turn = DialogTurnData::new(
        "turn-1".to_string(),
        0,
        session_id.clone(),
        UserMessageData {
            id: "turn-1-user".to_string(),
            content: "hello".to_string(),
            timestamp: 1,
            metadata: None,
        },
    );
    persistence_manager
        .save_dialog_turn(workspace.path(), &turn)
        .await
        .expect("turn should save");
    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session_id,
            0,
            &[Message::user("snapshot prompt".to_string())],
        )
        .await
        .expect("context snapshot should save");

    let (view_session, turns) = manager
        .restore_session_view(workspace.path(), &session_id)
        .await
        .expect("session view should restore");

    assert_eq!(view_session.dialog_turn_ids, vec!["turn-1".to_string()]);
    assert_eq!(turns.len(), 1);
    assert!(manager.get_session(&session_id).is_none());
    assert!(manager.context_store.get_context_messages(&session_id).is_empty());
}

#[tokio::test]
async fn restore_session_view_preserves_full_visible_tool_result_payload() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(
        PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager"),
    );
    let manager = test_manager(persistence_manager.clone());
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "History with tool output".to_string(),
        "agent".to_string(),
        SessionConfig {
            workspace_path: Some(workspace.path().to_string_lossy().to_string()),
            ..Default::default()
        },
    );
    session.dialog_turn_ids = vec!["turn-1".to_string()];

    persistence_manager
        .save_session(workspace.path(), &session)
        .await
        .expect("session should save");

    let visible_output = "complete visible output ".repeat(128);
    let assistant_output = "assistant visible summary ".repeat(16);
    let mut turn = DialogTurnData::new(
        "turn-1".to_string(),
        0,
        session_id.clone(),
        UserMessageData {
            id: "turn-1-user".to_string(),
            content: "show full output".to_string(),
            timestamp: 1,
            metadata: None,
        },
    );
    turn.model_rounds.push(ModelRoundData {
        id: "round-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_index: 0,
        timestamp: 1,
        text_items: vec![],
        tool_items: vec![ToolItemData {
            id: "tool-1".to_string(),
            tool_name: "Bash".to_string(),
            tool_call: ToolCallData {
                id: "call-1".to_string(),
                input: json!({ "command": "printf output" }),
            },
            tool_result: Some(ToolResultData {
                result: json!({
                    "stdout": visible_output,
                    "nested": {
                        "stderr": "also visible",
                    },
                }),
                success: true,
                result_for_assistant: Some(assistant_output.clone()),
                error: None,
                duration_ms: Some(1),
            }),
            ai_intent: None,
            start_time: 1,
            end_time: Some(2),
            duration_ms: Some(1),
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
            status: Some("completed".to_string()),
            interruption_reason: None,
        }],
        thinking_items: vec![],
        start_time: 1,
        end_time: Some(2),
        duration_ms: Some(1),
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
    persistence_manager
        .save_dialog_turn(workspace.path(), &turn)
        .await
        .expect("turn should save");

    let (view_session, turns) = manager
        .restore_session_view(workspace.path(), &session_id)
        .await
        .expect("session view should restore");

    let restored_result = turns[0].model_rounds[0].tool_items[0]
        .tool_result
        .as_ref()
        .expect("tool result should be preserved");
    assert_eq!(view_session.dialog_turn_ids, vec!["turn-1".to_string()]);
    assert_eq!(restored_result.result["stdout"].as_str(), Some(visible_output.as_str()));
    assert_eq!(
        restored_result.result["nested"]["stderr"].as_str(),
        Some("also visible")
    );
    assert_eq!(
        restored_result.result_for_assistant.as_deref(),
        Some(assistant_output.as_str())
    );
    assert!(manager.get_session(&session_id).is_none());
}
