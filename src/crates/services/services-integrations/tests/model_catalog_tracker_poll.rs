//! Model Catalog Tracker Poll contract tests.

#![cfg(feature = "remote-connect")]

mod common;
use common::*;

#[test]
fn remote_connect_model_catalog_builder_preserves_config_shape() {
    let catalog = build_remote_model_catalog(RemoteModelCatalogFacts {
        last_modified_ms: -7,
        models: vec![RemoteModelFacts {
            id: "model-1".to_string(),
            name: "Model One".to_string(),
            provider: "openai".to_string(),
            base_url: "https://api.example.com".to_string(),
            model_name: "gpt-test".to_string(),
            context_window: Some(128_000),
            enabled: true,
            capabilities: vec![
                RemoteModelCapabilityFact::TextChat,
                RemoteModelCapabilityFact::ImageUnderstanding,
                RemoteModelCapabilityFact::FunctionCalling,
            ],
            enable_thinking_process: true,
            reasoning_mode: Some(RemoteReasoningModeFact::Adaptive),
            reasoning_effort: Some("medium".to_string()),
            thinking_budget_tokens: Some(4096),
        }],
        default_models: RemoteDefaultModelsConfig {
            primary: Some("model-1".to_string()),
            fast: Some("fast-model".to_string()),
            search: Some("search-model".to_string()),
            ..RemoteDefaultModelsConfig::default()
        },
        session_model_id: Some("session-model".to_string()),
    });

    assert_eq!(catalog.version, 0);
    assert_eq!(catalog.session_model_id.as_deref(), Some("session-model"));
    assert_eq!(catalog.default_models.fast.as_deref(), Some("fast-model"));
    let model = catalog.models.first().expect("model config");
    assert_eq!(model.id, "model-1");
    assert_eq!(model.context_window, Some(128_000));
    assert_eq!(
        model.capabilities,
        vec![
            "text_chat".to_string(),
            "image_understanding".to_string(),
            "function_calling".to_string(),
        ]
    );
    assert!(model.enable_thinking_process);
    assert_eq!(model.reasoning_mode.as_deref(), Some("adaptive"));
    assert_eq!(model.reasoning_effort.as_deref(), Some("medium"));
    assert_eq!(model.thinking_budget_tokens, Some(4096));
}

#[test]
fn remote_connect_tracker_registry_owns_lifecycle_without_core_state() {
    let registry = RemoteSessionTrackerRegistry::new();
    let host = RecordingTrackerHost::with_active_turn("turn-1");

    let tracker = registry.ensure_tracker_with_host("session-1", &host);
    assert_eq!(host.subscribed.lock().unwrap().as_slice(), &["session-1".to_string()]);
    assert_eq!(
        tracker.snapshot_active_turn().expect("active turn seeded").turn_id,
        "turn-1"
    );

    let reused = registry.ensure_tracker_with_host("session-1", &host);
    assert!(Arc::ptr_eq(&tracker, &reused));
    assert_eq!(host.subscribed.lock().unwrap().len(), 1);
    assert!(registry.get_tracker("session-1").is_some());

    let removed = registry.remove_tracker_with_host("session-1", &host);
    assert!(removed.is_some());
    assert!(registry.get_tracker("session-1").is_none());
    assert_eq!(host.unsubscribed.lock().unwrap().as_slice(), &["session-1".to_string()]);
}

#[test]
fn remote_connect_tracker_preserves_streaming_snapshot_contract() {
    let tracker = RemoteSessionStateTracker::new("session-1".to_string());

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnStarted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        turn_index: 0,
        user_input: "hello".to_string(),
        original_user_input: None,
        user_message_metadata: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::ModelRoundStarted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        round_index: 3,
        model_id: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::ThinkingChunk {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        content: "<thinking>plan".to_string(),
        is_end: false,
    });
    tracker.handle_agentic_event(&AgenticEvent::TextChunk {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        text: "answer".to_string(),
    });

    let snapshot = tracker.snapshot_active_turn().expect("active turn snapshot");

    assert_eq!(tracker.session_state(), "running");
    assert_eq!(snapshot.turn_id, "turn-1");
    assert_eq!(snapshot.status, "active");
    assert_eq!(snapshot.round_index, 3);
    assert_eq!(snapshot.text, "");
    assert_eq!(snapshot.thinking, "");
    let items = snapshot.items.expect("ordered streaming items");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item_type, "thinking");
    assert_eq!(items[0].content.as_deref(), Some("plan"));
    assert_eq!(items[1].item_type, "text");
    assert_eq!(items[1].content.as_deref(), Some("answer"));
}

