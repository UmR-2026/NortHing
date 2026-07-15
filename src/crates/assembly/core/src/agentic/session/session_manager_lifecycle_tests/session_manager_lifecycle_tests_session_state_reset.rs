//! R59b split: session state reset tests

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
async fn reset_session_state_if_processing_ignores_a_newer_turn() {
    let manager = in_memory_test_manager();
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Active session".to_string(),
        "agent".to_string(),
        SessionConfig::default(),
    );
    session.state = SessionState::Processing {
        current_turn_id: "turn-2".to_string(),
        phase: ProcessingPhase::Thinking,
    };
    manager.sessions.insert(session_id.clone(), session);

    manager.reset_session_state_if_processing(&session_id, "turn-1");

    let session = manager
        .get_session(&session_id)
        .expect("session should remain available");
    assert!(matches!(
        session.state,
        SessionState::Processing {
            ref current_turn_id,
            ..
        } if current_turn_id == "turn-2"
    ));
}

#[tokio::test]
async fn reset_session_state_if_processing_resets_the_matching_turn() {
    let manager = in_memory_test_manager();
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Active session".to_string(),
        "agent".to_string(),
        SessionConfig::default(),
    );
    session.state = SessionState::Processing {
        current_turn_id: "turn-1".to_string(),
        phase: ProcessingPhase::Thinking,
    };
    manager.sessions.insert(session_id.clone(), session);

    manager.reset_session_state_if_processing(&session_id, "turn-1");

    let session = manager
        .get_session(&session_id)
        .expect("session should remain available");
    assert!(matches!(session.state, SessionState::Idle));
}

#[tokio::test]
async fn update_session_state_for_turn_if_processing_ignores_a_newer_turn() {
    let manager = in_memory_test_manager();
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Active session".to_string(),
        "agent".to_string(),
        SessionConfig::default(),
    );
    session.state = SessionState::Processing {
        current_turn_id: "turn-2".to_string(),
        phase: ProcessingPhase::Thinking,
    };
    manager.sessions.insert(session_id.clone(), session);

    let updated = manager
        .update_session_state_for_turn_if_processing(&session_id, "turn-1", SessionState::Idle)
        .await
        .expect("conditional state update should not fail");

    let session = manager
        .get_session(&session_id)
        .expect("session should remain available");
    assert!(!updated);
    assert!(matches!(
        session.state,
        SessionState::Processing {
            ref current_turn_id,
            ..
        } if current_turn_id == "turn-2"
    ));
}

#[tokio::test]
async fn update_session_state_for_turn_if_processing_updates_matching_turn() {
    let manager = in_memory_test_manager();
    let session_id = Uuid::new_v4().to_string();
    let mut session = Session::new_with_id(
        session_id.clone(),
        "Active session".to_string(),
        "agent".to_string(),
        SessionConfig::default(),
    );
    session.state = SessionState::Processing {
        current_turn_id: "turn-1".to_string(),
        phase: ProcessingPhase::Thinking,
    };
    manager.sessions.insert(session_id.clone(), session);

    let updated = manager
        .update_session_state_for_turn_if_processing(&session_id, "turn-1", SessionState::Idle)
        .await
        .expect("conditional state update should not fail");

    let session = manager
        .get_session(&session_id)
        .expect("session should remain available");
    assert!(updated);
    assert!(matches!(session.state, SessionState::Idle));
}
