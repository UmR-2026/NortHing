//! Turn scheduling and execution port tests.
//!
//! Tests for `AgentTurnCancellationPort`, subagent timeout handling,
//! max-concurrency normalization, and background subagent text formatting.

use super::super::{format_background_subagent_delivery_text, format_background_subagent_display_text, SubagentResult};
use crate::agentic::coordination::tests::build_isolated_coordinator;
use crate::agentic::core::{ProcessingPhase, SessionConfig, SessionState};
use crate::agentic::events::AgenticEvent;
use crate::util::errors::NortHingError;
use northhing_runtime_ports::{AgentTurnCancellationPort, DelegationPolicy};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use tokio::sync::watch;
use tokio::time::{Duration as TokioDuration, Instant};

#[test]
fn conversation_coordinator_exposes_remote_runtime_ports() {
    fn assert_cancellation_port<T: AgentTurnCancellationPort>() {}
    fn assert_state_port<T: northhing_runtime_ports::RemoteControlStatePort>() {}

    assert_cancellation_port::<crate::agentic::coordination::coordinator::ConversationCoordinator>();
    assert_state_port::<crate::agentic::coordination::coordinator::ConversationCoordinator>();
}

#[test]
fn clamps_subagent_max_concurrency_into_safe_range() {
    use crate::agentic::coordination::coordinator::normalize_subagent_max_concurrency;
    use crate::agentic::coordination::port_types::{DEFAULT_SUBAGENT_MAX_CONCURRENCY, MAX_SUBAGENT_MAX_CONCURRENCY};
    assert_eq!(normalize_subagent_max_concurrency(0), 1);
    assert_eq!(normalize_subagent_max_concurrency(5), 5);
    assert_eq!(
        normalize_subagent_max_concurrency(usize::MAX),
        MAX_SUBAGENT_MAX_CONCURRENCY
    );
}

// normalize_subagent_max_concurrency lives in coordinator.rs.
// The test references it through the direct module path.

#[test]
fn subagent_timeout_disable_clears_active_deadline() {
    use crate::agentic::coordination::coordinator::SubagentTimeoutAction;

    let initial_deadline = Instant::now() + TokioDuration::from_secs(1200);
    let (deadline_tx, mut deadline_rx) = watch::channel(Some(initial_deadline));
    let handle = crate::agentic::coordination::coordinator::SubagentTimeoutHandle {
        deadline_tx,
        session_id: "subagent-session".to_string(),
        original_timeout_seconds: Some(1200),
        remaining_at_pause: Mutex::new(None),
    };

    handle.apply_action(SubagentTimeoutAction::Disable);

    assert!(deadline_rx.borrow_and_update().is_none());
}

#[test]
fn background_subagent_delivery_text_includes_background_task_id() {
    let completed = SubagentResult::completed("done".to_string());
    let completed_text = format_background_subagent_delivery_text("bg-subagent-123", "GeneralPurpose", Ok(&completed));
    assert!(completed_text.contains(
        "Background subagent 'GeneralPurpose' (background_task_id='bg-subagent-123') completed successfully:"
    ));
    assert!(completed_text.contains("<result>\n"));
    assert!(!completed_text.contains("background_task_id=\"bg-subagent-123\""));

    let partial = SubagentResult::partial_timeout("partial".to_string(), "timeout".to_string());
    let partial_text = format_background_subagent_delivery_text("bg-subagent-456", "GeneralPurpose", Ok(&partial));
    assert!(partial_text.contains(
        "Background subagent 'GeneralPurpose' (background_task_id='bg-subagent-456') completed with partial timeout result:"
    ));
    assert!(partial_text.contains("<partial_result status=\"partial_timeout\">\n"));
    assert!(!partial_text.contains("background_task_id=\"bg-subagent-456\""));

    let failed_text = format_background_subagent_delivery_text(
        "bg-subagent-789",
        "GeneralPurpose",
        Err(&NortHingError::tool("boom".to_string())),
    );
    assert!(failed_text.contains(
        "Background subagent 'GeneralPurpose' (background_task_id='bg-subagent-789') failed before producing a final result."
    ));
    assert!(failed_text.contains("Error:"));
}

#[test]
fn background_subagent_display_text_is_concise() {
    let completed = SubagentResult::completed("done".to_string());
    assert_eq!(
        format_background_subagent_display_text(Ok(&completed)),
        "Background subagent completed successfully."
    );

    let partial = SubagentResult::partial_timeout("partial".to_string(), "timeout".to_string());
    assert_eq!(
        format_background_subagent_display_text(Ok(&partial)),
        "Background subagent completed with a partial timeout result."
    );

    assert_eq!(
        format_background_subagent_display_text(Err(&NortHingError::tool("boom".to_string()))),
        "Background subagent failed before producing a final result."
    );
}

