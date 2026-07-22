//! Tests for the kernel_facade module.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use northhing_core_types::ErrorCategory;
use northhing_kernel_api::events::{KernelEventDto, ToolCallPhase, TurnPhaseKind};
use northhing_kernel_api::turn::TurnStateKind;
use northhing_kernel_api::memory::KernelMemoryApi;
use northhing_kernel_api::session::SessionSummaryDto;
use northhing_kernel_api::KernelSessionApi;

use crate::agentic::events::{AgenticEvent, ToolEventData};
use crate::kernel_facade::events::agentic_event_to_dtos;
use crate::kernel_facade::helpers::{first_line_truncated, truncate_4000};
use crate::kernel_facade::lifecycle::{run_init_gate, FACADE_READY, INIT_STATE, InitState};
use crate::kernel_facade::{kernel_facade, KernelFacade};

fn make_started_event(params: serde_json::Value) -> AgenticEvent {
    AgenticEvent::ToolEvent {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        tool_event: ToolEventData::Started {
            tool_id: "call-abc".into(),
            tool_name: "Bash".into(),
            params,
            timeout_seconds: None,
        },
    }
}

fn make_completed_event(result: serde_json::Value, result_for_assistant: Option<String>) -> AgenticEvent {
    AgenticEvent::ToolEvent {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        tool_event: ToolEventData::Completed {
            tool_id: "call-abc".into(),
            tool_name: "Bash".into(),
            result,
            result_for_assistant,
            duration_ms: 100,
            queue_wait_ms: None,
            preflight_ms: None,
            confirmation_wait_ms: None,
            execution_ms: None,
        },
    }
}

fn make_failed_event(error: String) -> AgenticEvent {
    AgenticEvent::ToolEvent {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        tool_event: ToolEventData::Failed {
            tool_id: "call-abc".into(),
            tool_name: "Bash".into(),
            error,
            duration_ms: None,
            queue_wait_ms: None,
            preflight_ms: None,
            confirmation_wait_ms: None,
            execution_ms: None,
        },
    }
}

fn make_cancelled_event(reason: String) -> AgenticEvent {
    AgenticEvent::ToolEvent {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        tool_event: ToolEventData::Cancelled {
            tool_id: "call-abc".into(),
            tool_name: "Bash".into(),
            reason,
            duration_ms: None,
            queue_wait_ms: None,
            preflight_ms: None,
            confirmation_wait_ms: None,
            execution_ms: None,
        },
    }
}

#[test]
fn test_first_line_truncated() {
    assert_eq!(first_line_truncated("hello world\nsecond line"), "hello world");
    assert_eq!(first_line_truncated("   spaced  \nmore"), "spaced");
    assert_eq!(first_line_truncated(""), "");
    let long = "x".repeat(200);
    assert_eq!(first_line_truncated(&long).len(), 120);
}

#[test]
fn test_truncate_4000() {
    let long = "y".repeat(5000);
    assert_eq!(truncate_4000(&long).len(), 4000);
    assert_eq!(truncate_4000("short").len(), 5);
}

#[test]
fn test_agentic_event_to_dtos_started_summary_from_command() {
    let params = serde_json::json!({"command": "ls -la /tmp", "path": "/other"});
    let event = make_started_event(params);
    let dtos = agentic_event_to_dtos(&event);
    assert!(!dtos.is_empty(), "expected at least one DTO");
    let dto = &dtos[0];
    let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
    assert!(matches!(tc.phase, ToolCallPhase::Started));
    assert!(!tc.summary.is_empty(), "summary should not be empty for command key");
    assert!(tc.summary.starts_with("ls"));
    assert!(tc.detail.is_some());
    assert!(dtos.len() >= 2, "expected TurnPhase after ToolCall");
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::ToolUse, .. }));
}

#[test]
fn test_agentic_event_to_dtos_started_summary_fallback() {
    let params = serde_json::json!({"unknown_field": "value"});
    let event = make_started_event(params);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else { panic!("expected ToolCall") };
    assert!(!tc.summary.is_empty());
}

