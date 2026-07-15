//! R59b split: rollback + delete session tests

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
async fn rollback_context_deletes_persisted_turns_from_target() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Rollback session".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    for index in 0..3 {
        let mut turn = DialogTurnData::new(
            format!("turn-{index}"),
            index,
            session.session_id.clone(),
            UserMessageData {
                id: format!("turn-{index}-user"),
                content: format!("prompt {index}"),
                timestamp: index as u64,
                metadata: None,
            },
        );
        turn.agent_type = Some(if index == 0 {
            "agentic".to_string()
        } else {
            "Plan".to_string()
        });
        persistence_manager
            .save_dialog_turn(workspace.path(), &turn)
            .await
            .expect("turn should save");
    }

    {
        let mut active = manager
            .sessions
            .get_mut(&session.session_id)
            .expect("session should be active");
        active.dialog_turn_ids = vec!["turn-0".to_string(), "turn-1".to_string(), "turn-2".to_string()];
        active.last_user_dialog_agent_type = Some("Plan".to_string());
    }
    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session.session_id,
            0,
            &[crate::agentic::core::Message::user("prompt 0".to_string())],
        )
        .await
        .expect("snapshot 0 should save");
    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session.session_id,
            1,
            &[
                crate::agentic::core::Message::user("prompt 0".to_string()),
                crate::agentic::core::Message::user("prompt 1".to_string()),
            ],
        )
        .await
        .expect("snapshot 1 should save");

    manager
        .rollback_context_to_turn_start(workspace.path(), &session.session_id, 1)
        .await
        .expect("rollback should succeed");

    let turns = persistence_manager
        .load_session_turns(workspace.path(), &session.session_id)
        .await
        .expect("turns should load");
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].user_message.content, "prompt 0");
    assert_eq!(turns[0].agent_type.as_deref(), Some("agentic"));
    assert!(persistence_manager
        .load_turn_context_snapshot(workspace.path(), &session.session_id, 1)
        .await
        .expect("snapshot load should succeed")
        .is_none());

    manager.sessions.remove(&session.session_id);
    let restored = manager
        .restore_session(workspace.path(), &session.session_id)
        .await
        .expect("session should restore");
    assert_eq!(restored.dialog_turn_ids, vec!["turn-0".to_string()]);
    assert_eq!(restored.last_user_dialog_agent_type.as_deref(), Some("agentic"));
    assert_eq!(manager.context_store.get_context_messages(&session.session_id).len(), 1);

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata should load")
        .expect("metadata should exist");
    assert_eq!(metadata.turn_count, 1);
}

#[tokio::test]
async fn rollback_sanitizes_pre_cutoff_snapshot_and_truncates_cutoff() {
    use crate::agentic::core::{InternalReminderKind, Message, MessageSemanticKind};

    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Rollback sanitize".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    for index in 0..=2 {
        let turn = DialogTurnData::new(
            format!("turn-{index}"),
            index,
            session.session_id.clone(),
            UserMessageData {
                id: format!("turn-{index}-user"),
                content: format!("prompt {index}"),
                timestamp: index as u64,
                metadata: None,
            },
        );
        persistence_manager
            .save_dialog_turn(workspace.path(), &turn)
            .await
            .expect("turn should save");
    }

    {
        let mut active = manager
            .sessions
            .get_mut(&session.session_id)
            .expect("session should be active");
        active.dialog_turn_ids = vec!["turn-0".to_string(), "turn-1".to_string(), "turn-2".to_string()];
    }

    manager
        .merge_session_custom_metadata(
            &session.session_id,
            json!({
                super::super::super::session_manager::LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY: 2,
            }),
        )
        .await
        .expect("cutoff metadata should save");

    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session.session_id,
            0,
            &[
                Message::internal_reminder(
                    InternalReminderKind::AgentListingDiff,
                    "# Agent Listing Update\n\nChanged",
                )
                .with_turn_id("turn-0".to_string()),
                Message::user("prompt 0".to_string())
                    .with_turn_id("turn-0".to_string())
                    .with_semantic_kind(MessageSemanticKind::ActualUserInput),
            ],
        )
        .await
        .expect("snapshot 0 should save");
    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session.session_id,
            1,
            &[
                Message::user("prompt 0".to_string()),
                Message::user("prompt 1".to_string()),
            ],
        )
        .await
        .expect("snapshot 1 should save");

    manager
        .rollback_context_to_turn_start(workspace.path(), &session.session_id, 1)
        .await
        .expect("rollback should succeed");

    let context_messages = manager.context_store.get_context_messages(&session.session_id);
    assert_eq!(context_messages.len(), 1);
    assert_eq!(
        context_messages[0].metadata.semantic_kind,
        Some(MessageSemanticKind::ActualUserInput)
    );

    let sanitized_snapshot = persistence_manager
        .load_turn_context_snapshot(workspace.path(), &session.session_id, 0)
        .await
        .expect("snapshot 0 load should succeed")
        .expect("snapshot 0 should still exist");
    assert_eq!(sanitized_snapshot.len(), 1);
    assert_eq!(
        sanitized_snapshot[0].metadata.semantic_kind,
        Some(MessageSemanticKind::ActualUserInput)
    );

    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session.session_id)
        .await
        .expect("metadata load should succeed")
        .expect("metadata should exist");
    assert_eq!(
        SessionManager::listing_baseline_rebuild_turn_index_from_metadata(Some(&metadata)),
        Some(1)
    );
}

