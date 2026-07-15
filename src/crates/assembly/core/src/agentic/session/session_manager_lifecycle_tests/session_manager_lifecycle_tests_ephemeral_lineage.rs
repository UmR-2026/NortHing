//! R59b split: ephemeral child + lineage tests

#![cfg(test)]
#![allow(unused_imports)]

use super::super::super::session_manager::SessionManager;
use super::super::{in_memory_test_manager, test_manager, TestWorkspace};
use super::*;
use crate::agentic::core::{
    InternalReminderKind, Message, MessageContent, MessageRole, ProcessingPhase, Session, SessionConfig, SessionState,
};
use crate::agentic::persistence::PersistenceManager;
use crate::service::remote_ssh::workspace_state::local_workspace_roots_equal;
use crate::service::session::{DialogTurnData, DialogTurnKind, TurnStatus, UserMessageData};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn append_completed_local_command_turn_persists_without_model_context() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Usage session".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    let turn = manager
        .append_completed_local_command_turn(
            &session.session_id,
            "# Session Usage Report".to_string(),
            Some("local-usage-1".to_string()),
            Some(42),
            Some(json!({
                "localCommandKind": "usage_report",
                "modelVisible": false,
            })),
        )
        .await
        .expect("local command turn should persist");

    assert_eq!(turn.kind, DialogTurnKind::LocalCommand);
    assert_eq!(turn.status, TurnStatus::Completed);

    let active = manager
        .get_session(&session.session_id)
        .expect("session should remain active");
    assert_eq!(active.dialog_turn_ids, vec!["local-usage-1".to_string()]);
    assert!(manager
        .context_store
        .get_context_messages(&session.session_id)
        .is_empty());

    let persisted_turns = persistence_manager
        .load_session_turns(workspace.path(), &session.session_id)
        .await
        .expect("turns should load");
    assert_eq!(persisted_turns.len(), 1);
    assert_eq!(persisted_turns[0].kind, DialogTurnKind::LocalCommand);
    assert!(SessionManager::build_messages_from_turns(&persisted_turns).is_empty());

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata should load")
        .expect("metadata should exist");
    assert_eq!(metadata.turn_count, 1);
}

#[tokio::test]
async fn ephemeral_child_session_is_kept_in_memory_without_persisting() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());

    let session = manager
        .create_session_with_id_and_details(
            Some(Uuid::new_v4().to_string()),
            "Side thread".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
            Some("session-parent".to_string()),
            SessionKind::EphemeralChild,
        )
        .await
        .expect("ephemeral child session should create");

    assert!(manager.get_session(&session.session_id).is_some());
    assert!(persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata lookup should succeed")
        .is_none());
}

#[tokio::test]
async fn persist_session_lineage_updates_structured_relationship_and_clears_legacy_projection() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());

    let session = manager
        .create_session_with_id_and_details(
            Some(Uuid::new_v4().to_string()),
            "Review child".to_string(),
            "CodeReview".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
            Some("session-parent".to_string()),
            SessionKind::Standard,
        )
        .await
        .expect("session should create");

    manager
        .merge_session_custom_metadata(
            &session.session_id,
            json!({
                "kind": "review",
                "parentSessionId": "stale-parent",
                "parentRequestId": "stale-request",
                "parentDialogTurnId": "stale-turn",
                "parentTurnIndex": 1,
                "parentToolCallId": "stale-tool",
                "subagentType": "stale-subagent",
                "preservedKey": "preserved-value",
            }),
        )
        .await
        .expect("legacy compatibility metadata should seed");

    manager
        .persist_session_lineage(
            &session.session_id,
            SessionRelationship {
                kind: Some(SessionRelationshipKind::DeepReview),
                parent_session_id: Some("parent-1".to_string()),
                parent_request_id: Some("request-1".to_string()),
                parent_dialog_turn_id: Some("turn-2".to_string()),
                parent_turn_index: Some(2),
                parent_tool_call_id: None,
                subagent_type: None,
            },
        )
        .await
        .expect("lineage should persist");

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata lookup should succeed")
        .expect("metadata should exist");

    assert_eq!(
        metadata.relationship,
        Some(SessionRelationship {
            kind: Some(SessionRelationshipKind::DeepReview),
            parent_session_id: Some("parent-1".to_string()),
            parent_request_id: Some("request-1".to_string()),
            parent_dialog_turn_id: Some("turn-2".to_string()),
            parent_turn_index: Some(2),
            parent_tool_call_id: None,
            subagent_type: None,
        })
    );

    let custom_metadata = metadata
        .custom_metadata
        .expect("non-lineage custom metadata should remain");
    assert_eq!(custom_metadata["preservedKey"], "preserved-value");
    assert!(custom_metadata.get("kind").is_none());
    assert!(custom_metadata.get("parentSessionId").is_none());
    assert!(custom_metadata.get("parentRequestId").is_none());
    assert!(custom_metadata.get("parentDialogTurnId").is_none());
    assert!(custom_metadata.get("parentTurnIndex").is_none());
    assert!(custom_metadata.get("parentToolCallId").is_none());
    assert!(custom_metadata.get("subagentType").is_none());
}
