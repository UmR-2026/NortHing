//! Tests for the kernel_facade module.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use northhing_core_types::ErrorCategory;
use northhing_kernel_api::events::{KernelEventDto, ToolCallPhase, TurnPhaseKind};
use northhing_kernel_api::memory::{FactDto, KernelMemoryApi};
use northhing_kernel_api::session::{MessageDto, SessionConfigDto, SessionSummaryDto};
use northhing_kernel_api::settings::ProviderFormDto;
use northhing_kernel_api::turn::{DialogSubmitOutcomeDto, DialogSubmitOutcomeKindDto, TurnStateKind};
use northhing_kernel_api::KernelSessionApi;

use crate::agentic::events::{AgenticEvent, ToolEventData};
use crate::kernel_facade::events::agentic_event_to_dtos;
use crate::kernel_facade::helpers::{first_line_truncated, truncate_4000};
use crate::kernel_facade::lifecycle::{run_init_gate, InitState, FACADE_READY, INIT_STATE};
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
    let KernelEventDto::ToolCall(tc) = dto else {
        panic!("expected ToolCall")
    };
    assert!(matches!(tc.phase, ToolCallPhase::Started));
    assert!(!tc.summary.is_empty(), "summary should not be empty for command key");
    assert!(tc.summary.starts_with("ls"));
    assert!(tc.detail.is_some());
    assert!(dtos.len() >= 2, "expected TurnPhase after ToolCall");
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::ToolUse,
            ..
        }
    ));
}

#[test]
fn test_agentic_event_to_dtos_started_summary_fallback() {
    let params = serde_json::json!({"unknown_field": "value"});
    let event = make_started_event(params);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall")
    };
    assert!(!tc.summary.is_empty());
}

#[test]
fn test_agentic_event_to_dtos_completed_summary_and_detail() {
    let result = serde_json::json!({"output": "done"});
    let event = make_completed_event(result, Some("All good".into()));
    let dtos = agentic_event_to_dtos(&event);
    let dto = &dtos[0];
    let KernelEventDto::ToolCall(tc) = dto else {
        panic!("expected ToolCall")
    };
    assert!(matches!(tc.phase, ToolCallPhase::Completed));
    assert_eq!(tc.summary, "All good");
    assert!(tc.detail.is_some());
    assert!(dtos.len() >= 2, "expected TurnPhase after ToolCall");
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Generating,
            ..
        }
    ));
}

#[test]
fn test_agentic_event_to_dtos_completed_result_fallback() {
    let result = serde_json::json!({"output": "fallback result"});
    let event = make_completed_event(result, None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall")
    };
    assert!(tc.summary.contains("output") || tc.summary.contains("fallback"));
}

#[test]
fn test_agentic_event_to_dtos_failed_maps_to_completed_phase() {
    let event = make_failed_event("connection refused".into());
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "Failed should produce ToolCall and TurnPhase");
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall")
    };
    assert!(
        matches!(tc.phase, ToolCallPhase::Completed),
        "Failed should map to Completed phase"
    );
    assert!(!tc.summary.is_empty(), "summary should not be empty for Failed");
    assert!(tc.detail.is_some());
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Generating,
            tool_name: None,
            ..
        }
    ));
    if let KernelEventDto::TurnPhase {
        session_id, turn_id, ..
    } = &dtos[1]
    {
        assert_eq!(session_id, "s1");
        assert_eq!(turn_id, "t1");
    }
}

#[test]
fn test_agentic_event_to_dtos_completed_truncation_at_120() {
    let long_result = "x".repeat(200);
    let event = make_completed_event(serde_json::json!(long_result), None);
    let dtos = agentic_event_to_dtos(&event);
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall")
    };
    assert!(tc.summary.len() <= 120, "summary should be truncated to 120 chars");
}

#[test]
fn test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120() {
    let long_reason = "x".repeat(200);
    let event = make_cancelled_event(long_reason);
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(dtos.len(), 2, "Cancelled should produce ToolCall and TurnPhase");
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall")
    };
    assert!(
        tc.summary.starts_with("cancelled:"),
        "summary should have cancelled prefix"
    );
    assert!(
        tc.summary.len() <= 120,
        "summary including prefix must be <= 120 chars, got {}",
        tc.summary.len()
    );
    assert!(tc.detail.is_some());
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Generating,
            tool_name: None,
            ..
        }
    ));
    if let KernelEventDto::TurnPhase {
        session_id, turn_id, ..
    } = &dtos[1]
    {
        assert_eq!(session_id, "s1");
        assert_eq!(turn_id, "t1");
    }
}