#[test]
fn remote_connect_tracker_keeps_subagent_items_out_of_parent_accumulators() {
    let tracker = RemoteSessionStateTracker::new("parent-session".to_string());

    tracker.initialize_active_turn("parent-turn".to_string());
    tracker.handle_agentic_event(&AgenticEvent::SubagentSessionLinked {
        session_id: "child-session".to_string(),
        parent_session_id: "parent-session".to_string(),
        parent_dialog_turn_id: "parent-turn".to_string(),
        parent_tool_call_id: "task-1".to_string(),
        agent_type: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::TextChunk {
        session_id: "child-session".to_string(),
        turn_id: "child-turn".to_string(),
        round_id: "round-1".to_string(),
        text: "child text".to_string(),
    });

    assert_eq!(tracker.accumulated_text(), "");
    let snapshot = tracker.snapshot_active_turn().expect("active turn snapshot");
    let items = snapshot.items.expect("subagent item");
    assert_eq!(items[0].content.as_deref(), Some("child text"));
    assert_eq!(items[0].is_subagent, Some(true));
}

#[tokio::test]
async fn remote_connect_tracker_broadcasts_tool_and_turn_events() {
    let tracker = RemoteSessionStateTracker::new("session-1".to_string());
    let mut events = tracker.subscribe();

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnStarted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        turn_index: 0,
        user_input: "hello".to_string(),
        original_user_input: None,
        user_message_metadata: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::ToolEvent {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        tool_event: ToolEventData::Started {
            tool_id: "tool-1".to_string(),
            tool_name: "AskUserQuestion".to_string(),
            params: serde_json::json!({ "questions": [] }),
            timeout_seconds: None,
        },
    });
    tracker.handle_agentic_event(&AgenticEvent::DialogTurnCancelled {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
    });

    match events.recv().await.expect("tool started event") {
        TrackerEvent::ToolStarted {
            tool_id,
            tool_name,
            params,
        } => {
            assert_eq!(tool_id, "tool-1");
            assert_eq!(tool_name, "AskUserQuestion");
            assert!(params.is_some());
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match events.recv().await.expect("turn cancelled event") {
        TrackerEvent::TurnCancelled { turn_id } => assert_eq!(turn_id, "turn-1"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn remote_connect_tracker_keeps_finished_turn_snapshot_until_persistence_finalizes() {
    let tracker = RemoteSessionStateTracker::new("session-1".to_string());

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnStarted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        turn_index: 0,
        user_input: "hello".to_string(),
        original_user_input: None,
        user_message_metadata: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::TextChunk {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        text: "answer".to_string(),
    });
    tracker.mark_persistence_clean();

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnCompleted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        total_rounds: 1,
        total_tools: 0,
        duration_ms: 42,
        partial_recovery_reason: None,
        success: Some(true),
        finish_reason: Some("stop".to_string()),
    });

    assert_eq!(tracker.session_state(), "idle");
    assert!(tracker.is_turn_finished());
    assert!(tracker.is_persistence_dirty());
    let snapshot = tracker
        .snapshot_active_turn()
        .expect("finished snapshot remains until persistence catches up");
    assert_eq!(snapshot.status, "completed");
    assert_eq!(snapshot.turn_id, "turn-1");

    tracker.finalize_completed_turn();
    assert!(tracker.snapshot_active_turn().is_none());
    assert_eq!(tracker.accumulated_text(), "");
}

#[test]
fn remote_connect_model_catalog_delta_preserves_poll_invalidation_policy() {
    let unchanged = remote_model_catalog_poll_delta(Some(sample_remote_model_catalog(11)), Some(11));
    assert!(!unchanged.changed);
    assert!(unchanged.catalog.is_none());

    let changed = remote_model_catalog_poll_delta(Some(sample_remote_model_catalog(12)), Some(11));
    assert!(changed.changed);
    assert_eq!(changed.catalog.expect("changed catalog").version, 12);

    let initial_catalog = remote_model_catalog_poll_delta(Some(sample_remote_model_catalog(13)), None);
    assert!(initial_catalog.changed);
    assert_eq!(initial_catalog.catalog.expect("initial catalog").version, 13);

    let unavailable_after_known_version = remote_model_catalog_poll_delta(None, Some(11));
    assert!(unavailable_after_known_version.changed);
    assert!(unavailable_after_known_version.catalog.is_none());

    let unavailable_initial = remote_model_catalog_poll_delta(None, None);
    assert!(!unavailable_initial.changed);
    assert!(unavailable_initial.catalog.is_none());
}

#[test]
fn remote_connect_model_selection_policy_owns_alias_and_config_reference_rules() {
    assert_eq!(normalize_remote_session_model_id(None), Some("auto".to_string()));
    assert_eq!(
        normalize_remote_session_model_id(Some("  default  ")),
        Some("auto".to_string())
    );
    assert_eq!(
        normalize_remote_session_model_id(Some(" model-1 ")),
        Some("model-1".to_string())
    );

    assert!(!remote_model_selection_needs_config("auto"));
    assert!(!remote_model_selection_needs_config("default"));
    assert!(!remote_model_selection_needs_config("primary"));
    assert!(!remote_model_selection_needs_config("fast"));
    assert!(remote_model_selection_needs_config("custom-alias"));

    assert_eq!(normalize_remote_model_selection("default", |_| None).unwrap(), "auto");
    assert_eq!(
        normalize_remote_model_selection("primary", |_| None).unwrap(),
        "primary"
    );
    assert_eq!(
        normalize_remote_model_selection("custom-alias", |id| {
            (id == "custom-alias").then(|| "model-1".to_string())
        })
        .unwrap(),
        "model-1"
    );
    assert_eq!(
        normalize_remote_model_selection("unknown", |_| None).unwrap_err(),
        "Unknown model selection: unknown"
    );
    assert_eq!(
        normalize_remote_model_selection("   ", |_| None).unwrap_err(),
        "model_id is required"
    );
}

#[test]
fn remote_connect_poll_helpers_preserve_delta_and_completion_policy() {
    let tracker = RemoteSessionStateTracker::new("session-1".to_string());

    assert!(!should_send_remote_model_catalog(
        Some(&sample_remote_model_catalog(11)),
        Some(11)
    ));
    assert!(should_send_remote_model_catalog(
        Some(&sample_remote_model_catalog(12)),
        Some(11)
    ));

    let no_change = serde_json::to_value(remote_no_change_poll_response(7)).expect("serialize no-change poll");
    assert_eq!(no_change["resp"], "session_poll");
    assert_eq!(no_change["changed"], false);
    assert!(no_change.get("active_turn").is_none());

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnStarted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        turn_index: 0,
        user_input: "hello".to_string(),
        original_user_input: None,
        user_message_metadata: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::TextChunk {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        round_id: "round-1".to_string(),
        text: "answer".to_string(),
    });
    tracker.mark_persistence_clean();

    let snapshot = serde_json::to_value(remote_snapshot_poll_response(
        &tracker,
        tracker.version(),
        Some(sample_remote_model_catalog(13)),
    ))
    .expect("serialize snapshot poll");
    assert_eq!(snapshot["changed"], true);
    assert_eq!(snapshot["active_turn"]["turn_id"], "turn-1");
    assert!(snapshot.get("new_messages").is_none());
    assert_eq!(snapshot["model_catalog"]["version"], 13);

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnCompleted {
        session_id: "session-1".to_string(),
        turn_id: "turn-1".to_string(),
        total_rounds: 1,
        total_tools: 0,
        duration_ms: 42,
        partial_recovery_reason: None,
        success: Some(true),
        finish_reason: Some("stop".to_string()),
    });

    let waiting_for_persistence = serde_json::to_value(remote_persisted_poll_response(
        &tracker,
        tracker.version(),
        Vec::new(),
        0,
        None,
    ))
    .expect("serialize completed poll without assistant message");
    assert!(waiting_for_persistence.get("new_messages").is_none());
    assert_eq!(waiting_for_persistence["active_turn"]["status"], "completed");
    assert!(tracker.snapshot_active_turn().is_some());

    let assistant_message = ChatMessage {
        id: "msg-2".to_string(),
        role: "assistant".to_string(),
        content: "answer".to_string(),
        timestamp: "2".to_string(),
        metadata: None,
        tools: None,
        thinking: None,
        items: None,
        images: None,
    };
    let with_persisted_message = serde_json::to_value(remote_persisted_poll_response(
        &tracker,
        tracker.version(),
        vec![assistant_message],
        2,
        None,
    ))
    .expect("serialize completed poll with assistant message");
    assert_eq!(with_persisted_message["new_messages"][0]["role"], "assistant");
    assert_eq!(with_persisted_message["total_msg_count"], 2);
    assert!(with_persisted_message.get("active_turn").is_none());
    assert!(tracker.snapshot_active_turn().is_none());
}

