//! Continuity self-check test (seed-restore-diff pattern) for T2-10.
//!
//! Architectural ceiling / scope note:
//! This test verifies continuity across SessionManager / PersistenceManager /
//! MemoryDb / Identity drop and rebuild within the core crate.
//! The "kill" boundary tested here is at the manager and persistence layer.
//! True OS process-level termination and daemon crash recovery is owned by T5
//! (core daemonization and process supervisor lifecycle).

#![cfg(test)]
#![allow(unused_imports)]

use super::super::super::session_manager::SessionManager;
use super::super::{test_manager, TestWorkspace};
use super::*;
use crate::agentic::core::{
    Message, MessageContent, MessageRole, MessageSemanticKind, Session, SessionConfig, SessionState,
};
use crate::agentic::identity::{load_identity, save_identity, unique_test_identity_path, with_test_identity_path};
use crate::agentic::persistence::PersistenceManager;
use crate::service::agent_memory::{
    default_memory_db_path, unique_test_memory_db_path, with_test_memory_db_path, Fact, FactConfidence, FactProvenance,
    FactScope, FactType, MemoryDb,
};
use crate::service::session::{
    DialogTurnData, DialogTurnKind, ModelRoundData, TextItemData, TurnStatus, UserMessageData,
};
use std::sync::Arc;
use std::sync::OnceLock;

async fn ensure_global_config_for_tests() {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() {
        return;
    }
    if let Err(e) = crate::service::config::GlobalConfigManager::initialize().await {
        eprintln!("GlobalConfigManager::initialize failed in test setup: {}", e);
    }
    DONE.set(()).ok();
}

fn create_text_item(id: &str, content: &str) -> TextItemData {
    TextItemData {
        id: id.to_string(),
        content: content.to_string(),
        is_streaming: false,
        timestamp: 0,
        is_markdown: true,
        order_index: None,
        is_subagent_item: None,
        parent_task_tool_id: None,
        subagent_session_id: None,
        status: None,
    }
}

fn create_model_round_with_text(round_id: &str, turn_id: &str, text: &str) -> ModelRoundData {
    ModelRoundData {
        id: round_id.to_string(),
        turn_id: turn_id.to_string(),
        round_index: 0,
        timestamp: 1001,
        text_items: vec![create_text_item(&format!("text-{}", round_id), text)],
        tool_items: Vec::new(),
        thinking_items: Vec::new(),
        start_time: 1000,
        end_time: Some(1002),
        duration_ms: Some(2),
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
    }
}

fn extract_role_and_text(msg: &Message) -> (MessageRole, String) {
    let text = match &msg.content {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Multimodal { text, .. } => text.clone(),
        MessageContent::Mixed { text, .. } => text.clone(),
        MessageContent::ToolResult {
            result_for_assistant, ..
        } => result_for_assistant.clone().unwrap_or_default(),
    };
    (msg.role.clone(), text)
}

