//! Dialog Cancel Contracts contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[test]
fn remote_connect_cancel_and_restore_policy_preserve_runtime_decisions() {
    assert_eq!(
        remote_session_restore_target(false, Some("D:/workspace/project")),
        Some("D:/workspace/project")
    );
    assert_eq!(remote_session_restore_target(true, Some("D:/workspace/project")), None);
    assert_eq!(remote_session_restore_target(false, None), None);

    assert_eq!(
        resolve_remote_cancel_decision(Some("turn-current"), Some("turn-current")),
        RemoteCancelDecision::CancelCurrent("turn-current".to_string())
    );
    assert_eq!(
        resolve_remote_cancel_decision(Some("turn-current"), None),
        RemoteCancelDecision::CancelCurrent("turn-current".to_string())
    );
    assert_eq!(
        resolve_remote_cancel_decision(Some("turn-current"), Some("turn-stale")),
        RemoteCancelDecision::StaleRequestedTurn
    );
    assert_eq!(
        resolve_remote_cancel_decision(None, Some("turn-finished")),
        RemoteCancelDecision::AlreadyFinished
    );
    assert_eq!(
        resolve_remote_cancel_decision(None, None),
        RemoteCancelDecision::NoRunningTask
    );
}

#[tokio::test]
async fn remote_connect_dialog_runtime_owns_restore_prewarm_and_submit_order() {
    let host = RecordingDialogHost::new(false, Some("D:/workspace/project"));

    let outcome = submit_remote_dialog(
        &host,
        RemoteDialogSubmissionRequest {
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            agent_type: Some("code".to_string()),
            image_contexts: vec!["image-1".to_string()],
            policy: RemoteDialogSubmissionPolicy::for_source(RemoteConnectSubmissionSource::Relay),
            turn_id: None,
        },
    )
    .await
    .expect("dialog submit succeeds");

    assert_eq!(
        outcome,
        RemoteDialogSubmitOutcome::Started {
            session_id: "session-1".to_string(),
            turn_id: "turn-generated".to_string()
        }
    );
    assert_eq!(
        host.events(),
        vec![
            "ensure_tracker:session-1",
            "resolve_workspace:session-1",
            "session_exists:session-1",
            "restore:session-1:D:/workspace/project",
            "prewarm:session-1:D:/workspace/project",
            "generate_turn",
            "submit:session-1",
        ]
    );

    let submitted = host.submitted();
    assert_eq!(submitted.session_id, "session-1");
    assert_eq!(submitted.content, "hello");
    assert_eq!(submitted.resolved_agent_type, "agentic");
    assert_eq!(submitted.binding_workspace.as_deref(), Some("D:/workspace/project"));
    assert_eq!(submitted.image_contexts, vec!["image-1".to_string()]);
    assert_eq!(submitted.turn_id, "turn-generated");
    assert_eq!(submitted.policy.source, RemoteConnectSubmissionSource::Relay);
    assert_eq!(submitted.policy.queue_priority, RemoteDialogQueuePriority::Normal);
    assert!(submitted.policy.skip_tool_confirmation);
}

#[tokio::test]
async fn remote_connect_dialog_runtime_preserves_explicit_turn_without_restore() {
    let host = RecordingDialogHost::new(true, Some("D:/workspace/project")).with_submit_outcome(
        RemoteDialogSubmitOutcome::Queued {
            session_id: "session-1".to_string(),
            turn_id: "turn-bot".to_string(),
        },
    );

    let outcome = submit_remote_dialog(
        &host,
        RemoteDialogSubmissionRequest {
            session_id: "session-1".to_string(),
            content: "from bot".to_string(),
            agent_type: Some("Cowork".to_string()),
            image_contexts: Vec::new(),
            policy: RemoteDialogSubmissionPolicy::for_source(RemoteConnectSubmissionSource::Bot),
            turn_id: Some("turn-bot".to_string()),
        },
    )
    .await
    .expect("dialog submit succeeds");

    assert_eq!(
        outcome,
        RemoteDialogSubmitOutcome::Queued {
            session_id: "session-1".to_string(),
            turn_id: "turn-bot".to_string()
        }
    );
    assert_eq!(
        host.events(),
        vec![
            "ensure_tracker:session-1",
            "resolve_workspace:session-1",
            "session_exists:session-1",
            "prewarm:session-1:D:/workspace/project",
            "submit:session-1",
        ]
    );

    let submitted = host.submitted();
    assert_eq!(submitted.resolved_agent_type, "Cowork");
    assert_eq!(submitted.turn_id, "turn-bot");
    assert_eq!(submitted.policy.source, RemoteConnectSubmissionSource::Bot);
}

