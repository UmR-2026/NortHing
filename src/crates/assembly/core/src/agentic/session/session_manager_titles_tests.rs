//! Round 9b split: titles tests
//!
//! Test fns moved from session_manager_tests.rs (1:1 with production sibling).
//! Helpers (TestWorkspace, test_manager, etc.) live in the facade
//! and are imported via super::.

#![cfg(test)]
#![allow(unused_imports)]

use super::super::session_manager::SessionManager;

#[test]
fn fallback_session_title_uses_sentence_break_when_available() {
    let title = SessionManager::fallback_session_title("Fix the flaky integration test. Add logging for retries.", 20);

    assert_eq!(title, "Fix the flaky...");
}

#[test]
fn fallback_session_title_appends_ellipsis_when_truncated_without_sentence_break() {
    let title = SessionManager::fallback_session_title("Implement session title generation fallback", 12);

    assert_eq!(title, "Implement...");
}

#[test]
fn fallback_session_title_uses_default_for_blank_input() {
    let title = SessionManager::fallback_session_title("   ", 20);

    assert_eq!(title, "New Session");
}
