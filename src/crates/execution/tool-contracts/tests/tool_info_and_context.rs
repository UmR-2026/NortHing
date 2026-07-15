//! Group 2: tool_image_attachment, dynamic_tool_info, tool_render_options,
//! runtime_restrictions, tool_context_facts, portable_tool_context_provider,
//! file_tool_guidance, file_read_freshness, persisted_tool_output_message,
//! tool_result_preview, round_budget, tool_result_storage tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn tool_image_attachment_keeps_wire_shape_without_ai_adapter_dependency() {
    let attachment = ToolImageAttachment {
        mime_type: "image/png".to_string(),
        data_base64: "aW1hZ2U=".to_string(),
    };
    let result = ToolResult::ok_with_images(
        json!({"ok": true}),
        Some("captured screenshot".to_string()),
        vec![attachment],
    );

    let value = serde_json::to_value(&result).expect("serialize image tool result");
    assert_eq!(value["type"], "result");
    assert_eq!(value["image_attachments"][0]["mime_type"], "image/png");
    assert_eq!(value["image_attachments"][0]["data_base64"], "aW1hZ2U=");

    let round_trip: ToolResult = serde_json::from_value(value).expect("deserialize tool result");
    match round_trip {
        ToolResult::Result {
            image_attachments: Some(images),
            ..
        } => {
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].mime_type, "image/png");
            assert_eq!(images[0].data_base64, "aW1hZ2U=");
        }
        other => panic!("expected image result, got {other:?}"),
    }
}

#[test]
fn dynamic_tool_info_keeps_provider_and_mcp_metadata_without_core_dependency() {
    let info = DynamicToolInfo {
        provider_id: "github-server-id".to_string(),
        provider_kind: Some("mcp".to_string()),
        mcp: Some(DynamicMcpToolInfo {
            server_id: "github-server-id".to_string(),
            server_name: "GitHub".to_string(),
            tool_name: "search_repos".to_string(),
        }),
    };

    let value = serde_json::to_value(&info).expect("serialize dynamic info");

    assert_eq!(value["providerId"], "github-server-id");
    assert_eq!(value["providerKind"], "mcp");
    assert_eq!(value["mcp"]["serverId"], "github-server-id");
    assert_eq!(value["mcp"]["serverName"], "GitHub");
    assert_eq!(value["mcp"]["toolName"], "search_repos");

    let round_trip: DynamicToolInfo = serde_json::from_value(value).expect("deserialize dynamic info");
    assert_eq!(round_trip.provider_id, "github-server-id");
    assert_eq!(round_trip.provider_kind.as_deref(), Some("mcp"));
    assert_eq!(
        round_trip.mcp.as_ref().map(|mcp| mcp.tool_name.as_str()),
        Some("search_repos")
    );
}

#[test]
fn tool_render_options_stays_a_lightweight_contract() {
    let options = ToolRenderOptions { verbose: true };

    assert!(options.verbose);
}

#[test]
fn runtime_restrictions_keep_allow_deny_semantics_without_core_dependency() {
    let restrictions = ToolRuntimeRestrictions {
        allowed_tool_names: ["Read", "Write"].into_iter().map(str::to_string).collect(),
        denied_tool_names: ["Write"].into_iter().map(str::to_string).collect(),
        denied_tool_messages: Default::default(),
        path_policy: Default::default(),
    };

    assert!(restrictions.is_tool_allowed("Read"));
    assert!(!restrictions.is_tool_allowed("Write"));
    assert!(!restrictions.is_tool_allowed("Bash"));

    let denied = restrictions
        .ensure_tool_allowed("Write")
        .expect_err("deny list must override allow list");
    assert_eq!(denied.to_string(), "Tool 'Write' is denied by runtime restrictions");

    let not_allowed = restrictions
        .ensure_tool_allowed("Bash")
        .expect_err("non-empty allow list must reject missing tools");
    assert_eq!(
        not_allowed.to_string(),
        "Tool 'Bash' is not allowed by runtime restrictions"
    );
}