#[test]
fn remote_connect_dialog_submit_outcome_builder_preserves_scheduler_shape() {
    assert_eq!(
        remote_dialog_submit_outcome_from_scheduler(RemoteDialogSchedulerOutcomeFact::Started {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
        }),
        RemoteDialogSubmitOutcome::Started {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
        }
    );
    assert_eq!(
        remote_dialog_submit_outcome_from_scheduler(RemoteDialogSchedulerOutcomeFact::Queued {
            session_id: "session-2".to_string(),
            turn_id: "turn-2".to_string(),
        }),
        RemoteDialogSubmitOutcome::Queued {
            session_id: "session-2".to_string(),
            turn_id: "turn-2".to_string(),
        }
    );
}

#[tokio::test]
async fn remote_connect_dialog_runtime_keeps_legacy_restore_failure_tolerance() {
    let host = RecordingDialogHost::new(false, Some("D:/workspace/project")).with_restore_error();

    submit_remote_dialog(
        &host,
        RemoteDialogSubmissionRequest {
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            agent_type: None,
            image_contexts: Vec::new(),
            policy: RemoteDialogSubmissionPolicy::for_source(RemoteConnectSubmissionSource::Relay),
            turn_id: Some("turn-1".to_string()),
        },
    )
    .await
    .expect("restore failure is still tolerated before scheduler submit");

    assert_eq!(
        host.events(),
        vec![
            "ensure_tracker:session-1",
            "resolve_workspace:session-1",
            "session_exists:session-1",
            "restore:session-1:D:/workspace/project",
            "prewarm:session-1:D:/workspace/project",
            "submit:session-1",
        ]
    );
    assert_eq!(host.submitted().turn_id, "turn-1");
}

#[tokio::test]
async fn remote_connect_cancel_runtime_restores_missing_session_before_cancel() {
    let host = RecordingCancelHost::new(
        None,
        Some(remote_state(
            "session-1",
            RemoteControlSessionState::Processing,
            Some("turn-current"),
        )),
        Some("D:/workspace/project"),
    );

    cancel_remote_task(
        &host,
        RemoteCancelTaskRequest {
            session_id: "session-1".to_string(),
            requested_turn_id: None,
        },
    )
    .await
    .expect("cancel succeeds after restore");

    assert_eq!(
        host.events(),
        vec![
            "read_state:session-1",
            "resolve_workspace:session-1",
            "restore:session-1:D:/workspace/project",
            "read_state:session-1",
            "cancel:session-1:turn-current",
        ]
    );
}

#[tokio::test]
async fn remote_connect_cancel_runtime_preserves_stale_and_idle_errors_without_restore() {
    let stale_host = RecordingCancelHost::new(
        Some(remote_state(
            "session-1",
            RemoteControlSessionState::Processing,
            Some("turn-current"),
        )),
        None,
        Some("D:/workspace/project"),
    );
    let err = cancel_remote_task(
        &stale_host,
        RemoteCancelTaskRequest {
            session_id: "session-1".to_string(),
            requested_turn_id: Some("turn-stale".to_string()),
        },
    )
    .await
    .expect_err("stale turn is rejected");
    assert_eq!(err, "This task is no longer running.");
    assert_eq!(stale_host.events(), vec!["read_state:session-1"]);

    let idle_host = RecordingCancelHost::new(
        Some(remote_state("session-2", RemoteControlSessionState::Idle, None)),
        None,
        Some("D:/workspace/project"),
    );
    let err = cancel_remote_task(
        &idle_host,
        RemoteCancelTaskRequest {
            session_id: "session-2".to_string(),
            requested_turn_id: None,
        },
    )
    .await
    .expect_err("idle session has no running turn");
    assert_eq!(err, "No running task to cancel for session: session-2");
    assert_eq!(idle_host.events(), vec!["read_state:session-2"]);
}

#[tokio::test]
async fn remote_connect_cancel_runtime_preserves_restore_failure_error() {
    let host = RecordingCancelHost::new(None, None, Some("D:/workspace/project")).with_restore_error();

    let err = cancel_remote_task(
        &host,
        RemoteCancelTaskRequest {
            session_id: "session-1".to_string(),
            requested_turn_id: Some("turn-current".to_string()),
        },
    )
    .await
    .expect_err("restore error is propagated with legacy prefix");

    assert_eq!(err, "Session not found: restore failed");
    assert_eq!(
        host.events(),
        vec![
            "read_state:session-1",
            "resolve_workspace:session-1",
            "restore:session-1:D:/workspace/project",
        ]
    );
}