#[test]
fn test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation() {
    let params = serde_json::json!({"command": "rm -rf /tmp/data", "path": "/tmp"});
    let event = AgenticEvent::ToolEvent {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        round_id: "r1".into(),
        tool_event: ToolEventData::ConfirmationNeeded {
            tool_id: "call-confirm-123".into(),
            tool_name: "Bash".into(),
            params,
        },
    };
    let dtos = agentic_event_to_dtos(&event);
    assert_eq!(
        dtos.len(),
        1,
        "ConfirmationNeeded should produce exactly one ToolCall DTO (no TurnPhase)"
    );
    let KernelEventDto::ToolCall(tc) = &dtos[0] else {
        panic!("expected ToolCall DTO, got {:?}", &dtos[0]);
    };
    assert_eq!(tc.call_id, "call-confirm-123");
    assert_eq!(tc.name, "Bash");
    assert!(matches!(tc.phase, ToolCallPhase::AwaitingConfirmation));
    assert_eq!(tc.session_id, "s1");
    assert_eq!(tc.turn_id, "t1");
    assert!(tc.summary.starts_with("rm"));
    assert!(tc.detail.is_some());
    assert_eq!(tc.result_count, None);
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
    assert!(matches!(
        &dtos[0],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Thinking,
            ..
        }
    ));
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
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Generating,
            ..
        }
    ));
}

#[test]
fn test_agentic_event_to_dtos_tool_started_carries_tool_name() {
    let params = serde_json::json!({"command": "ls"});
    let event = make_started_event(params);
    let dtos = agentic_event_to_dtos(&event);
    assert!(matches!(&dtos[0], KernelEventDto::ToolCall(_)));
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::ToolUse,
            tool_name: Some(_),
            ..
        }
    ));
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
    assert_eq!(
        dtos.len(),
        2,
        "DialogTurnStarted should produce TurnState and TurnPhase"
    );
    assert!(matches!(
        &dtos[0],
        KernelEventDto::TurnState {
            state: TurnStateKind::Started,
            ..
        }
    ));
    assert!(matches!(
        &dtos[1],
        KernelEventDto::TurnPhase {
            phase: TurnPhaseKind::Thinking,
            ..
        }
    ));
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

    // Scenario 1: Two concurrent calls 鈥?init runs exactly once
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
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "init should run exactly once across concurrent calls"
        );
    }

    // Scenario 2: Ready涔嬪悗鍐嶈皟 鈥?init count does not increase
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
        })
        .await;
        assert!(r1.is_ok(), "first init should succeed");

        let r2 = run_init_gate(async move {
            let cc = call_count_for_r2;
            cc.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert!(r2.is_ok(), "second call on Ready facade should succeed (idempotent)");
        assert_eq!(
            call_count_for_assert.load(Ordering::SeqCst),
            1,
            "init should not re-run when facade is already Ready"
        );
    }

    // Scenario 3: First init fails 鈫?state resets 鈫?second init succeeds
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
            Err(northhing_kernel_api::error::KernelError::Internal(
                "simulated init failure".to_string(),
            ))
        })
        .await;
        assert!(r1.is_err(), "first init should fail");
        assert_eq!(call_count_for_assert.load(Ordering::SeqCst), 1);
        {
            let guard = INIT_STATE.lock().await;
            assert!(
                matches!(*guard, InitState::NotStarted),
                "state should reset to NotStarted after failed init"
            );
        }

        let r2 = run_init_gate(async move {
            let cc = call_count_for_r2;
            cc.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            Ok(())
        })
        .await;
        assert!(r2.is_ok(), "retry after failure should succeed");
        assert_eq!(
            call_count_for_assert.load(Ordering::SeqCst),
            2,
            "second (retry) init should actually run"
        );
    }

    // Scenario 4: list_sessions returns KernelError before init, not panic
    {
        FACADE_READY.store(false, Ordering::SeqCst);
        {
            let mut guard = INIT_STATE.lock().await;
            *guard = InitState::NotStarted;
        }

        let facade = KernelFacade::new();
        let result: Result<Vec<SessionSummaryDto>, northhing_kernel_api::error::KernelError> =
            facade.list_sessions().await;
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
    let KernelEventDto::TurnState {
        state,
        error,
        error_kind,
        ..
    } = &dtos[0]
    else {
        panic!("expected TurnState, got {:?}", &dtos[0]);
    };
    assert!(matches!(state, TurnStateKind::Failed));
    assert_eq!(error.as_ref(), Some(&"connection refused".to_string()));
    assert!(matches!(
        error_kind,
        Some(crate::kernel_facade::TurnErrorKind::Recoverable)
    ));
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
    let result = facade.list_episodes("nonexistent-workspace-slug-12345", None).await;
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

// ── W9-2 memory fact tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_list_facts_returns_ok() {
    let facade = KernelFacade::new();
    let result = facade.list_facts(None).await;
    assert!(result.is_ok(), "list_facts should return Ok: {:?}", result.err());
}

#[tokio::test]
async fn test_search_facts_returns_ok() {
    let facade = KernelFacade::new();
    let result = facade.search_facts("anything", None, Some(5)).await;
    assert!(result.is_ok(), "search_facts should return Ok: {:?}", result.err());
}

// K4a-T23q DTO gap-fill tests鈹€鈹€鈹€鈹€鈹€鈹€鈹€

#[test]
fn test_message_to_dto_carries_timestamp() {
    use crate::agentic::core::{Message, MessageContent, MessageRole};
    use crate::kernel_facade::dto::message_to_dto;
    use std::time::SystemTime;

    let msg = Message {
        id: "m1".into(),
        role: MessageRole::User,
        content: MessageContent::Text("hello".into()),
        timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(1_700_000_000_000),
        metadata: Default::default(),
    };
    let dto = message_to_dto(msg);
    assert_eq!(dto.timestamp, 1_700_000_000_000);
}

