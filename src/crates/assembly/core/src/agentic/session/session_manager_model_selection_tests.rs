//! Round 9b split: model_selection tests
//!
//! Test fns moved from session_manager_tests.rs (1:1 with production sibling).
//! Helpers (TestWorkspace, test_manager, etc.) live in the facade
//! and are imported via super::.

#![cfg(test)]
#![allow(unused_imports)]

use super::super::session_manager::SessionManager;
use super::*;
use super::{test_model, ServiceAIConfig};
use crate::agentic::core::{Session, SessionConfig};

#[test]
fn sync_session_context_window_refreshes_stale_explicit_model_window() {
    let mut ai_config = ServiceAIConfig::default();
    ai_config.models = vec![test_model("deepseek-v4-pro", 1_000_000)];

    let mut session = Session::new_with_id(
        "session-804".to_string(),
        "DeepSeek session".to_string(),
        "agentic".to_string(),
        SessionConfig {
            model_id: Some("deepseek-v4-pro".to_string()),
            max_context_tokens: 256_000,
            ..Default::default()
        },
    );

    let resolved = SessionManager::sync_session_context_window_from_ai_config(&mut session, &ai_config);

    assert_eq!(resolved, Some(1_000_000));
    assert_eq!(session.config.max_context_tokens, 1_000_000);
}

#[test]
fn sync_session_context_window_resolves_auto_through_agent_model_then_primary() {
    let mut ai_config = ServiceAIConfig::default();
    ai_config.models = vec![
        test_model("primary-model", 512_000),
        test_model("agent-model", 1_000_000),
    ];
    ai_config.default_models.primary = Some("primary-model".to_string());
    ai_config
        .agent_models
        .insert("agentic".to_string(), "agent-model".to_string());

    let mut session = Session::new_with_id(
        "session-auto".to_string(),
        "Auto session".to_string(),
        "agentic".to_string(),
        SessionConfig {
            model_id: Some("auto".to_string()),
            max_context_tokens: 256_000,
            ..Default::default()
        },
    );

    let resolved = SessionManager::sync_session_context_window_from_ai_config(&mut session, &ai_config);

    assert_eq!(resolved, Some(1_000_000));
    assert_eq!(session.config.max_context_tokens, 1_000_000);

    ai_config.agent_models.clear();
    session.config.max_context_tokens = 256_000;

    let resolved = SessionManager::sync_session_context_window_from_ai_config(&mut session, &ai_config);

    assert_eq!(resolved, Some(512_000));
    assert_eq!(session.config.max_context_tokens, 512_000);
}
