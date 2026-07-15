//! R59b split: restore session + start dialog turn tests

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
async fn restore_session_resets_processing_state_without_marking_unread_completion() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Legacy processing session".to_string(),
        "agent".to_string(),
        SessionConfig {
            workspace_path: Some(workspace.path().to_string_lossy().to_string()),
            ..Default::default()
        },
    );
    session.state = SessionState::Processing {
        current_turn_id: "turn-1".to_string(),
        phase: ProcessingPhase::Thinking,
    };

    persistence_manager
        .save_session(workspace.path(), &session)
        .await
        .expect("session should save");
    persistence_manager
        .save_session_state(workspace.path(), &session_id, &session.state)
        .await
        .expect("processing state should save");

    let manager = test_manager(persistence_manager.clone());
    let restored = manager
        .restore_session(workspace.path(), &session_id)
        .await
        .expect("session should restore");
    let metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session_id)
        .await
        .expect("metadata should load")
        .expect("metadata should exist");

    assert!(matches!(restored.state, SessionState::Idle));
    assert_eq!(metadata.unread_completion, None);
}

#[tokio::test]
async fn start_dialog_turn_with_existing_context_persists_turn_and_snapshot() {
    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence"));
    let manager = test_manager(persistence_manager.clone());
    let session = manager
        .create_session(
            "Fork child".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    let seeded_messages = vec![
        Message::user("fork reminder".to_string()),
        Message::assistant("inherited context".to_string()),
    ];
    manager
        .replace_context_messages(&session.session_id, seeded_messages.clone())
        .await;

    let turn_id = manager
        .start_dialog_turn_with_existing_context(
            &session.session_id,
            "agentic".to_string(),
            "delegate task".to_string(),
            Some("subagent-turn-0".to_string()),
            None,
        )
        .await
        .expect("turn should start");

    assert_eq!(turn_id, "subagent-turn-0");
    assert_eq!(
        manager
            .get_session(&session.session_id)
            .expect("session should remain in memory")
            .dialog_turn_ids,
        vec!["subagent-turn-0".to_string()]
    );

    let persisted_turn = persistence_manager
        .load_dialog_turn(workspace.path(), &session.session_id, 0)
        .await
        .expect("turn load should succeed")
        .expect("turn should exist");
    assert_eq!(persisted_turn.turn_id, "subagent-turn-0");
    assert_eq!(persisted_turn.user_message.content, "delegate task");

    let snapshot = persistence_manager
        .load_turn_context_snapshot(workspace.path(), &session.session_id, 0)
        .await
        .expect("snapshot load should succeed")
        .expect("snapshot should exist");
    assert_eq!(snapshot.len(), seeded_messages.len());
    assert!(matches!(snapshot[0].role, MessageRole::User));
    assert!(matches!(snapshot[1].role, MessageRole::Assistant));
    assert!(matches!(
        &snapshot[0].content,
        MessageContent::Text(text) if text == "fork reminder"
    ));
    assert!(matches!(
        &snapshot[1].content,
        MessageContent::Text(text) if text == "inherited context"
    ));

    let runtime_context = manager
        .get_context_messages(&session.session_id)
        .await
        .expect("runtime context should remain readable");
    assert_eq!(runtime_context.len(), seeded_messages.len());
}

#[tokio::test]
async fn restore_session_sanitizes_pre_cutoff_listing_diff_snapshot() {
    use crate::agentic::core::{InternalReminderKind, Message, MessageSemanticKind};

    let workspace = TestWorkspace::new();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager()).expect("persistence manager"));
    let manager = test_manager(persistence_manager.clone());
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Restore sanitize".to_string(),
        "agentic".to_string(),
        SessionConfig {
            workspace_path: Some(workspace.path().to_string_lossy().to_string()),
            ..Default::default()
        },
    );
    session.dialog_turn_ids = vec!["turn-0".to_string(), "turn-1".to_string()];

    persistence_manager
        .save_session(workspace.path(), &session)
        .await
        .expect("session should save");

    let mut metadata = persistence_manager
        .load_session_metadata(workspace.path(), &session_id)
        .await
        .expect("metadata load should succeed")
        .expect("metadata should exist");
    metadata.custom_metadata = Some(json!({
        super::super::super::session_manager::LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY: 2,
    }));
    persistence_manager
        .save_session_metadata(workspace.path(), &metadata)
        .await
        .expect("metadata should save");

    for index in 0..=1 {
        let turn = DialogTurnData::new(
            format!("turn-{index}"),
            index,
            session_id.clone(),
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

    persistence_manager
        .save_turn_context_snapshot(
            workspace.path(),
            &session_id,
            1,
            &[
                Message::internal_reminder(
                    InternalReminderKind::SkillListingDiff,
                    "# Skill Listing Update\n\nChanged",
                )
                .with_turn_id("turn-1".to_string()),
                Message::user("prompt 1".to_string())
                    .with_turn_id("turn-1".to_string())
                    .with_semantic_kind(MessageSemanticKind::ActualUserInput),
            ],
        )
        .await
        .expect("snapshot should save");

    let restored = manager
        .restore_session(workspace.path(), &session_id)
        .await
        .expect("session should restore");

    assert_eq!(
        restored.dialog_turn_ids,
        vec!["turn-0".to_string(), "turn-1".to_string()]
    );
    let context_messages = manager.context_store.get_context_messages(&session_id);
    assert_eq!(context_messages.len(), 1);
    assert_eq!(
        context_messages[0].metadata.semantic_kind,
        Some(MessageSemanticKind::ActualUserInput)
    );

    let sanitized_snapshot = persistence_manager
        .load_turn_context_snapshot(workspace.path(), &session_id, 1)
        .await
        .expect("snapshot load should succeed")
        .expect("snapshot should still exist");
    assert_eq!(sanitized_snapshot.len(), 1);
    assert_eq!(
        sanitized_snapshot[0].metadata.semantic_kind,
        Some(MessageSemanticKind::ActualUserInput)
    );
}