#[tokio::test]
async fn continuity_selfcheck_seed_restore_diff() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Isolated test environment
    let workspace = TestWorkspace::new();
    let memory_guard = with_test_memory_db_path(unique_test_memory_db_path());
    let identity_guard = with_test_identity_path(unique_test_identity_path());
    ensure_global_config_for_tests().await;

    let session_id = "session-continuity-seed-001".to_string();
    let persistence_manager = Arc::new(PersistenceManager::new(workspace.path_manager())?);
    let manager = test_manager(persistence_manager.clone());

    // 2. Create session with fixed ID
    let session = manager
        .create_session_with_id(
            Some(session_id.clone()),
            "Continuity Selfcheck Session".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await?;

    // 3. Seed 2 dialog turns
    let turn_1_id = "turn-001".to_string();
    let turn_2_id = "turn-002".to_string();

    let mut turn_1 = DialogTurnData::new(
        turn_1_id.clone(),
        0,
        session_id.clone(),
        UserMessageData {
            id: "turn-1-user".to_string(),
            content: "User prompt for turn 1".to_string(),
            timestamp: 1000,
            metadata: None,
        },
    );
    turn_1.kind = DialogTurnKind::UserDialog;
    turn_1.status = TurnStatus::Completed;
    turn_1.model_rounds.push(create_model_round_with_text(
        "round-1",
        &turn_1_id,
        "Assistant response for turn 1",
    ));
    persistence_manager.save_dialog_turn(workspace.path(), &turn_1).await?;

    let mut turn_2 = DialogTurnData::new(
        turn_2_id.clone(),
        1,
        session_id.clone(),
        UserMessageData {
            id: "turn-2-user".to_string(),
            content: "User prompt for turn 2".to_string(),
            timestamp: 2000,
            metadata: None,
        },
    );
    turn_2.kind = DialogTurnKind::UserDialog;
    turn_2.status = TurnStatus::Completed;
    turn_2.model_rounds.push(create_model_round_with_text(
        "round-2",
        &turn_2_id,
        "Assistant response for turn 2",
    ));
    persistence_manager.save_dialog_turn(workspace.path(), &turn_2).await?;

    let mut updated_session = session.clone();
    updated_session.dialog_turn_ids = vec![turn_1_id.clone(), turn_2_id.clone()];
    persistence_manager
        .save_session(workspace.path(), &updated_session)
        .await?;

    let seeded_messages = vec![
        Message::user("User prompt for turn 1".to_string())
            .with_turn_id(turn_1_id.clone())
            .with_semantic_kind(MessageSemanticKind::ActualUserInput),
        Message::assistant("Assistant response for turn 1".to_string()).with_turn_id(turn_1_id.clone()),
        Message::user("User prompt for turn 2".to_string())
            .with_turn_id(turn_2_id.clone())
            .with_semantic_kind(MessageSemanticKind::ActualUserInput),
        Message::assistant("Assistant response for turn 2".to_string()).with_turn_id(turn_2_id.clone()),
    ];
    persistence_manager
        .save_turn_context_snapshot(workspace.path(), &session_id, 1, &seeded_messages)
        .await?;
    manager
        .replace_context_messages(&session_id, seeded_messages.clone())
        .await;

    // 4. Seed 2 memory facts
    let memory_db = MemoryDb::open(&default_memory_db_path())?;
    let workspace_key = workspace.path().to_string_lossy().to_string();

    let fact_1 = Fact {
        schema_version: 1,
        id: "fact-continuity-001".to_string(),
        text: "User prefers concise diffs and verified tests".to_string(),
        provenance: FactProvenance {
            session_id: session_id.clone(),
            turn_id: turn_1_id.clone(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        fact_type: FactType::Feedback,
        created_at: 1000,
    };
    let fact_2 = Fact {
        schema_version: 1,
        id: "fact-continuity-002".to_string(),
        text: "Global identity key continuity requirement".to_string(),
        provenance: FactProvenance {
            session_id: session_id.clone(),
            turn_id: turn_2_id.clone(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Global,
        fact_type: FactType::Project,
        created_at: 2000,
    };

    memory_db.insert_fact(&fact_1, Some(&workspace_key))?;
    memory_db.insert_fact(&fact_2, None)?;

    // 5. Seed identity configuration
    let expected_identity_text = "I am northhing assistant, focused on reliable orchestration.";
    save_identity(expected_identity_text)?;

    // 6. Kill / drop all in-memory runtime handles
    drop(manager);
    drop(persistence_manager);
    drop(memory_db);

    // 7. Rebuild runtime from persistent state
    let persistence_manager_rebuilt = Arc::new(PersistenceManager::new(workspace.path_manager())?);
    let manager_rebuilt = test_manager(persistence_manager_rebuilt.clone());

    let (restored_session, restored_turns) = manager_rebuilt
        .restore_session_with_turns(workspace.path(), &session_id)
        .await?;

    // 8. Equivalence Assertions

    // 8a. Session diff assertions
    assert_eq!(restored_turns.len(), 2);
    assert_eq!(
        restored_session.dialog_turn_ids,
        vec![turn_1_id.clone(), turn_2_id.clone()]
    );
    assert_eq!(restored_turns[0].turn_id, turn_1_id);
    assert_eq!(restored_turns[1].turn_id, turn_2_id);
    assert!(matches!(restored_session.state, SessionState::Idle));

    let restored_messages = manager_rebuilt.context_store.get_context_messages(&session_id);
    assert_eq!(restored_messages.len(), 4);
    assert_eq!(
        extract_role_and_text(&restored_messages[0]),
        (MessageRole::User, "User prompt for turn 1".to_string())
    );
    assert_eq!(
        restored_messages[0].metadata.turn_id.as_deref(),
        Some(turn_1_id.as_str())
    );

    assert_eq!(
        extract_role_and_text(&restored_messages[1]),
        (MessageRole::Assistant, "Assistant response for turn 1".to_string())
    );
    assert_eq!(
        restored_messages[1].metadata.turn_id.as_deref(),
        Some(turn_1_id.as_str())
    );

    assert_eq!(
        extract_role_and_text(&restored_messages[2]),
        (MessageRole::User, "User prompt for turn 2".to_string())
    );
    assert_eq!(
        restored_messages[2].metadata.turn_id.as_deref(),
        Some(turn_2_id.as_str())
    );

    assert_eq!(
        extract_role_and_text(&restored_messages[3]),
        (MessageRole::Assistant, "Assistant response for turn 2".to_string())
    );
    assert_eq!(
        restored_messages[3].metadata.turn_id.as_deref(),
        Some(turn_2_id.as_str())
    );

    // 8b. Memory diff assertions (text, scope, confidence, session_id, turn_id; excluding *_at)
    let memory_db_rebuilt = MemoryDb::open(&default_memory_db_path())?;
    let restored_facts = memory_db_rebuilt.get_facts(Some(&workspace_key))?;
    assert_eq!(restored_facts.len(), 2);

    let f1 = restored_facts
        .iter()
        .find(|f| f.id == "fact-continuity-001")
        .ok_or("fact 1 should exist")?;
    assert_eq!(f1.text, fact_1.text);
    assert_eq!(f1.scope, fact_1.scope);
    assert_eq!(f1.confidence, fact_1.confidence);
    assert_eq!(f1.provenance.session_id, fact_1.provenance.session_id);
    assert_eq!(f1.provenance.turn_id, fact_1.provenance.turn_id);
    assert_eq!(f1.fact_type, fact_1.fact_type);

    let f2 = restored_facts
        .iter()
        .find(|f| f.id == "fact-continuity-002")
        .ok_or("fact 2 should exist")?;
    assert_eq!(f2.text, fact_2.text);
    assert_eq!(f2.scope, fact_2.scope);
    assert_eq!(f2.confidence, fact_2.confidence);
    assert_eq!(f2.provenance.session_id, fact_2.provenance.session_id);
    assert_eq!(f2.provenance.turn_id, fact_2.provenance.turn_id);
    assert_eq!(f2.fact_type, fact_2.fact_type);

    // 8c. Identity diff assertion (full string match)
    let restored_identity = load_identity().ok_or("restored identity should exist")?;
    assert_eq!(restored_identity, expected_identity_text);

    drop(memory_guard);
    drop(identity_guard);
    Ok(())
}