#[test]
fn runtime_restrictions_surface_custom_deny_messages() {
    let restrictions = ToolRuntimeRestrictions {
        denied_tool_names: ["Task"].into_iter().map(str::to_string).collect(),
        denied_tool_messages: [(
            "Task".to_string(),
            "Recursive subagent delegation is blocked. Use direct tools instead.".to_string(),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    let denied = restrictions
        .ensure_tool_allowed("Task")
        .expect_err("deny message should be returned");
    assert_eq!(
        denied.to_string(),
        "Recursive subagent delegation is blocked. Use direct tools instead."
    );
}

#[test]
fn tool_context_facts_keep_portable_wire_shape_without_runtime_handles() {
    let facts = ToolContextFacts {
        tool_call_id: Some("call-1".to_string()),
        agent_type: Some("Agentic".to_string()),
        session_id: Some("session-1".to_string()),
        dialog_turn_id: Some("turn-1".to_string()),
        workspace_kind: Some(ToolWorkspaceKind::Remote),
        workspace_root: Some("/remote/workspace".to_string()),
        runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
    };

    let value = serde_json::to_value(&facts).expect("serialize context facts");

    assert_eq!(value["toolCallId"], "call-1");
    assert_eq!(value["agentType"], "Agentic");
    assert_eq!(value["sessionId"], "session-1");
    assert_eq!(value["dialogTurnId"], "turn-1");
    assert_eq!(value["workspaceKind"], "remote");
    assert_eq!(value["workspaceRoot"], "/remote/workspace");
    assert!(value.get("unlockedCollapsedTools").is_none());
    assert!(value.get("computer_use_host").is_none());
    assert!(value.get("workspace_services").is_none());
    assert!(value.get("cancellation_token").is_none());

    let round_trip: ToolContextFacts = serde_json::from_value(value).expect("deserialize context facts");
    assert_eq!(round_trip.workspace_kind, Some(ToolWorkspaceKind::Remote));
}

#[test]
fn portable_tool_context_provider_exposes_facts_only() {
    struct FactsOnlyProvider {
        facts: ToolContextFacts,
    }

    impl PortableToolContextProvider for FactsOnlyProvider {
        fn tool_context_facts(&self) -> ToolContextFacts {
            self.facts.clone()
        }
    }

    let provider = FactsOnlyProvider {
        facts: ToolContextFacts {
            tool_call_id: Some("call-2".to_string()),
            agent_type: Some("Agentic".to_string()),
            session_id: Some("session-2".to_string()),
            dialog_turn_id: None,
            workspace_kind: Some(ToolWorkspaceKind::Local),
            workspace_root: Some("/repo/project".to_string()),
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
        },
    };

    let value = serde_json::to_value(provider.tool_context_facts()).expect("serialize context facts");

    assert_eq!(value["toolCallId"], "call-2");
    assert_eq!(value["workspaceKind"], "local");
    assert!(value.get("workspace_services").is_none());
    assert!(value.get("unlockedCollapsedTools").is_none());
}

#[test]
fn file_tool_guidance_marker_is_provider_neutral() {
    let message = file_tool_guidance_message("Read the file first");

    assert_eq!(FILE_TOOL_GUIDANCE_PREFIX, "[guidance] ");
    assert_eq!(message, "[guidance] Read the file first");
    assert!(is_file_tool_guidance_message(&message));
    assert!(!is_file_tool_guidance_message("Read the file first"));
}

#[test]
fn file_read_freshness_policy_preserves_read_edit_write_guardrails() {
    let full_read = FileReadFreshnessFacts {
        content: "alpha\r\n",
        timestamp_ms: 100,
        is_full_file_read: true,
    };

    assert_eq!(normalize_tool_file_content("alpha\r\n"), "alpha\n");
    assert!(file_read_facts_content_matches(full_read, "alpha\n"));
    assert!(file_read_facts_are_fresh(full_read, "alpha\n", Some(200)));
    assert!(!file_read_facts_are_fresh(full_read, "beta\n", Some(200)));
    assert!(file_read_facts_are_fresh(full_read, "beta\n", Some(50)));
    assert!(!file_read_facts_are_fresh(full_read, "beta\n", None));

    let partial_read = FileReadFreshnessFacts {
        content: "middle\n",
        timestamp_ms: 100,
        is_full_file_read: false,
    };
    assert!(!file_read_facts_content_matches(partial_read, "middle\n"));
    assert!(!file_read_facts_are_fresh(partial_read, "full file\n", Some(200)));
    assert!(file_read_facts_are_fresh(partial_read, "full file\n", None));
}

#[test]
fn persisted_tool_output_message_keeps_reference_preview_and_metadata_shape() {
    let rendered = build_persisted_tool_output_message(
        &PersistedToolOutput {
            reference: "northhing-runtime://session/session-1/tool-results/bash_1.txt".to_string(),
            original_chars: 12_345,
            line_count: 7,
            preview: "first lines".to_string(),
            has_more: true,
            metadata: vec![
                ("exit_code".to_string(), "1".to_string()),
                ("working_directory".to_string(), "/repo".to_string()),
            ],
        },
        TOOL_RESULT_PREVIEW_CHARS,
    );

    assert!(rendered.starts_with(PERSISTED_OUTPUT_TAG));
    assert!(rendered.contains("Output too large (12345 chars). Full output saved to:"));
    assert!(rendered.contains("Line count: 7"));
    assert!(rendered.contains("Preview (first 2000 chars):\nfirst lines"));
    assert!(rendered.contains("- exit_code: 1"));
    assert!(rendered.contains("- working_directory: /repo"));
    assert!(tool_result_is_persisted_output(&rendered));
}

#[test]
fn tool_result_preview_prefers_line_boundary_when_possible() {
    let content = "first line\nsecond line\nthird line";

    let (preview, has_more) = generate_tool_result_preview(content, 23);

    assert!(has_more);
    assert_eq!(preview, "first line\nsecond line");
}

#[test]
fn round_budget_candidate_selection_persists_largest_until_under_limit() {
    let candidates = vec![
        ToolResultPersistenceCandidate {
            index: 0,
            visible_chars: 170_000,
        },
        ToolResultPersistenceCandidate {
            index: 1,
            visible_chars: 60_000,
        },
        ToolResultPersistenceCandidate {
            index: 2,
            visible_chars: 30_000,
        },
    ];

    let selected = select_tool_result_indices_for_persistence(&candidates, 260_000, 200_000);

    assert_eq!(selected, vec![0]);
}

#[test]
fn tool_result_storage_helpers_keep_stable_file_and_line_contracts() {
    assert_eq!(sanitize_tool_result_file_component("tool/one", "fallback"), "tool_one");
    assert_eq!(sanitize_tool_result_file_component("", "fallback"), "fallback");
    assert_eq!(count_tool_result_lines(""), 0);
    assert_eq!(count_tool_result_lines("a\nb\n"), 2);
    assert!(!tool_result_is_persisted_output("plain output"));
}
