//! Submission Images contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[test]
fn remote_connect_submission_contract_preserves_relay_source_and_turn_id() {
    let request = build_remote_submission_request(
        "session-1",
        "hello from phone",
        Some("turn-1".to_string()),
        RemoteConnectSubmissionSource::Relay,
    );

    assert_eq!(request.session_id, "session-1");
    assert_eq!(request.message, "hello from phone");
    assert_eq!(request.turn_id.as_deref(), Some("turn-1"));
    assert_eq!(request.source, Some(AgentSubmissionSource::RemoteRelay));
    assert!(request.attachments.is_empty());
}

#[test]
fn remote_connect_submission_contract_preserves_bot_source() {
    let request =
        build_remote_submission_request("session-2", "hello from bot", None, RemoteConnectSubmissionSource::Bot);

    assert_eq!(request.source, Some(AgentSubmissionSource::Bot));
    assert!(request.turn_id.is_none());
}

#[test]
fn remote_connect_image_attachment_contract_preserves_portable_metadata() {
    let image = ImageAttachment {
        name: "clip.png".to_string(),
        data_url: "data:image/png;base64,abc".to_string(),
    };

    let attachment = build_remote_image_attachment(1, &image);
    let json = serde_json::to_value(attachment).expect("serialize image attachment");

    assert_eq!(json["kind"], "remote_image");
    assert_eq!(json["id"], "remote-image-2");
    assert_eq!(json["metadata"]["name"], "clip.png");
    assert_eq!(json["metadata"]["dataUrl"], "data:image/png;base64,abc");
}

#[test]
fn remote_connect_image_submission_request_preserves_existing_source_and_turn_shape() {
    let image = ImageAttachment {
        name: "clip.png".to_string(),
        data_url: "data:image/png;base64,abc".to_string(),
    };

    let request = build_remote_image_submission_request(
        "session-3",
        "hello with image",
        Some("turn-3".to_string()),
        RemoteConnectSubmissionSource::Relay,
        &[image],
    );

    assert_eq!(request.session_id, "session-3");
    assert_eq!(request.message, "hello with image");
    assert_eq!(request.turn_id.as_deref(), Some("turn-3"));
    assert_eq!(request.source, Some(AgentSubmissionSource::RemoteRelay));
    assert_eq!(request.attachments.len(), 1);
    assert_eq!(request.attachments[0].kind, "remote_image");
    assert_eq!(request.attachments[0].id, "remote-image-1");
    assert_eq!(request.attachments[0].metadata["dataUrl"], "data:image/png;base64,abc");
}

#[test]
fn remote_connect_image_context_policy_preserves_legacy_fallback_shape() {
    let images = vec![
        ImageAttachment {
            name: "clip.png".to_string(),
            data_url: "data:image/png;base64,abc".to_string(),
        },
        ImageAttachment {
            name: "raw".to_string(),
            data_url: "not-a-data-url".to_string(),
        },
    ];

    let contexts = build_remote_image_contexts(Some(&images));

    assert_eq!(contexts.len(), 2);
    assert!(contexts[0].id.starts_with("remote_img_"));
    assert_eq!(contexts[0].image_path, None);
    assert_eq!(contexts[0].data_url.as_deref(), Some("data:image/png;base64,abc"));
    assert_eq!(contexts[0].mime_type, "image/png");
    assert_eq!(contexts[0].metadata.as_ref().unwrap()["name"], "clip.png");
    assert_eq!(contexts[0].metadata.as_ref().unwrap()["source"], "remote");
    assert_eq!(contexts[1].mime_type, "image/png");
}

#[test]
fn remote_connect_image_context_policy_prefers_explicit_contexts() {
    let legacy_images = vec![ImageAttachment {
        name: "legacy.png".to_string(),
        data_url: "data:image/png;base64,legacy".to_string(),
    }];
    let explicit = RemoteImageContext {
        id: "ctx-1".to_string(),
        image_path: Some("D:/workspace/project/screenshot.png".to_string()),
        data_url: None,
        mime_type: "image/png".to_string(),
        metadata: Some(serde_json::json!({ "source": "desktop" })),
    };

    let contexts = resolve_remote_execution_image_contexts(
        Some(&legacy_images),
        Some(vec![explicit.clone()]),
        build_remote_image_contexts,
    );

    assert_eq!(contexts, vec![explicit]);
}

#[test]
fn remote_connect_image_context_adapter_owns_portable_conversion_shape() {
    let context = RemoteImageContext {
        id: "ctx-1".to_string(),
        image_path: Some("D:/workspace/project/screenshot.png".to_string()),
        data_url: Some("data:image/png;base64,abc".to_string()),
        mime_type: "image/png".to_string(),
        metadata: Some(serde_json::json!({ "source": "remote" })),
    };

    let adapted = TestImageContext::from_remote_image_context(context);

    assert_eq!(adapted.id, "ctx-1");
    assert_eq!(
        adapted.image_path.as_deref(),
        Some("D:/workspace/project/screenshot.png")
    );
    assert_eq!(adapted.data_url.as_deref(), Some("data:image/png;base64,abc"));
    assert_eq!(adapted.mime_type, "image/png");
    assert_eq!(adapted.metadata.as_ref().unwrap()["source"], "remote");
}

#[test]
fn remote_chat_history_assembly_preserves_message_shape_and_item_order() {
    let turn = remote_history_contract_turn(false);

    let messages = build_remote_chat_messages(vec![turn]);

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].id, "user-1");
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content, "original question");
    assert_eq!(messages[0].timestamp, "1");
    assert_eq!(
        messages[0].images.as_ref().unwrap()[0],
        ChatImageAttachment {
            name: "screenshot.png".to_string(),
            data_url: "data:image/png;base64,abcd".to_string(),
        }
    );

    assert_eq!(messages[1].id, "turn-1_assistant");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[1].content, "visible text");
    assert_eq!(messages[1].timestamp, "1");
    assert_eq!(messages[1].thinking.as_deref(), Some("visible thought"));
    let items = messages[1].items.as_ref().expect("assistant items");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].item_type, "thinking");
    assert_eq!(items[1].item_type, "text");
    assert_eq!(items[2].item_type, "tool");
    let tool = items[2].tool.as_ref().expect("tool item");
    assert_eq!(tool.name, "AskUserQuestion");
    assert_eq!(tool.status, "running");
    assert_eq!(tool.duration_ms, Some(25));
    assert_eq!(tool.input_preview.as_deref(), Some(r#"{"question":"confirm?"}"#));
    assert_eq!(tool.tool_input.as_ref().unwrap()["question"], "confirm?");
}

#[test]
fn remote_chat_history_assembly_skips_in_progress_assistant_history() {
    let turn = remote_history_contract_turn(true);

    let messages = build_remote_chat_messages(vec![turn]);

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "user");
}