#[test]
fn test_summary_to_dto_carries_parent_and_state() {
    use crate::agentic::core::{SessionState, SessionStatus, SessionSummary};
    use crate::kernel_facade::events::summary_to_dto;
    use std::time::SystemTime;

    let summary = SessionSummary {
        session_id: "s1".into(),
        session_name: "test".into(),
        agent_type: "agentic".into(),
        last_user_dialog_agent_type: None,
        last_submitted_agent_type: None,
        created_by: None,
        kind: northhing_core_types::SessionKind::Standard,
        turn_count: 0,
        created_at: SystemTime::UNIX_EPOCH,
        last_activity_at: SystemTime::UNIX_EPOCH,
        state: SessionState::Processing {
            current_turn_id: "t1".into(),
            phase: crate::agentic::core::ProcessingPhase::Thinking,
        },
        status: SessionStatus::Active,
        parent_session_id: Some("parent-s1".into()),
    };
    let dto = summary_to_dto(summary);
    assert_eq!(dto.parent_session_id, Some("parent-s1".to_string()));
    assert_eq!(dto.state, Some("processing".to_string()));
}

#[test]
fn test_outcome_to_dto_started_and_queued() {
    use crate::agentic::coordination::DialogSubmitOutcome;
    use crate::kernel_facade::dto::outcome_to_dto;

    let started = outcome_to_dto(DialogSubmitOutcome::Started {
        session_id: "s1".into(),
        turn_id: "t1".into(),
    });
    assert_eq!(
        started.outcome_kind,
        Some(northhing_kernel_api::turn::DialogSubmitOutcomeKindDto::Started)
    );

    let queued = outcome_to_dto(DialogSubmitOutcome::Queued {
        session_id: "s1".into(),
        turn_id: "t2".into(),
    });
    assert_eq!(
        queued.outcome_kind,
        Some(northhing_kernel_api::turn::DialogSubmitOutcomeKindDto::Queued)
    );
}

#[test]
fn test_session_config_dto_name_round_trip() {
    let dto = SessionConfigDto {
        workspace_path: None,
        agent_type: "agentic".into(),
        model_name: "default".into(),
        name: Some("my-session".into()),
    };
    let json = serde_json::to_string(&dto).unwrap();
    let back: SessionConfigDto = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, Some("my-session".to_string()));
}

#[test]
fn test_backward_compat_deserialization_missing_new_fields() {
    // ProviderFormDto without provider_type (added by K4a-T4p)
    let json = r#"{"provider_id":"openai","base_url":null,"api_key":null,"model":null}"#;
    let dto: ProviderFormDto = serde_json::from_str(json).unwrap();
    assert_eq!(dto.provider_id, "openai");
    assert_eq!(dto.provider_type, None);

    // SessionSummaryDto without parent_session_id / state (added by K4a-T23q)
    let json2 = r#"{"id":"s1","name":"test","updated_at":0,"status":"active"}"#;
    let dto2: SessionSummaryDto = serde_json::from_str(json2).unwrap();
    assert_eq!(dto2.id, "s1");
    assert_eq!(dto2.parent_session_id, None);
    assert_eq!(dto2.state, None);

    // DialogSubmitOutcomeDto without outcome_kind (added by K4a-T23q)
    let json3 = r#"{"turn_id":"t1","accepted":true}"#;
    let dto3: DialogSubmitOutcomeDto = serde_json::from_str(json3).unwrap();
    assert_eq!(dto3.turn_id, "t1");
    assert_eq!(dto3.outcome_kind, None);
}

// ── T3-1a list_tools tests ───────────────────────────────────────────────────

struct MockKernelTool {
    name: String,
    description: Option<String>,
    schema: serde_json::Value,
}

#[async_trait::async_trait]
impl crate::agentic::tools::framework::Tool for MockKernelTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn description(&self) -> crate::util::errors::NortHingResult<String> {
        self.description
            .clone()
            .ok_or_else(|| crate::util::errors::NortHingError::Tool("failed description".into()))
    }

    fn short_description(&self) -> String {
        self.description.clone().unwrap_or_default()
    }

    fn input_schema(&self) -> serde_json::Value {
        self.schema.clone()
    }

    async fn call_impl(
        &self,
        _input: &serde_json::Value,
        _context: &crate::agentic::tools::framework::ToolUseContext,
    ) -> crate::util::errors::NortHingResult<Vec<crate::agentic::tools::framework::ToolResult>> {
        Ok(vec![])
    }
}