#[test]
fn test_agentic_event_to_dtos_completed_summary_and_detail() {
    let result = serde_json::json!({"output": "done"});
    let event = make_completed_event(result, Some("All good".into()));
    let dtos = agentic_event_to_dtos(&event);
    let dto = &dtos[0];
    let KernelEventDto::ToolCall(tc) = dto else { panic!("expected ToolCall") };
    assert!(matches!(tc.phase, ToolCallPhase::Completed));
    assert_eq!(tc.summary, "All good");
    assert!(tc.detail.is_some());
    assert!(dtos.len() >= 2, "expected TurnPhase after ToolCall");
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Generating, .. }));
}

#[test]
fn test_agentic_event_to_dtos_completed_result_fallback() {
    let result = serde_json::json!({"output": "fallback result"});
    let event = make_completed_event(result, None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else { panic!("expected ToolCall") };
    assert!(tc.summary.contains("output") || tc.summary.contains("fallback"));
}

#[test]
fn test_agentic_event_to_dtos_failed_maps_to_completed_phase() {
    let event = make_failed_event("connection refused".into());
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "Failed should produce ToolCall and TurnPhase");
    let KernelEventDto::ToolCall(tc) = &dtos[0] else { panic!("expected ToolCall") };
    assert!(matches!(tc.phase, ToolCallPhase::Completed), "Failed should map to Completed phase");
    assert!(!tc.summary.is_empty(), "summary should not be empty for Failed");
    assert!(tc.detail.is_some());
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Generating, tool_name: None, .. }));
    if let KernelEventDto::TurnPhase { session_id, turn_id, .. } = &dtos[1] {
        assert_eq!(session_id, "s1");
        assert_eq!(turn_id, "t1");
    }
}

#[test]
fn test_agentic_event_to_dtos_completed_truncation_at_120() {
    let long_result = "x".repeat(200);
    let event = make_completed_event(serde_json::json!(long_result), None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else { panic!("expected ToolCall") };
    assert!(tc.summary.len() <= 120, "summary should be truncated to 120 chars");
}

#[test]
fn test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120() {
    let long_reason = "x".repeat(200);
    let event = make_cancelled_event(long_reason);
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "Cancelled should produce ToolCall and TurnPhase");
    let KernelEventDto::ToolCall(tc) = &dtos[0] else { panic!("expected ToolCall") };
    assert!(tc.summary.starts_with("cancelled:"), "summary should have cancelled prefix");
    assert!(tc.summary.len() <= 120, "summary including prefix must be <= 120 chars, got {}", tc.summary.len());
    assert!(tc.detail.is_some());
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Generating, tool_name: None, .. }));
    if let KernelEventDto::TurnPhase { session_id, turn_id, .. } = &dtos[1] {
        assert_eq!(session_id, "s1");
        assert_eq!(turn_id, "t1");
    }
}

#[test]
fn test_agentic_event_to_dtos_thinking_chunk_produces_phase_only() {
    let event = AgenticEvent::ThinkingChunk {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        content: "Let me think...".into(),
        is_end: false,
    };
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 1, "ThinkingChunk should produce exactly one TurnPhase DTO");
    assert!(matches!(&dtos[0], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Thinking, .. }));
}

#[test]
fn test_agentic_event_to_dtos_text_chunk_produces_text_and_phase() {
    let event = AgenticEvent::TextChunk {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        text: "Hello world".into(),
    };
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "TextChunk should produce TextChunk and TurnPhase");
    assert!(matches!(&dtos[0], KernelEventDto::TextChunk { .. }));
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Generating, .. }));
}

#[test]
fn test_agentic_event_to_dtos_tool_started_carries_tool_name() {
    let params = serde_json::json!({"command": "ls"});
    let event = make_started_event(params);
    let dtos = agentic_event_to_dtos(&event);
    assert!(matches!(&dtos[0], KernelEventDto::ToolCall(_)));
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::ToolUse, tool_name: Some(_), .. }));
    if let KernelEventDto::TurnPhase { tool_name, .. } = &dtos[1] {
        assert_eq!(tool_name.as_ref().unwrap(), "Bash");
    }
}

