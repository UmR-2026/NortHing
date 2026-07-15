//! Command Runtime contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[tokio::test]
async fn remote_connect_command_owner_routes_send_message_and_prefers_explicit_images() {
    let host = RecordingCommandHost::default();

    let response = handle_remote_command(
        &host,
        &RemoteCommand::SendMessage {
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            agent_type: Some("code".to_string()),
            images: Some(vec![ImageAttachment {
                name: "legacy.png".to_string(),
                data_url: "data:image/png;base64,legacy".to_string(),
            }]),
            image_contexts: Some(vec![RemoteImageContext {
                id: "ctx-1".to_string(),
                image_path: Some("D:/workspace/project/screenshot.png".to_string()),
                data_url: None,
                mime_type: "image/png".to_string(),
                metadata: Some(serde_json::json!({ "source": "desktop" })),
            }]),
        },
        RemoteConnectSubmissionSource::Bot,
    )
    .await;

    assert_eq!(
        response,
        RemoteResponse::MessageSent {
            session_id: "session-1".to_string(),
            turn_id: "turn-command".to_string()
        }
    );
    assert_eq!(host.events(), vec!["submit"]);
    assert_eq!(
        host.explicit_context_ids.lock().unwrap().as_slice(),
        &["ctx-1".to_string()]
    );
    assert!(host.legacy_image_names.lock().unwrap().is_empty());

    let submitted = host.submitted_dialog();
    assert_eq!(submitted.session_id, "session-1");
    assert_eq!(submitted.content, "hello");
    assert_eq!(submitted.agent_type.as_deref(), Some("code"));
    assert_eq!(submitted.image_contexts, vec!["explicit:ctx-1".to_string()]);
    assert_eq!(submitted.policy.source, RemoteConnectSubmissionSource::Bot);
    assert!(submitted.turn_id.is_none());
}

#[tokio::test]
async fn remote_connect_command_owner_preserves_cancel_and_group_routing() {
    let host = RecordingCommandHost::default();

    assert_eq!(
        handle_remote_command(&host, &RemoteCommand::Ping, RemoteConnectSubmissionSource::Relay).await,
        RemoteResponse::Pong
    );

    let workspace = handle_remote_command(
        &host,
        &RemoteCommand::GetWorkspaceInfo,
        RemoteConnectSubmissionSource::Relay,
    )
    .await;
    assert!(matches!(workspace, RemoteResponse::WorkspaceInfo { .. }));

    let file = handle_remote_command(
        &host,
        &RemoteCommand::GetFileInfo {
            path: "README.md".to_string(),
            session_id: None,
        },
        RemoteConnectSubmissionSource::Relay,
    )
    .await;
    assert!(matches!(file, RemoteResponse::FileInfo { .. }));

    let interaction = handle_remote_command(
        &host,
        &RemoteCommand::ConfirmTool {
            tool_id: "tool-1".to_string(),
            updated_input: None,
        },
        RemoteConnectSubmissionSource::Relay,
    )
    .await;
    assert!(matches!(interaction, RemoteResponse::InteractionAccepted { .. }));

    let cancel = handle_remote_command(
        &host,
        &RemoteCommand::CancelTask {
            session_id: "session-1".to_string(),
            turn_id: Some("turn-1".to_string()),
        },
        RemoteConnectSubmissionSource::Relay,
    )
    .await;
    assert_eq!(
        cancel,
        RemoteResponse::TaskCancelled {
            session_id: "session-1".to_string()
        }
    );
    assert_eq!(host.events(), vec!["workspace", "file", "interaction", "cancel"]);
    assert_eq!(
        host.cancel_request(),
        RemoteCancelTaskRequest {
            session_id: "session-1".to_string(),
            requested_turn_id: Some("turn-1".to_string()),
        }
    );
}