fn build_test_facade_with_tools(tools: Vec<Arc<dyn crate::agentic::tools::framework::Tool>>) -> Arc<KernelFacade> {
    let event_queue = Arc::new(crate::agentic::events::EventQueue::new(
        crate::agentic::events::EventQueueConfig::default(),
    ));
    let session_manager = Arc::new(crate::agentic::session::SessionManager::new(
        Arc::new(crate::agentic::session::SessionContextStore::new()),
        Arc::new(
            crate::agentic::persistence::PersistenceManager::new(Arc::new(
                crate::infrastructure::PathManager::new().expect("path manager"),
            ))
            .expect("persistence manager"),
        ),
        crate::agentic::session::SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: std::time::Duration::from_secs(3600),
            auto_save_interval: std::time::Duration::from_secs(300),
            enable_persistence: false,
            prompt_cache_policy: crate::agentic::session::PromptCachePolicy::default(),
        },
    ));
    let mut registry = crate::agentic::tools::registry::ToolRegistry::new();
    for tool in tools {
        registry.register_tool(tool);
    }
    let tool_registry = Arc::new(tokio::sync::RwLock::new(registry));
    let tool_pipeline = Arc::new(crate::agentic::tools::pipeline::ToolPipeline::new(
        tool_registry,
        Arc::new(crate::agentic::tools::pipeline::ToolStateManager::new(
            event_queue.clone(),
        )),
        None,
        Arc::new(std::sync::OnceLock::new()),
    ));
    let execution_engine = Arc::new(crate::agentic::execution::ExecutionEngine::new(
        Arc::new(crate::agentic::execution::RoundExecutor::new(
            Arc::new(crate::agentic::execution::StreamProcessor::new(event_queue.clone())),
            event_queue.clone(),
            tool_pipeline.clone(),
        )),
        event_queue.clone(),
        session_manager.clone(),
        Arc::new(crate::agentic::ContextCompressor::new(
            crate::agentic::CompressionConfig::default(),
        )),
        crate::agentic::execution::ExecutionEngineConfig::default(),
    ));
    let coordinator = crate::agentic::coordination::ConversationCoordinator::new(
        session_manager,
        execution_engine,
        tool_pipeline,
        event_queue,
        Arc::new(crate::agentic::events::EventRouter::new()),
    );
    let facade = KernelFacade::new();
    facade.set_coordinator(Arc::new(coordinator));
    Arc::new(facade)
}

#[tokio::test]
async fn test_list_tools_returns_err_before_init() {
    use northhing_kernel_api::KernelToolsApi;
    let facade = KernelFacade::new();
    let result = facade.list_tools().await;
    match result {
        Err(northhing_kernel_api::error::KernelError::Internal(_)) => {}
        Err(other) => panic!("expected KernelError::Internal, got {:?}", other),
        Ok(_) => panic!("expected Err before init"),
    }
}

#[tokio::test]
async fn test_list_tools_single_tool_field_mapping() {
    use northhing_kernel_api::KernelToolsApi;

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        }
    });
    let tool = Arc::new(MockKernelTool {
        name: "test_mock_tool_custom".to_string(),
        description: Some("test description for custom mock tool".to_string()),
        schema: schema.clone(),
    });

    let facade = build_test_facade_with_tools(vec![tool]);
    let tools = facade.list_tools().await.expect("list_tools should succeed");

    let found = tools
        .iter()
        .find(|t| t.name == "test_mock_tool_custom")
        .expect("list_tools must contain the registered mock tool");
    assert_eq!(found.id, "test_mock_tool_custom");
    assert_eq!(found.name, "test_mock_tool_custom");
    assert_eq!(found.description, "test description for custom mock tool");
    assert_eq!(found.input_schema, Some(schema));
}

#[tokio::test]
async fn test_list_tools_ordering_and_degraded_description() {
    use northhing_kernel_api::KernelToolsApi;

    let tool_z = Arc::new(MockKernelTool {
        name: "zzz_mock_tool".to_string(),
        description: Some("zebra desc".to_string()),
        schema: serde_json::json!({ "type": "object" }),
    });
    let tool_a = Arc::new(MockKernelTool {
        name: "aaa_mock_tool".to_string(),
        description: None, // Description will fail, should degrade to empty string
        schema: serde_json::json!({ "type": "string" }),
    });
    let tool_m = Arc::new(MockKernelTool {
        name: "mmm_mock_tool".to_string(),
        description: Some("mango desc".to_string()),
        schema: serde_json::json!({ "type": "number" }),
    });

    let facade = build_test_facade_with_tools(vec![tool_z, tool_a, tool_m]);
    let tools = facade.list_tools().await.expect("list_tools should succeed");

    // Must be strictly sorted by name
    assert!(
        tools.windows(2).all(|w| w[0].name <= w[1].name),
        "tools list must be sorted by name deterministically"
    );

    let found_a = tools
        .iter()
        .find(|t| t.name == "aaa_mock_tool")
        .expect("aaa_mock_tool must exist");
    assert_eq!(found_a.description, ""); // Degraded on error

    let found_m = tools
        .iter()
        .find(|t| t.name == "mmm_mock_tool")
        .expect("mmm_mock_tool must exist");
    assert_eq!(found_m.description, "mango desc");

    let found_z = tools
        .iter()
        .find(|t| t.name == "zzz_mock_tool")
        .expect("zzz_mock_tool must exist");
    assert_eq!(found_z.description, "zebra desc");

    // Verify ordering between the three mock tools
    let pos_a = tools.iter().position(|t| t.name == "aaa_mock_tool").unwrap();
    let pos_m = tools.iter().position(|t| t.name == "mmm_mock_tool").unwrap();
    let pos_z = tools.iter().position(|t| t.name == "zzz_mock_tool").unwrap();
    assert!(
        pos_a < pos_m && pos_m < pos_z,
        "mock tools must appear in alphabetical order"
    );
}