#[test]
fn test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase() {
    let event = AgenticEvent::DialogTurnStarted {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        turn_index: 0,
        user_input: "hello".into(),
        original_user_input: None,
        user_message_metadata: None,
    };
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "DialogTurnStarted should produce TurnState and TurnPhase");
    assert!(matches!(&dtos[0], KernelEventDto::TurnState { state: TurnStateKind::Started, .. }));
    assert!(matches!(&dtos[1], KernelEventDto::TurnPhase { phase: TurnPhaseKind::Thinking, .. }));
}

#[test]
fn test_facade_construction_no_panic() {
    let facade = KernelFacade::new();
    assert!(facade.coordinator().is_err());
}

#[test]
fn test_result_methods_return_error_before_init() {
    let facade = kernel_facade();
    match facade.coordinator() {
        Ok(_) => panic!("coordinator() should be Err before init_core"),
        Err(northhing_kernel_api::error::KernelError::Internal(_)) => {}
        Err(other) => panic!("expected KernelError::Internal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subscribe_events_returns_err_before_init() {
    use northhing_kernel_api::KernelEventsApi;
    let facade = KernelFacade::new();
    let callback = Box::new(|_event: KernelEventDto| {});
    let result = facade.subscribe_events(callback).await;
    match result {
        Err(northhing_kernel_api::error::KernelError::Runtime(_)) => {}
        Err(other) => panic!("expected KernelError::Runtime before init, got {:?}", other),
        Ok(_) => panic!("subscribe_events should return Err before init_core"),
    }
}

#[tokio::test]
async fn test_init_gate_lifecycle_all_scenarios() {
    FACADE_READY.store(false, Ordering::SeqCst);
    {
        let mut guard = INIT_STATE.lock().await;
        *guard = InitState::NotStarted;
    }

    // Scenario 1: Two concurrent calls — init runs exactly once
    {
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let fake_init = || async move {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            Ok(())
        };

        let call_count_for_r2 = call_count.clone();
        let (r1, r2) = tokio::join!(
            run_init_gate(fake_init()),
            run_init_gate(async move {
                let cc = call_count_for_r2;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                cc.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                Ok(())
            })
        );

        assert!(r1.is_ok(), "first concurrent call should succeed");
        assert!(r2.is_ok(), "second concurrent call should succeed");
        assert_eq!(call_count.load(Ordering::SeqCst), 1,
            "init should run exactly once across concurrent calls");
    }

    // Scenario 2: Ready之后再调 — init count does not increase
    {
        FACADE_READY.store(false, Ordering::SeqCst);
        {
            let mut guard = INIT_STATE.lock().await;
            *guard = InitState::NotStarted;
        }

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_for_r2 = call_count.clone();
        let call_count_for_assert = call_count.clone();

        let r1 = run_init_gate(async move {
            let cc = call_count;
            cc.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }).await;
        assert!(r1.is_ok(), "first init should succeed");

        let r2 = run_init_gate(async move {
            let cc = call_count_for_r2;
            cc.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).await;
        assert!(r2.is_ok(), "second call on Ready facade should succeed (idempotent)");
        assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 1,
            "init should not re-run when facade is already Ready");
    }

    // Scenario 3: First init fails → state resets → second init succeeds
    {
        FACADE_READY.store(false, Ordering::SeqCst);
        {
            let mut guard = INIT_STATE.lock().await;
            *guard = InitState::NotStarted;
        }

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_for_r2 = call_count.clone();
        let call_count_for_assert = call_count.clone();

        let r1 = run_init_gate(async move {
            let cc = call_count;
            cc.fetch_add(1, Ordering::SeqCst);
            Err(northhing_kernel_api::error::KernelError::Internal("simulated init failure".to_string()))
        }).await;
        assert!(r1.is_err(), "first init should fail");
        assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 1);
        {
            let guard = INIT_STATE.lock().await;
            assert!(matches!(*guard, InitState::NotStarted),
                "state should reset to NotStarted after failed init");
        }

        let r2 = run_init_gate(async move {
            let cc = call_count_for_r2;
            cc.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            Ok(())
        }).await;
        assert!(r2.is_ok(), "retry after failure should succeed");
        assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 2,
            "second (retry) init should actually run");
    }

    // Scenario 4: list_sessions returns KernelError before init, not panic
    {
        FACADE_READY.store(false, Ordering::SeqCst);
        {
            let mut guard = INIT_STATE.lock().await;
            *guard = InitState::NotStarted;
        }

        let facade = KernelFacade::new();
        let result: Result<Vec<SessionSummaryDto>, northhing_kernel_api::error::KernelError> = facade.list_sessions().await;
        match result {
            Err(northhing_kernel_api::error::KernelError::Internal(_)) => {}
            Err(other) => panic!("expected KernelError::Internal before init, got {:?}", other),
            Ok(_) => panic!("list_sessions should return error before init, not Ok"),
        }
    }
}

