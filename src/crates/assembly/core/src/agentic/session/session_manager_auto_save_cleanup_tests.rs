//! Round 9b split: auto_save_cleanup tests
//!
//! Test fns moved from session_manager_tests.rs (1:1 with production sibling).
//! Helpers (TestWorkspace, test_manager, etc.) live in the facade
//! and are imported via super::.

#![cfg(test)]
#![allow(unused_imports)]

use super::super::session_manager::SessionManager;
use super::*;
use super::{in_memory_test_manager, TestWorkspace};
use crate::agentic::core::{Session, SessionConfig};
use dashmap::try_result::TryResult;

#[tokio::test]
async fn auto_save_interval_waits_before_first_tick() {
    let mut ticker = SessionManager::auto_save_interval(Duration::from_millis(40));
    let started = tokio::time::Instant::now();

    ticker.tick().await;

    assert!(started.elapsed() >= Duration::from_millis(30));
}

#[tokio::test]
async fn auto_save_snapshot_collection_releases_session_map_guards() {
    let workspace = TestWorkspace::new();
    let manager = in_memory_test_manager();
    let session = manager
        .create_session(
            "Auto-save snapshot".to_string(),
            "agent".to_string(),
            SessionConfig {
                workspace_path: Some(workspace.path().to_string_lossy().to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("session should create");

    let snapshots = SessionManager::collect_auto_save_snapshots(&manager.sessions);
    assert!(snapshots
        .iter()
        .any(|snapshot| snapshot.session_id == session.session_id));

    match manager.sessions.try_get_mut(&session.session_id) {
        TryResult::Present(_) => {}
        TryResult::Absent => panic!("session should remain present"),
        TryResult::Locked => panic!("snapshot collection should not retain session map guards"),
    };
}