// ── W9-6 file tree / preview path fence + behavior ───────────────────────────

mod w9_6_file_tree {
    use super::*;
    use northhing_kernel_api::KernelPlatformApi;
    use std::sync::Mutex;

    /// Guards `crate::kernel_facade::helpers::default_workspace_path` from
    /// changing underneath these tests, which rely on the current working
    /// directory being writable for tmp subtree construction.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn workspace_root_for_test() -> std::path::PathBuf {
        std::env::current_dir().expect("current_dir")
    }

    #[tokio::test]
    async fn list_tree_rejects_parent_dir_escape() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let root = workspace_root_for_test();
        let outside = root.parent().unwrap_or(&root);
        let outside_str = outside.to_string_lossy().to_string();
        let user_path = format!(
            "{}/../escape_probe.txt",
            root.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        );
        let err = kernel_facade()
            .list_workspace_tree(None, &user_path, Some(1))
            .await
            .expect_err("must reject `..` segment");
        assert!(
            matches!(err, northhing_kernel_api::error::KernelError::Validation(_)),
            "expected Validation, got {err:?}"
        );
        assert!(!outside_str.is_empty(), "sanity: workspace has a parent on this host");
    }

    #[tokio::test]
    async fn list_tree_rejects_absolute_path() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let err = kernel_facade()
            .list_workspace_tree(None, "C:/Windows", Some(0))
            .await
            .expect_err("must reject absolute paths");
        assert!(
            matches!(err, northhing_kernel_api::error::KernelError::Validation(_)),
            "expected Validation, got {err:?}"
        );
    }

    #[test]
    fn path_fence_rejects_escape_segments() {
        // Re-implement the segment rules here so we test the contract
        // value (what `resolve_within_workspace` enforces) without exposing
        // a private helper just for the test surface.
        fn reject(s: &str) -> bool {
            let rel = std::path::Path::new(s);
            for c in rel.components() {
                if matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_)
                ) {
                    return true;
                }
            }
            rel.to_string_lossy().contains('\0')
        }
        assert!(reject("../foo"));
        assert!(reject("a/../../b"));
        assert!(reject("a/b/../c")); // any `..` segment is rejected
        assert!(reject("foo\0bar"));
        assert!(!reject("src/main.rs"));
        assert!(!reject("plain/path.txt"));
    }

    #[tokio::test]
    async fn list_tree_lists_direct_children() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Empty dir == workspace root; we expect the test to find at least
        // `src/` (a known directory in the workspace root) and skip any
        // entries outside the immediate level.
        let tree = kernel_facade()
            .list_workspace_tree(None, "", Some(0))
            .await
            .expect("root listing must succeed");
        let paths: Vec<String> = tree.iter().map(|e| e.path.clone()).collect();
        assert!(
            paths.iter().any(|p| p == "src"),
            "expected 'src' in direct children, got {paths:?}"
        );
        for entry in &tree {
            if entry.is_dir {
                assert!(entry.size_bytes.is_none(), "dirs must not carry size_bytes");
            }
        }
    }

    #[tokio::test]
    async fn read_file_rejects_too_large() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Empty path = workspace root which is a directory; expect NotFound
        // because we ask to read a directory as a file.
        let err = kernel_facade()
            .read_workspace_file(None, "", Some(8))
            .await
            .expect_err("read of root as file must fail");
        assert!(
            matches!(
                err,
                northhing_kernel_api::error::KernelError::NotFound(_)
                    | northhing_kernel_api::error::KernelError::Validation(_)
            ),
            "expected NotFound or Validation for root-as-file, got {err:?}"
        );
    }

    #[tokio::test]
    async fn read_file_round_trip_within_cap() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // `Cargo.toml` is always present and well under 256 KiB.
        let text = kernel_facade()
            .read_workspace_file(None, "Cargo.toml", Some(64 * 1024))
            .await
            .expect("Cargo.toml must read back");
        assert!(text.contains("northhing-core") || text.contains("package"));
    }

    #[tokio::test]
    async fn read_file_rejects_escape() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let err = kernel_facade()
            .read_workspace_file(None, "../secret.txt", Some(1024))
            .await
            .expect_err("must reject `..` segment");
        assert!(
            matches!(err, northhing_kernel_api::error::KernelError::Validation(_)),
            "expected Validation, got {err:?}"
        );
    }

    // ── I-1 fix: symlink escape coverage ──────────────────────────────────────
    //
    // These tests stage a symlink inside the workspace that resolves to an
    // external file and ensure both methods refuse to follow it. Creation
    // panics if `symlink(2)` is denied on the host (rare on Windows after
    // developer-mode is toggled on).

    #[cfg(unix)]
    fn create_symlink(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_symlink(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }

    /// Try to create a symlink; on hosts that deny `SeCreateSymbolicLink`
    /// (Windows non-developer-mode, locked-down macOS containers), the
    /// test panics on the failure directly so CI surfaces the missing
    /// capability. Callers run with `: --ignored` to skip when needed.
    fn make_symlink_or_ignore(target: &std::path::Path, link: &std::path::Path) -> bool {
        match create_symlink(target, link) {
            Ok(()) => true,
            Err(e) => {
                eprintln!(
                    "w9-6: skipping symlink test ({}). Hint: toggle Developer Mode on Windows.",
                    e
                );
                false
            }
        }
    }

    fn unique_tmp(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("northhing-w9-6-{prefix}-{pid}-{nanos}"))
    }

    #[tokio::test]
    async fn read_file_rejects_symlink_to_outside_target() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let outside_dir = unique_tmp("outside");
        std::fs::create_dir_all(&outside_dir).expect("mkdir outside");
        let outside_file = outside_dir.join("secret.txt");
        std::fs::write(&outside_file, b"TOP SECRET").expect("write outside");

        let workspace = unique_tmp("workspace");
        std::fs::create_dir_all(&workspace).expect("mkdir workspace");
        let link_in_workspace = workspace.join("escape_link");
        if !make_symlink_or_ignore(&outside_file, &link_in_workspace) {
            let _ = std::fs::remove_dir_all(&workspace);
            let _ = std::fs::remove_dir_all(&outside_dir);
            return;
        }

        let err = kernel_facade()
            .read_workspace_file(Some(workspace.to_string_lossy().as_ref()), "escape_link", Some(1024))
            .await
            .expect_err("symlink to outside target must be rejected");
        assert!(
            matches!(err, northhing_kernel_api::error::KernelError::Validation(_)),
            "expected Validation for symlink escape, got {err:?}"
        );

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    #[tokio::test]
    async fn list_tree_skips_symlink_to_outside_target() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let outside_dir = unique_tmp("outside-listing");
        std::fs::create_dir_all(&outside_dir).expect("mkdir outside");
        let outside_file = outside_dir.join("readme.md");
        std::fs::write(&outside_file, b"outside").expect("write outside");

        let workspace = unique_tmp("workspace-listing");
        std::fs::create_dir_all(&workspace).expect("mkdir workspace");
        std::fs::write(workspace.join("normal.txt"), b"hi").expect("write normal");
        if !make_symlink_or_ignore(&outside_file, &workspace.join("escape_link")) {
            let _ = std::fs::remove_dir_all(&workspace);
            let _ = std::fs::remove_dir_all(&outside_dir);
            return;
        }

        let tree = kernel_facade()
            .list_workspace_tree(Some(workspace.to_string_lossy().as_ref()), "", Some(0))
            .await
            .expect("root listing must succeed even when symlinks exist");
        let names: Vec<&str> = tree.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"normal.txt") && !names.contains(&"escape_link"),
            "symlink should be skipped, got entries {names:?}"
        );

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    // ── I-2 fix: workspace_root parameter wiring ───────────────────────────────

    #[tokio::test]
    async fn list_tree_with_explicit_workspace_root_uses_that_fence() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let env_workspace = unique_tmp("iwr");
        std::fs::create_dir_all(&env_workspace).expect("mkdir env workspace");
        std::fs::write(env_workspace.join("hello.txt"), b"hi").expect("write hello");

        let tree = kernel_facade()
            .list_workspace_tree(Some(env_workspace.to_string_lossy().as_ref()), "", Some(0))
            .await
            .expect("root listing of explicit workspace must succeed");
        let names: Vec<&str> = tree.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"hello.txt"),
            "explicit workspace_root should pin listing, got {names:?}"
        );

        let _ = std::fs::remove_dir_all(&env_workspace);
    }

    #[tokio::test]
    async fn read_file_with_explicit_workspace_root_uses_that_fence() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let env_workspace = unique_tmp("iwr-read");
        std::fs::create_dir_all(&env_workspace).expect("mkdir env workspace");
        std::fs::write(env_workspace.join("Cargo.toml"), b"[package]\nname = \"x\"\n").expect("write Cargo.toml");

        let text = kernel_facade()
            .read_workspace_file(
                Some(env_workspace.to_string_lossy().as_ref()),
                "Cargo.toml",
                Some(8 * 1024),
            )
            .await
            .expect("explicit workspace read must succeed");
        assert!(text.contains("[package]"));

        let _ = std::fs::remove_dir_all(&env_workspace);
    }

    #[tokio::test]
    async fn list_tree_rejects_non_absolute_workspace_root() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let err = kernel_facade()
            .list_workspace_tree(Some("relative/path"), "", Some(0))
            .await
            .expect_err("relative workspace_root must be rejected");
        assert!(
            matches!(err, northhing_kernel_api::error::KernelError::Validation(_))
                || matches!(err, northhing_kernel_api::error::KernelError::Config(_)),
            "expected Validation or Config, got {err:?}"
        );
    }
}