// 2026-07-18 (W3a-3): Cancel convergence test. Verifies that when a turn is
// cancelled but the spawn task does not drain within the 1.5s window
// (simulated by manually incrementing the active_turns_per_session counter),
// the cancel path emits DialogTurnCancelled and transitions the session to
// Idle. This covers the "stuck turn" scenario where the spawned task would
// not emit the terminal event on its own.
#[tokio::test]
async fn cancel_convergence_emits_terminal_event_when_turn_stuck() {
    let (coordinator, session_manager) = build_isolated_coordinator();
    let workspace_path =
        std::env::temp_dir().join(format!("northhing-cancel-convergence-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");

    // Create a session and set it to Processing state.
    let session = session_manager
        .create_session(
            "CancelConvergenceTest".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let session_id = session.session_id.clone();
    let turn_id = "test-turn-stuck".to_string();
    session_manager
        .update_session_state(
            &session_id,
            SessionState::Processing {
                current_turn_id: turn_id.clone(),
                phase: ProcessingPhase::Thinking,
            },
        )
        .await
        .expect("should set processing state");

    // Simulate a stuck turn by incrementing the active_turns_per_session counter.
    let counter = coordinator
        .active_turns_per_session
        .entry(session_id.clone())
        .or_insert_with(|| std::sync::Arc::new(AtomicUsize::new(0)))
        .clone();
    counter.fetch_add(1, Ordering::SeqCst);

    // Cancel the turn. Since the counter is > 0, wait_session_drained will
    // time out and the convergence fallback should fire.
    coordinator
        .cancel_dialog_turn(&session_id, &turn_id)
        .await
        .expect("cancel should succeed");

    // Verify session state converged to Idle.
    let final_state = session_manager
        .get_session(&session_id)
        .map(|s| s.state)
        .expect("session should exist");
    assert_eq!(
        final_state,
        SessionState::Idle,
        "session should be Idle after cancel convergence"
    );

    // Verify DialogTurnCancelled event was emitted.
    let events = coordinator.event_queue.dequeue_batch(100).await;
    let cancelled_event = events.iter().find(|e| {
        matches!(
            &e.event,
            AgenticEvent::DialogTurnCancelled { session_id: sid, turn_id: tid }
                if sid == &session_id && tid == &turn_id
        )
    });
    assert!(
        cancelled_event.is_some(),
        "DialogTurnCancelled event should be emitted for stuck turn"
    );

    let _ = std::fs::remove_dir_all(&workspace_path);
}

// 2026-07-18 (W3a-3): Cancel convergence test — stale cancel should NOT emit
// the terminal event. When the session is already Idle (state_updated == false),
// the convergence fallback must not fire.
#[tokio::test]
async fn cancel_convergence_stale_cancel_does_not_emit() {
    let (coordinator, session_manager) = build_isolated_coordinator();
    let workspace_path =
        std::env::temp_dir().join(format!("northhing-cancel-convergence-stale-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");

    // Create a session in Idle state.
    let session = session_manager
        .create_session(
            "CancelConvergenceStale".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let session_id = session.session_id.clone();
    let turn_id = "test-turn-stale".to_string();

    // Simulate a stuck counter but session is Idle (stale cancel scenario).
    let counter = coordinator
        .active_turns_per_session
        .entry(session_id.clone())
        .or_insert_with(|| std::sync::Arc::new(AtomicUsize::new(0)))
        .clone();
    counter.fetch_add(1, Ordering::SeqCst);

    // Cancel — state_updated will be false because session is Idle.
    coordinator
        .cancel_dialog_turn(&session_id, &turn_id)
        .await
        .expect("cancel should succeed");

    // Verify NO DialogTurnCancelled event was emitted for stale cancel.
    let events = coordinator.event_queue.dequeue_batch(100).await;
    let cancelled_event = events.iter().find(|e| {
        matches!(
            &e.event,
            AgenticEvent::DialogTurnCancelled { session_id: sid, turn_id: tid }
                if sid == &session_id && tid == &turn_id
        )
    });
    assert!(
        cancelled_event.is_none(),
        "DialogTurnCancelled event should NOT be emitted for stale cancel"
    );

    let _ = std::fs::remove_dir_all(&workspace_path);
}

// 2026-07-18 (W3a-3): Watchdog test — verifies the watchdog's "still active"
// detection logic. When a turn is still Processing after the timeout, the
// watchdog should detect it as active. This test exercises the same
// state-checking logic used by the watchdog task in finalize_turn.
#[tokio::test]
async fn watchdog_detects_active_turn() {
    let (coordinator, session_manager) = build_isolated_coordinator();
    let workspace_path =
        std::env::temp_dir().join(format!("northhing-watchdog-active-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");

    // Create a session and set it to Processing state.
    let session = session_manager
        .create_session(
            "WatchdogActive".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let session_id = session.session_id.clone();
    let turn_id = "test-turn-watchdog-active".to_string();
    session_manager
        .update_session_state(
            &session_id,
            SessionState::Processing {
                current_turn_id: turn_id.clone(),
                phase: ProcessingPhase::Thinking,
            },
        )
        .await
        .expect("should set processing state");

    // Simulate a stuck turn by incrementing the active_turns_per_session counter.
    let counter = coordinator
        .active_turns_per_session
        .entry(session_id.clone())
        .or_insert_with(|| std::sync::Arc::new(AtomicUsize::new(0)))
        .clone();
    counter.fetch_add(1, Ordering::SeqCst);

    // The watchdog checks: session state is Processing + current_turn_id matches.
    let still_active = session_manager
        .get_session(&session_id)
        .map(|session| {
            matches!(
                &session.state,
                SessionState::Processing { current_turn_id, .. }
                    if current_turn_id == &turn_id
            )
        })
        .unwrap_or(false);
    assert!(still_active, "watchdog should detect turn as active");

    // Now simulate the cancel that the watchdog would trigger.
    coordinator
        .cancel_dialog_turn(&session_id, &turn_id)
        .await
        .expect("cancel should succeed");

    // Verify session state converged to Idle.
    let final_state = session_manager
        .get_session(&session_id)
        .map(|s| s.state)
        .expect("session should exist");
    assert_eq!(
        final_state,
        SessionState::Idle,
        "session should be Idle after watchdog-prompted cancel"
    );

    // Verify DialogTurnCancelled event was emitted.
    let events = coordinator.event_queue.dequeue_batch(100).await;
    let cancelled_event = events.iter().find(|e| {
        matches!(
            &e.event,
            AgenticEvent::DialogTurnCancelled { session_id: sid, turn_id: tid }
                if sid == &session_id && tid == &turn_id
        )
    });
    assert!(
        cancelled_event.is_some(),
        "DialogTurnCancelled event should be emitted by cancel"
    );

    let _ = std::fs::remove_dir_all(&workspace_path);
}

// 2026-07-18 (W3a-3): Watchdog test — verifies that when a turn completes
// (session state becomes Idle) before the watchdog timeout, the watchdog's
// "still active" check returns false and no cancel is triggered.
#[tokio::test]
async fn watchdog_does_not_detect_completed_turn() {
    let (_coordinator, session_manager) = build_isolated_coordinator();
    let workspace_path =
        std::env::temp_dir().join(format!("watchdog-completed-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir should exist");

    // Create a session and set it to Processing state.
    let session = session_manager
        .create_session(
            "WatchdogCompleted".to_string(),
            "agentic".to_string(),
            SessionConfig {
                workspace_path: Some(workspace_path.to_string_lossy().into_owned()),
                ..Default::default()
            },
        )
        .await
        .expect("session should be created");
    let session_id = session.session_id.clone();
    let turn_id = "test-turn-watchdog-completed".to_string();
    session_manager
        .update_session_state(
            &session_id,
            SessionState::Processing {
                current_turn_id: turn_id.clone(),
                phase: ProcessingPhase::Thinking,
            },
        )
        .await
        .expect("should set processing state");

    // Simulate turn completing: transition to Idle.
    session_manager
        .update_session_state(&session_id, SessionState::Idle)
        .await
        .expect("should set idle state");

    // The watchdog checks: session state is Processing + current_turn_id matches.
    let still_active = session_manager
        .get_session(&session_id)
        .map(|s| {
            matches!(
                &s.state,
                SessionState::Processing { current_turn_id, .. }
                    if current_turn_id == &turn_id
            )
        })
        .unwrap_or(false);
    assert!(
        !still_active,
        "watchdog should NOT detect completed turn as active"
    );

    let _ = std::fs::remove_dir_all(&workspace_path);
}