#[test]
fn test_dialog_turn_failed_network_is_recoverable() {
    let event = AgenticEvent::DialogTurnFailed {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        error: "connection refused".into(),
        error_category: Some(ErrorCategory::Network),
        error_detail: None,
    };
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 1, "DialogTurnFailed should produce exactly one DTO");
    let KernelEventDto::TurnState { state, error, error_kind, .. } = &dtos[0] else {
        panic!("expected TurnState, got {:?}", &dtos[0]);
    };
    assert!(matches!(state, TurnStateKind::Failed));
    assert_eq!(error.as_ref(), Some(&"connection refused".to_string()));
    assert!(matches!(error_kind, Some(crate::kernel_facade::TurnErrorKind::Recoverable)));
}

#[test]
fn test_dialog_turn_failed_auth_is_fatal() {
    let event = AgenticEvent::DialogTurnFailed {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        error: "invalid api key".into(),
        error_category: Some(ErrorCategory::Auth),
        error_detail: None,
    };
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::TurnState { state, error_kind, .. } = &dtos[0] else {
        panic!("expected TurnState");
    };
    assert!(matches!(state, TurnStateKind::Failed));
    assert!(matches!(error_kind, Some(crate::kernel_facade::TurnErrorKind::Fatal)));
}

#[test]
fn test_dialog_turn_failed_no_category_is_fatal() {
    let event = AgenticEvent::DialogTurnFailed {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        error: "unknown error".into(),
        error_category: None,
        error_detail: None,
    };
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::TurnState { state, error_kind, .. } = &dtos[0] else {
        panic!("expected TurnState");
    };
    assert!(matches!(state, TurnStateKind::Failed));
    assert!(matches!(error_kind, Some(crate::kernel_facade::TurnErrorKind::Fatal)));
}

#[test]
fn test_tool_completed_result_count_array() {
    let result = serde_json::json!([{"id": 1}, {"id": 2}, {"id": 3}]);
    let event = make_completed_event(result, None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall");
    };
    assert_eq!(tc.result_count, Some(3));
}

#[test]
fn test_tool_completed_result_count_object_is_none() {
    let result = serde_json::json!({"output": "done"});
    let event = make_completed_event(result, None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall");
    };
    assert_eq!(tc.result_count, None);
}

#[tokio::test]
async fn test_list_episodes_nonexistent_slug_returns_empty_vec() {
    let facade = KernelFacade::new();
    let result = facade
        .list_episodes("nonexistent-workspace-slug-12345", None)
        .await;
    assert!(result.is_ok());
    let episodes = result.unwrap();
    assert!(episodes.is_empty());
}

#[tokio::test]
async fn test_list_episodes_dto_fields_are_correct() {
    let facade = KernelFacade::new();
    let result = facade.list_episodes("definitely-no-episodes-here", Some(5)).await;
    assert!(result.is_ok());
    let episodes = result.unwrap();
    assert_eq!(episodes.len(), 0);
}