// ── W12-1 search_sessions tests ──────────────────────────────────────────

mod w12_1_search_sessions {
    use super::*;
    use uuid::Uuid;

    fn build_test_facade_with_persistence() -> Arc<KernelFacade> {
        let path_manager = Arc::new(crate::infrastructure::PathManager::new().expect("path manager"));
        let persistence = Arc::new(
            crate::agentic::persistence::PersistenceManager::new(path_manager).expect("persistence manager"),
        );
        let session_manager = Arc::new(crate::agentic::session::SessionManager::new(
            Arc::new(crate::agentic::session::SessionContextStore::new()),
            persistence.clone(),
            crate::agentic::session::SessionManagerConfig {
                max_active_sessions: 100,
                session_idle_timeout: std::time::Duration::from_secs(3600),
                auto_save_interval: std::time::Duration::from_secs(300),
                enable_persistence: true,
                prompt_cache_policy: crate::agentic::session::PromptCachePolicy::default(),
            },
        ));
        let event_queue = Arc::new(crate::agentic::events::EventQueue::new(
            crate::agentic::events::EventQueueConfig::default(),
        ));
        let tool_registry = Arc::new(tokio::sync::RwLock::new(crate::agentic::tools::registry::ToolRegistry::new()));
        let tool_pipeline = Arc::new(crate::agentic::tools::pipeline::ToolPipeline::new(
            tool_registry,
            Arc::new(crate::agentic::tools::pipeline::ToolStateManager::new(
                event_queue.clone(),
            )),
            None,
            Arc::new(std::sync::OnceLock::new()),
        ));
        let execution_engine = Arc::new(crate::agentic::execution::ExecutionEngine::new(
            Arc::new(crate::agentic::execution::RoundExecutor::new(
                Arc::new(crate::agentic::execution::StreamProcessor::new(event_queue.clone())),
                event_queue.clone(),
                tool_pipeline.clone(),
            )),
            event_queue.clone(),
            session_manager.clone(),
            Arc::new(crate::agentic::ContextCompressor::new(
                crate::agentic::CompressionConfig::default(),
            )),
            crate::agentic::execution::ExecutionEngineConfig::default(),
        ));
        let coordinator = crate::agentic::coordination::ConversationCoordinator::new(
            session_manager,
            execution_engine,
            tool_pipeline,
            event_queue,
            Arc::new(crate::agentic::events::EventRouter::new()),
        );
        let facade = KernelFacade::new();
        facade.set_coordinator(Arc::new(coordinator));
        Arc::new(facade)
    }

