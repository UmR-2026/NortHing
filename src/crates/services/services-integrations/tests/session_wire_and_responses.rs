//! Session Wire And Responses contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[test]
fn remote_connect_execution_response_helpers_preserve_wire_shape() {
    let started = remote_dialog_submit_response(Ok(RemoteDialogSubmitOutcome::Started {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
    }));
    assert_eq!(
        started,
        RemoteResponse::MessageSent {
            session_id: "session-1".to_string(),
            turn_id: "turn-1".to_string(),
        }
    );

    let queued = remote_dialog_submit_response(Ok(RemoteDialogSubmitOutcome::Queued {
        session_id: "session-1".to_string(),
        turn_id: "turn-2".to_string(),
    }));
    assert_eq!(
        queued,
        RemoteResponse::MessageSent {
            session_id: "session-1".to_string(),
            turn_id: "turn-2".to_string(),
        }
    );

    assert_eq!(
        remote_task_cancel_response("session-1", Ok(())),
        RemoteResponse::TaskCancelled {
            session_id: "session-1".to_string(),
        }
    );
    assert_eq!(
        remote_interaction_accepted_response("confirm_tool", "tool-1", Ok(())),
        RemoteResponse::InteractionAccepted {
            action: "confirm_tool".to_string(),
            target_id: "tool-1".to_string(),
        }
    );
    assert_eq!(remote_answer_question_response(Ok(())), RemoteResponse::AnswerAccepted);
    assert_eq!(
        remote_answer_question_response(Err("question closed".to_string())),
        RemoteResponse::Error {
            message: "question closed".to_string(),
        }
    );
}

#[test]
fn remote_connect_workspace_response_helpers_own_wire_shape() {
    let workspace = RemoteWorkspaceFacts {
        path: "D:/workspace/project".to_string(),
        name: "project".to_string(),
        git_branch: Some("main".to_string()),
        kind: RemoteWorkspaceKind::Remote,
        assistant_id: Some("assistant-1".to_string()),
    };

    let info_json = serde_json::to_value(remote_workspace_info_response(Some(workspace.clone())))
        .expect("serialize workspace info");
    assert_eq!(info_json["resp"], "workspace_info");
    assert_eq!(info_json["has_workspace"], true);
    assert_eq!(info_json["path"], "D:/workspace/project");
    assert_eq!(info_json["project_name"], "project");
    assert_eq!(info_json["git_branch"], "main");
    assert_eq!(info_json["workspace_kind"], "remote");
    assert_eq!(info_json["assistant_id"], "assistant-1");

    let empty_json = serde_json::to_value(remote_workspace_info_response(None)).expect("serialize empty info");
    assert_eq!(empty_json["resp"], "workspace_info");
    assert_eq!(empty_json["has_workspace"], false);
    assert!(empty_json.get("workspace_kind").is_none());

    let recent_json = serde_json::to_value(remote_recent_workspaces_response(vec![RemoteRecentWorkspaceFacts {
        path: workspace.path.clone(),
        name: workspace.name.clone(),
        last_opened: "2026-05-25T00:00:00Z".to_string(),
        kind: workspace.kind,
    }]))
    .expect("serialize recent workspaces");
    assert_eq!(recent_json["resp"], "recent_workspaces");
    assert_eq!(recent_json["workspaces"][0]["workspace_kind"], "remote");
    assert_eq!(recent_json["workspaces"][0]["last_opened"], "2026-05-25T00:00:00Z");

    let assistant_json = serde_json::to_value(remote_assistant_list_response(vec![RemoteAssistantWorkspaceFacts {
        path: "D:/workspace/assistant".to_string(),
        name: "assistant".to_string(),
        assistant_id: Some("assistant-2".to_string()),
    }]))
    .expect("serialize assistant list");
    assert_eq!(assistant_json["resp"], "assistant_list");
    assert_eq!(assistant_json["assistants"][0]["assistant_id"], "assistant-2");

    assert_eq!(
        remote_workspace_updated_response(Ok(RemoteWorkspaceUpdate {
            path: "D:/workspace/project".to_string(),
            name: "project".to_string(),
        })),
        RemoteResponse::WorkspaceUpdated {
            success: true,
            path: Some("D:/workspace/project".to_string()),
            project_name: Some("project".to_string()),
            error: None,
        }
    );
    assert_eq!(
        remote_assistant_updated_response(Err("open failed".to_string())),
        RemoteResponse::AssistantUpdated {
            success: false,
            path: None,
            name: None,
            error: Some("open failed".to_string()),
        }
    );
}