#[tokio::test]
async fn rollback_to_empty_history_clears_last_user_dialog_agent_type() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Rollback empty history".to_string(),
            "Plan".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    let mut turn = DialogTurnData::new(
        "turn-0".to_string(),
        0,
        session.session_id.clone(),
        UserMessageData {
            id: "turn-0-user".to_string(),
            content: "plan prompt".to_string(),
            timestamp: 0,
            metadata: None,
        },
    );
    turn.agent_type = Some("Plan".to_string());
    persistence_manager
        .save_dialog_turn(workspace.path(), &turn)
        .await
        .expect("turn should save");

    {
        let mut active = manager
            .sessions
            .get_mut(&session.session_id)
            .expect("session should be active");
        active.dialog_turn_ids = vec!["turn-0".to_string()];
        active.last_user_dialog_agent_type = Some("Plan".to_string());
    }

    manager
        .rollback_context_to_turn_start(workspace.path(), &session.session_id, 0)
        .await
        .expect("rollback should succeed");

    let active = manager
        .get_session(&session.session_id)
        .expect("session should remain in memory");
    assert_eq!(active.agent_type, "Plan");
    assert_eq!(active.last_user_dialog_agent_type, None);
}

#[tokio::test]
async fn delete_session_removes_workspace_cache_entry() {
    let workspace = TestWorkspace::new();
    let manager = in_memory_test_manager();
    let session = manager
        .create_session(
            "Cached session".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    assert_eq!(
        manager
            .session_workspace_index
            .get(&session.session_id)
            .as_deref()
            .map(|entry| local_workspace_roots_equal(entry, workspace.path())),
        Some(true)
    );

    manager
        .delete_session(workspace.path(), &session.session_id)
        .await
        .expect("session should delete");

    assert!(manager.session_workspace_index.get(&session.session_id).is_none());
}

#[test]
fn build_messages_from_turns_skips_model_invisible_turns() {
    use crate::service::session::{DialogTurnData, DialogTurnKind, UserMessageData};

    let turns = vec![
        DialogTurnData::new(
            "turn-1".to_string(),
            0,
            "session-1".to_string(),
            UserMessageData {
                id: "user-1".to_string(),
                content: "hello".to_string(),
                timestamp: 1,
                metadata: None,
            },
        ),
        DialogTurnData::new_with_kind(
            DialogTurnKind::ManualCompaction,
            "turn-2".to_string(),
            1,
            "session-1".to_string(),
            None,
            UserMessageData {
                id: "user-2".to_string(),
                content: "/compact".to_string(),
                timestamp: 2,
                metadata: None,
            },
        ),
        DialogTurnData::new_with_kind(
            DialogTurnKind::LocalCommand,
            "turn-3".to_string(),
            2,
            "session-1".to_string(),
            None,
            UserMessageData {
                id: "user-3".to_string(),
                content: "# Session Usage Report".to_string(),
                timestamp: 3,
                metadata: Some(serde_json::json!({
                    "localCommandKind": "usage_report",
                    "modelVisible": false
                })),
            },
        ),
    ];

    let messages = SessionManager::build_messages_from_turns(&turns);

    assert_eq!(messages.len(), 1);
    assert!(messages[0].is_actual_user_message());
}