#[test]
fn remote_connect_tracker_ignores_unrelated_direct_session_events() {
    let tracker = RemoteSessionStateTracker::new("session-1".to_string());

    tracker.handle_agentic_event(&AgenticEvent::DialogTurnStarted {
        session_id: "session-2".to_string(),
        turn_id: "turn-2".to_string(),
        turn_index: 0,
        user_input: "hello".to_string(),
        original_user_input: None,
        user_message_metadata: None,
    });
    tracker.handle_agentic_event(&AgenticEvent::TextChunk {
        session_id: "session-2".to_string(),
        turn_id: "turn-2".to_string(),
        round_id: "round-1".to_string(),
        text: "other answer".to_string(),
    });

    assert_eq!(tracker.version(), 0);
    assert_eq!(tracker.session_state(), "idle");
    assert!(tracker.snapshot_active_turn().is_none());
    assert_eq!(tracker.accumulated_text(), "");
}

#[test]
fn remote_connect_tool_preview_slimming_keeps_short_fields_and_drops_large_strings() {
    let preview = make_slim_tool_params(&serde_json::json!({
        "path": "README.md",
        "content": "x".repeat(201),
        "line": 12
    }))
    .expect("object preview");
    let preview_json: serde_json::Value = serde_json::from_str(&preview).expect("preview remains json object");

    assert_eq!(preview_json["path"], "README.md");
    assert_eq!(preview_json["line"], 12);
    assert!(preview_json.get("content").is_none());

    let long_text = "a".repeat(260);
    let text_preview = make_slim_tool_params(&serde_json::Value::String(long_text)).expect("string preview");
    assert_eq!(text_preview.len(), 200);

    assert!(make_slim_tool_params(&serde_json::json!(42)).is_none());
}