#[test]
fn remote_connect_session_response_helpers_own_pagination_and_timestamps() {
    let metadata = vec![
        RemoteSessionMetadata {
            session_id: "session-1".to_string(),
            name: "first".to_string(),
            agent_type: "agentic".to_string(),
            created_at_ms: 1_700_000_000_000,
            last_active_at_ms: 1_700_000_001_000,
            turn_count: 3,
        },
        RemoteSessionMetadata {
            session_id: "session-2".to_string(),
            name: "second".to_string(),
            agent_type: "Cowork".to_string(),
            created_at_ms: 1_700_000_002_000,
            last_active_at_ms: 1_700_000_003_000,
            turn_count: 5,
        },
        RemoteSessionMetadata {
            session_id: "session-3".to_string(),
            name: "third".to_string(),
            agent_type: "Plan".to_string(),
            created_at_ms: 1_700_000_004_000,
            last_active_at_ms: 1_700_000_005_000,
            turn_count: 8,
        },
    ];

    let session = remote_session_info(&metadata[0], Some("D:/workspace/project"), Some("project"));
    assert_eq!(session.session_id, "session-1");
    assert_eq!(session.created_at, "1700000000");
    assert_eq!(session.updated_at, "1700000001");
    assert_eq!(session.message_count, 3);
    assert_eq!(session.workspace_path.as_deref(), Some("D:/workspace/project"));
    assert_eq!(session.workspace_name.as_deref(), Some("project"));

    let list = remote_session_list_response(metadata.clone(), Some("D:/workspace/project"), Some("project"), 1, 1);
    let list_json = serde_json::to_value(list).expect("serialize session list");
    assert_eq!(list_json["resp"], "session_list");
    assert_eq!(list_json["has_more"], true);
    assert_eq!(list_json["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(list_json["sessions"][0]["session_id"], "session-2");
    assert_eq!(list_json["sessions"][0]["created_at"], "1700000002");

    let initial = remote_initial_sync_response(
        Some(RemoteWorkspaceFacts {
            path: "D:/workspace/project".to_string(),
            name: "project".to_string(),
            git_branch: Some("main".to_string()),
            kind: RemoteWorkspaceKind::Normal,
            assistant_id: None,
        }),
        metadata,
        Some("project"),
        true,
        Some("user-1".to_string()),
    );
    let initial_json = serde_json::to_value(initial).expect("serialize initial sync");
    assert_eq!(initial_json["resp"], "initial_sync");
    assert_eq!(initial_json["has_workspace"], true);
    assert_eq!(initial_json["workspace_kind"], "normal");
    assert_eq!(initial_json["has_more_sessions"], true);
    assert_eq!(initial_json["sessions"].as_array().unwrap().len(), 3);
    assert_eq!(initial_json["authenticated_user_id"], "user-1");

    assert_eq!(
        remote_session_created_response("session-new"),
        RemoteResponse::SessionCreated {
            session_id: "session-new".to_string(),
        }
    );
    assert_eq!(
        remote_session_model_updated_response("session-1", "model-1"),
        RemoteResponse::SessionModelUpdated {
            session_id: "session-1".to_string(),
            model_id: "model-1".to_string(),
        }
    );
    assert_eq!(
        remote_messages_response("session-1", vec![], false),
        RemoteResponse::Messages {
            session_id: "session-1".to_string(),
            messages: vec![],
            has_more: false,
        }
    );
    assert_eq!(
        remote_session_deleted_response("session-1"),
        RemoteResponse::SessionDeleted {
            session_id: "session-1".to_string(),
        }
    );
}

#[test]
fn remote_connect_session_create_contract_preserves_workspace_binding() {
    let request = build_remote_session_create_request(
        "Remote Session",
        "agentic",
        Some("D:/workspace/project"),
        RemoteConnectSubmissionSource::Relay,
    );

    assert_eq!(request.session_name, "Remote Session");
    assert_eq!(request.agent_type, "agentic");
    assert_eq!(request.workspace_path.as_deref(), Some("D:/workspace/project"));
    assert_eq!(request.metadata["source"], "remote_relay");
}

#[test]
fn remote_connect_agent_type_mapping_preserves_current_mobile_aliases() {
    assert_eq!(resolve_remote_agent_type(Some("code")), "agentic");
    assert_eq!(resolve_remote_agent_type(Some("agentic")), "agentic");
    assert_eq!(resolve_remote_agent_type(Some("Agentic")), "agentic");
    assert_eq!(resolve_remote_agent_type(Some("cowork")), "Cowork");
    assert_eq!(resolve_remote_agent_type(Some("Cowork")), "Cowork");
    assert_eq!(resolve_remote_agent_type(Some("plan")), "Plan");
    assert_eq!(resolve_remote_agent_type(Some("Plan")), "Plan");
    assert_eq!(resolve_remote_agent_type(Some("debug")), "debug");
    assert_eq!(resolve_remote_agent_type(Some("Debug")), "debug");
    assert_eq!(resolve_remote_agent_type(Some("unknown")), "agentic");
    assert_eq!(resolve_remote_agent_type(None), "agentic");
}

#[test]
fn remote_connect_message_dtos_keep_current_wire_shape() {
    let image = ImageAttachment {
        name: "clip.png".to_string(),
        data_url: "data:image/png;base64,abc".to_string(),
    };
    let chat = ChatMessage {
        id: "msg-1".to_string(),
        role: "assistant".to_string(),
        content: "done".to_string(),
        timestamp: "1".to_string(),
        metadata: None,
        tools: Some(vec![RemoteToolStatus {
            id: "tool-1".to_string(),
            name: "bash".to_string(),
            status: "running".to_string(),
            duration_ms: None,
            start_ms: Some(42),
            input_preview: Some("{\"cmd\":\"git status\"}".to_string()),
            tool_input: None,
        }]),
        thinking: None,
        items: Some(vec![ChatMessageItem {
            item_type: "tool".to_string(),
            content: None,
            tool: None,
            is_subagent: Some(false),
        }]),
        images: Some(vec![ChatImageAttachment {
            name: image.name.clone(),
            data_url: image.data_url.clone(),
        }]),
    };

    let json = serde_json::to_value(chat).expect("serialize chat message");

    assert_eq!(json["id"], "msg-1");
    assert_eq!(json["tools"][0]["start_ms"], 42);
    assert_eq!(json["items"][0]["type"], "tool");
    assert_eq!(json["images"][0]["data_url"], "data:image/png;base64,abc");
}

#[test]
fn remote_connect_command_wire_shape_lives_in_owner_contract() {
    let command = RemoteCommand::SendMessage {
        session_id: "session-1".to_string(),
        content: "hello".to_string(),
        agent_type: Some("code".to_string()),
        images: Some(vec![ImageAttachment {
            name: "clip.png".to_string(),
            data_url: "data:image/png;base64,abc".to_string(),
        }]),
        image_contexts: Some(vec![RemoteImageContext {
            id: "ctx-1".to_string(),
            image_path: Some("D:/workspace/project/screenshot.png".to_string()),
            data_url: None,
            mime_type: "image/png".to_string(),
            metadata: Some(serde_json::json!({ "source": "remote" })),
        }]),
    };
    let json = serde_json::to_value(command).expect("serialize send command");

    assert_eq!(json["cmd"], "send_message");
    assert_eq!(json["session_id"], "session-1");
    assert_eq!(json["agent_type"], "code");
    assert_eq!(json["images"][0]["name"], "clip.png");
    assert_eq!(json["image_contexts"][0]["id"], "ctx-1");
    assert_eq!(
        json["image_contexts"][0]["image_path"],
        "D:/workspace/project/screenshot.png"
    );
    assert!(json.get("imageContexts").is_none());

    let cancel = serde_json::to_value(RemoteCommand::CancelTask {
        session_id: "session-1".to_string(),
        turn_id: Some("turn-1".to_string()),
    })
    .expect("serialize cancel command");
    assert_eq!(cancel["cmd"], "cancel_task");
    assert_eq!(cancel["turn_id"], "turn-1");

    let list = serde_json::to_value(RemoteCommand::ListSessions {
        workspace_path: Some("/workspace/project".to_string()),
        limit: Some(30),
        offset: Some(0),
        query: Some("alpha".to_string()),
    })
    .expect("serialize list command");
    assert_eq!(list["cmd"], "list_sessions");
    assert_eq!(list["query"], "alpha");

    let rename = serde_json::to_value(RemoteCommand::UpdateSessionTitle {
        session_id: "session-1".to_string(),
        title: "Renamed session".to_string(),
    })
    .expect("serialize rename command");
    assert_eq!(rename["cmd"], "update_session_title");
    assert_eq!(rename["title"], "Renamed session");

    let poll = serde_json::to_value(RemoteCommand::PollSession {
        session_id: "session-1".to_string(),
        since_version: 7,
        known_msg_count: 3,
        known_model_catalog_version: Some(11),
    })
    .expect("serialize poll command");
    assert_eq!(poll["cmd"], "poll_session");
    assert_eq!(poll["since_version"], 7);
    assert_eq!(poll["known_msg_count"], 3);
    assert_eq!(poll["known_model_catalog_version"], 11);
}

#[test]
fn remote_connect_response_wire_shape_lives_in_owner_contract() {
    let active_turn = ActiveTurnSnapshot {
        turn_id: "turn-1".to_string(),
        status: "active".to_string(),
        text: String::new(),
        thinking: String::new(),
        tools: vec![RemoteToolStatus {
            id: "tool-1".to_string(),
            name: "Read".to_string(),
            status: "running".to_string(),
            duration_ms: None,
            start_ms: Some(42),
            input_preview: Some("{\"path\":\"README.md\"}".to_string()),
            tool_input: None,
        }],
        round_index: 2,
        items: Some(vec![ChatMessageItem {
            item_type: "tool".to_string(),
            content: None,
            tool: None,
            is_subagent: None,
        }]),
    };

    let poll = serde_json::to_value(RemoteResponse::SessionPoll {
        version: 8,
        changed: true,
        session_state: Some("running".to_string()),
        title: Some("session title".to_string()),
        new_messages: None,
        total_msg_count: None,
        active_turn: Some(active_turn),
        model_catalog: Box::new(Some(sample_remote_model_catalog(11))),
    })
    .expect("serialize poll response");

    assert_eq!(poll["resp"], "session_poll");
    assert_eq!(poll["version"], 8);
    assert_eq!(poll["active_turn"]["turn_id"], "turn-1");
    assert_eq!(
        poll["active_turn"]["tools"][0]["input_preview"],
        "{\"path\":\"README.md\"}"
    );
    assert_eq!(poll["model_catalog"]["version"], 11);
    assert_eq!(poll["model_catalog"]["default_models"]["primary"], "model-1");
    assert!(poll.get("new_messages").is_none());

    let sent = serde_json::to_value(RemoteResponse::MessageSent {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
    })
    .expect("serialize sent response");
    assert_eq!(sent["resp"], "message_sent");
    assert_eq!(sent["turn_id"], "turn-1");

    let title_updated = serde_json::to_value(RemoteResponse::SessionTitleUpdated {
        session_id: "session-1".to_string(),
        title: "Renamed session".to_string(),
    })
    .expect("serialize title response");
    assert_eq!(title_updated["resp"], "session_title_updated");
    assert_eq!(title_updated["title"], "Renamed session");
}
