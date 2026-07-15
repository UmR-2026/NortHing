//! Turn scheduling and execution port tests.
//!
//! Tests for `AgentTurnCancellationPort`, subagent timeout handling,
//! max-concurrency normalization, and background subagent text formatting.

use super::super::{format_background_subagent_delivery_text, format_background_subagent_display_text, SubagentResult};
use crate::util::errors::NortHingError;
use northhing_runtime_ports::{AgentTurnCancellationPort, DelegationPolicy};
use std::sync::Mutex;
use std::time::Duration;
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