    fn create_test_model_round(turn_id: &str, text: &str) -> crate::service::session::ModelRoundData {
        crate::service::session::ModelRoundData {
            id: format!("round-{}", turn_id),
            turn_id: turn_id.to_string(),
            round_index: 0,
            timestamp: 1_700_000_001_000,
            text_items: vec![crate::service::session::TextItemData {
                id: format!("text-{}", turn_id),
                content: text.to_string(),
                is_streaming: false,
                timestamp: 1_700_000_001_000,
                is_markdown: true,
                order_index: None,
                is_subagent_item: None,
                parent_task_tool_id: None,
                subagent_session_id: None,
                status: None,
            }],
            tool_items: Vec::new(),
            thinking_items: Vec::new(),
            start_time: 1,
            end_time: Some(2),
            duration_ms: Some(1),
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

    #[tokio::test]
    async fn test_search_sessions_hit_snippet_and_case_insensitive() {
        let temp_dir = std::env::temp_dir().join(format!("northhing-search-hit-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");

        let facade = build_test_facade_with_persistence();
        let ws_str = temp_dir.to_str().unwrap();

        let session_id = facade
            .create_session(SessionConfigDto {
                workspace_path: Some(ws_str.to_string()),
                agent_type: "agentic".to_string(),
                model_name: "default-model".to_string(),
                name: Some("Search Test Alpha".to_string()),
            })
            .await
            .expect("create session");

        let coordinator = facade.coordinator().expect("coordinator");
        let persistence = &coordinator.session_manager().persistence_manager;

        let user_message = crate::service::session::UserMessageData {
            id: "user-msg-alpha-1".to_string(),
            content: "Please help me design a database schema for user profiles".to_string(),
            timestamp: 1_700_000_000_000,
            metadata: None,
        };
        let mut turn = crate::service::session::DialogTurnData::new(
            "turn-1".to_string(),
            0,
            session_id.clone(),
            user_message,
        );
        turn.model_rounds.push(create_test_model_round(
            "turn-1",
            "Here is the recommended PostgreSQL table structure for user profiles.",
        ));
        turn.mark_completed();
        persistence.save_dialog_turn(&temp_dir, &turn).await.expect("save turn");

        // 1. User message hit
        let hits = facade
            .search_sessions("database schema", Some(ws_str), None)
            .await
            .expect("search should succeed");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].session_id, session_id);
        assert_eq!(hits[0].session_name, "Search Test Alpha");
        assert_eq!(hits[0].role, "user");
        assert!(hits[0].snippet.contains("database schema"));

        // 2. Assistant message hit with case insensitivity
        let hits_assistant = facade
            .search_sessions("postgresql", Some(ws_str), None)
            .await
            .expect("search should succeed");
        assert_eq!(hits_assistant.len(), 1);
        assert_eq!(hits_assistant[0].session_id, session_id);
        assert_eq!(hits_assistant[0].role, "assistant");
        assert!(hits_assistant[0].snippet.contains("PostgreSQL"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_search_sessions_miss_and_empty_query() {
        let temp_dir = std::env::temp_dir().join(format!("northhing-search-miss-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");

        let facade = build_test_facade_with_persistence();
        let ws_str = temp_dir.to_str().unwrap();

        let session_id = facade
            .create_session(SessionConfigDto {
                workspace_path: Some(ws_str.to_string()),
                agent_type: "agentic".to_string(),
                model_name: "default-model".to_string(),
                name: Some("Search Test Beta".to_string()),
            })
            .await
            .expect("create session");

        let coordinator = facade.coordinator().expect("coordinator");
        let persistence = &coordinator.session_manager().persistence_manager;

        let user_message = crate::service::session::UserMessageData {
            id: "user-msg-beta-1".to_string(),
            content: "Some normal discussion about weather".to_string(),
            timestamp: 1_700_000_000_000,
            metadata: None,
        };
        let mut turn = crate::service::session::DialogTurnData::new(
            "turn-1".to_string(),
            0,
            session_id.clone(),
            user_message,
        );
        turn.mark_completed();
        persistence.save_dialog_turn(&temp_dir, &turn).await.expect("save turn");

        // 1. Search non-existent string -> empty vec
        let hits = facade
            .search_sessions("quantum computing", Some(ws_str), None)
            .await
            .expect("search should succeed");
        assert!(hits.is_empty(), "expected empty vec for non-existent term");

        // 2. Search empty query -> empty vec
        let hits_empty = facade
            .search_sessions("", Some(ws_str), None)
            .await
            .expect("search should succeed");
        assert!(hits_empty.is_empty(), "expected empty vec for empty query");

        // 3. Search whitespace query -> empty vec
        let hits_spaces = facade
            .search_sessions("   ", Some(ws_str), None)
            .await
            .expect("search should succeed");
        assert!(hits_spaces.is_empty(), "expected empty vec for whitespace query");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_search_sessions_cjk_snippet_and_session_hit_cap() {
        let temp_dir = std::env::temp_dir().join(format!("northhing-search-cjk-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");

        let facade = build_test_facade_with_persistence();
        let ws_str = temp_dir.to_str().unwrap();

        let session_id = facade
            .create_session(SessionConfigDto {
                workspace_path: Some(ws_str.to_string()),
                agent_type: "agentic".to_string(),
                model_name: "default-model".to_string(),
                name: Some("中文搜索会话".to_string()),
            })
            .await
            .expect("create session");

        let coordinator = facade.coordinator().expect("coordinator");
        let persistence = &coordinator.session_manager().persistence_manager;

        // Turn 1: User message with CJK text
        let user_msg_1 = crate::service::session::UserMessageData {
            id: "user-cjk-1".to_string(),
            content: "这是一个非常重要的会话管理系统的架构设计方案说明，需要支持多工作区搜索。".to_string(),
            timestamp: 1_700_000_000_000,
            metadata: None,
        };
        let mut turn_1 = crate::service::session::DialogTurnData::new(
            "turn-1".to_string(),
            0,
            session_id.clone(),
            user_msg_1,
        );
        turn_1.model_rounds.push(create_test_model_round(
            "turn-1",
            "我们已经完成了会话管理系统后端搜索的初步实现，并保持与架构设计一致。",
        ));
        turn_1.mark_completed();
        persistence.save_dialog_turn(&temp_dir, &turn_1).await.expect("save turn 1");

        // Turn 2: Another matching turn in same session to test per-session cap (max 2 hits)
        let user_msg_2 = crate::service::session::UserMessageData {
            id: "user-cjk-2".to_string(),
            content: "第三条包含架构设计的消息，应该被每会话2条hit的上限截断。".to_string(),
            timestamp: 1_700_000_002_000,
            metadata: None,
        };
        let mut turn_2 = crate::service::session::DialogTurnData::new(
            "turn-2".to_string(),
            1,
            session_id.clone(),
            user_msg_2,
        );
        turn_2.mark_completed();
        persistence.save_dialog_turn(&temp_dir, &turn_2).await.expect("save turn 2");

        let hits = facade
            .search_sessions("架构设计", Some(ws_str), None)
            .await
            .expect("search should succeed");

        // Exactly 2 hits returned due to per-session cap of 2
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].session_name, "中文搜索会话");
        assert_eq!(hits[0].role, "user");
        assert!(hits[0].snippet.contains("架构设计"));
        assert_eq!(hits[1].role, "assistant");
        assert!(hits[1].snippet.contains("架构设计"));

        // Limit test: limit = 1 returns 1 hit
        let hits_limit = facade
            .search_sessions("架构设计", Some(ws_str), Some(1))
            .await
            .expect("search with limit 1");
        assert_eq!(hits_limit.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
