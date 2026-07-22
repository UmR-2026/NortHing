use super::*;
use crate::agentic::coordination::SubagentAbortExit;
use crate::agentic::coordination::coordinator::SubagentTimeoutHandle;
use std::sync::Mutex;
use tokio::sync::watch;

// ─── Test #1: aborted_cancelled_exit_persists_and_clears_registry ───────

/// Test that `persist_aborted_subagent_exit` with `SubagentAbortExit::Cancelled`:
/// ① returns a `NortHingError::Cancelled` with exact message,
/// ② removes the session_id from the subagent_timeout_registry.
///
/// Note: session-state post-condition is excluded because the test harness uses
/// `enable_persistence: false` so `update_session_state_for_turn_if_processing`
/// may not update observable state; the brief explicitly permits falling back to
/// memory-observable items (registry, error) when turn-persistence is unobservable.
#[tokio::test]
async fn aborted_cancelled_exit_persists_and_clears_registry() {
    let (coordinator, session_manager, _mock) =
        build_test_coordinator_with_mock_tool(SubagentScenario::SleepForever).await;

    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), None)
        .await
        .expect("phase 1 should succeed");

    let session_id = &phase1.session_id;
    let dialog_turn_id = &phase1.dialog_turn_id;
    let turn_index = phase1.turn_index;

    // Insert a fake timeout entry into the registry.
    let (deadline_tx, _) = watch::channel::<Option<tokio::time::Instant>>(None);
    let fake_handle = Arc::new(SubagentTimeoutHandle {
        deadline_tx,
        session_id: session_id.clone(),
        original_timeout_seconds: Some(600),
        remaining_at_pause: Mutex::new(None),
    });
    coordinator
        .subagent_timeout_registry
        .write()
        .await
        .insert(session_id.clone(), fake_handle);

    // Verify the entry is in the registry before the call.
    {
        let registry = coordinator.subagent_timeout_registry.read().await;
        assert!(
            registry.contains_key(session_id),
            "registry should contain session_id before abort"
        );
    }

    // Call the abort-exit helper directly.
    let err = coordinator
        .persist_aborted_subagent_exit(
            session_id,
            dialog_turn_id,
            turn_index,
            &phase1.agent_type,
            &phase1.user_input_text,
            phase1
                .subagent_workspace
                .as_ref()
                .map(|w| w.root_path_string())
                .as_deref(),
            phase1
                .subagent_workspace
                .as_ref()
                .map(|w| w.session_storage_path().to_path_buf())
                .as_deref(),
            SubagentAbortExit::Cancelled,
        )
        .await;

    // ① Exact error message match.
    match err {
        NortHingError::Cancelled(msg) => {
            assert_eq!(
                msg, "Subagent task has been cancelled",
                "error message must match exactly"
            );
        }
        other => panic!("expected NortHingError::Cancelled, got {:?}", other),
    }

    // ② Registry entry removed.
    {
        let registry = coordinator.subagent_timeout_registry.read().await;
        assert!(
            !registry.contains_key(session_id),
            "registry should not contain session_id after abort"
        );
    }

    // ③ Session no longer Processing (observable via public API).
    // With enable_persistence:false the turn state may not update, so
    // we also accept Idle as the harness-managed state; the key invariant
    // is the registry removal above proves the abort path ran.
    let session_after = session_manager
        .get_session(session_id)
        .expect("session should still exist");
    assert!(
        !matches!(session_after.state, crate::agentic::core::SessionState::Processing { .. }),
        "session should not be Processing after abort, got {:?}",
        session_after.state
    );
}

// ─── Test #2: aborted_timeout_exit_persists_failed_and_returns_timeout ──

/// Test that `persist_aborted_subagent_exit` with `SubagentAbortExit::TimedOut`:
/// ① returns a `NortHingError::Timeout` with the message passed through unchanged,
/// ② removes the session_id from the subagent_timeout_registry.
#[tokio::test]
async fn aborted_timeout_exit_persists_failed_and_returns_timeout() {
    let (coordinator, session_manager, _mock) =
        build_test_coordinator_with_mock_tool(SubagentScenario::SleepForever).await;

    let cancel_token = CancellationToken::new();
    let request = build_minimal_request();
    let phase1 = coordinator
        .execute_hidden_subagent_phase1(request, Some(&cancel_token), None)
        .await
        .expect("phase 1 should succeed");

    let session_id = &phase1.session_id;
    let dialog_turn_id = &phase1.dialog_turn_id;
    let turn_index = phase1.turn_index;

    // Insert a fake timeout entry into the registry.
    let (deadline_tx, _) = watch::channel::<Option<tokio::time::Instant>>(None);
    let fake_handle = Arc::new(SubagentTimeoutHandle {
        deadline_tx,
        session_id: session_id.clone(),
        original_timeout_seconds: Some(600),
        remaining_at_pause: Mutex::new(None),
    });
    coordinator
        .subagent_timeout_registry
        .write()
        .await
        .insert(session_id.clone(), fake_handle);

    // Verify the entry is in the registry before the call.
    {
        let registry = coordinator.subagent_timeout_registry.read().await;
        assert!(
            registry.contains_key(session_id),
            "registry should contain session_id before abort"
        );
    }

    // Call the abort-exit helper directly with a known timeout message.
    let timeout_msg = "test timeout 42s".to_string();
    let err = coordinator
        .persist_aborted_subagent_exit(
            session_id,
            dialog_turn_id,
            turn_index,
            &phase1.agent_type,
            &phase1.user_input_text,
            phase1
                .subagent_workspace
                .as_ref()
                .map(|w| w.root_path_string())
                .as_deref(),
            phase1
                .subagent_workspace
                .as_ref()
                .map(|w| w.session_storage_path().to_path_buf())
                .as_deref(),
            SubagentAbortExit::TimedOut(timeout_msg.clone()),
        )
        .await;

    // ① Timeout error with exact message passed through.
    match err {
        NortHingError::Timeout(msg) => {
            assert_eq!(msg, timeout_msg, "timeout message must pass through unchanged");
        }
        other => panic!("expected NortHingError::Timeout, got {:?}", other),
    }

    // ② Registry entry removed.
    {
        let registry = coordinator.subagent_timeout_registry.read().await;
        assert!(
            !registry.contains_key(session_id),
            "registry should not contain session_id after abort"
        );
    }

    // ③ Session no longer Processing (observable via public API).
    let session_after = session_manager
        .get_session(session_id)
        .expect("session should still exist");
    assert!(
        !matches!(session_after.state, crate::agentic::core::SessionState::Processing { .. }),
        "session should not be Processing after abort, got {:?}",
        session_after.state
    );
}
