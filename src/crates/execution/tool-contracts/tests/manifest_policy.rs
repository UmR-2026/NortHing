//! Group 5: tool_manifest_definition, tool_manifest_policy, get_tool_spec_load_collector,
//! collapsed_tool_stub, tool_manifest_sorting, prompt_visible_manifest_builder,
//! get_tool_spec_contract, get_tool_spec_contract_escapes, duplicate_load_hint,
//! duplicate_load_result, detail_result, plans_duplicate_load, plans_detail_load,
//! rejects_missing_tool_name tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn tool_manifest_definition_keeps_lightweight_wire_shape() {
    let definition = ToolManifestDefinition::new(
        "Read",
        "Read a file",
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" }
            },
            "required": ["file_path"]
        }),
    );

    let value = serde_json::to_value(&definition).expect("serialize definition");

    assert_eq!(value["name"], json!("Read"));
    assert_eq!(value["description"], json!("Read a file"));
    assert_eq!(value["parameters"]["required"], json!(["file_path"]));
    assert_eq!(
        serde_json::from_value::<ToolManifestDefinition>(value).expect("deserialize definition"),
        definition
    );
}

#[test]
fn tool_manifest_policy_keeps_get_tool_spec_insertion_and_registry_order() {
    let tools = vec![
        ToolManifestPolicyTool {
            name: "Read".to_string(),
            default_exposure: ToolExposure::Expanded,
            available: true,
        },
        ToolManifestPolicyTool {
            name: "WebSearch".to_string(),
            default_exposure: ToolExposure::Collapsed,
            available: true,
        },
        ToolManifestPolicyTool {
            name: "WebFetch".to_string(),
            default_exposure: ToolExposure::Collapsed,
            available: true,
        },
        ToolManifestPolicyTool {
            name: GET_TOOL_SPEC_TOOL_NAME.to_string(),
            default_exposure: ToolExposure::Expanded,
            available: true,
        },
        ToolManifestPolicyTool {
            name: "HiddenUnavailable".to_string(),
            default_exposure: ToolExposure::Expanded,
            available: false,
        },
    ];
    let allowed_tools = vec![
        "WebFetch".to_string(),
        "Read".to_string(),
        "WebSearch".to_string(),
        "HiddenUnavailable".to_string(),
    ];
    let overrides = Default::default();

    let policy = resolve_tool_manifest_policy(&tools, &allowed_tools, &overrides, GET_TOOL_SPEC_TOOL_NAME);

    assert_eq!(
        policy.allowed_tool_names,
        vec![
            "WebFetch",
            "Read",
            "WebSearch",
            "HiddenUnavailable",
            GET_TOOL_SPEC_TOOL_NAME,
        ]
    );
    assert_eq!(policy.expanded_tool_names, vec!["Read", GET_TOOL_SPEC_TOOL_NAME]);
    assert_eq!(policy.collapsed_tool_names, vec!["WebSearch", "WebFetch"]);
}

#[test]
fn tool_manifest_policy_preserves_explicit_get_tool_spec_duplicate_runtime_contract() {
    let tools = vec![
        ToolManifestPolicyTool {
            name: GET_TOOL_SPEC_TOOL_NAME.to_string(),
            default_exposure: ToolExposure::Expanded,
            available: true,
        },
        ToolManifestPolicyTool {
            name: "WebFetch".to_string(),
            default_exposure: ToolExposure::Collapsed,
            available: true,
        },
    ];
    let allowed_tools = vec![GET_TOOL_SPEC_TOOL_NAME.to_string(), "WebFetch".to_string()];
    let overrides = Default::default();

    let policy = resolve_tool_manifest_policy(&tools, &allowed_tools, &overrides, GET_TOOL_SPEC_TOOL_NAME);

    assert_eq!(policy.allowed_tool_names, vec![GET_TOOL_SPEC_TOOL_NAME, "WebFetch"]);
    assert_eq!(
        policy.expanded_tool_names,
        vec![GET_TOOL_SPEC_TOOL_NAME, GET_TOOL_SPEC_TOOL_NAME],
        "core currently appends the runtime GetToolSpec entry whenever collapsed tools exist"
    );
    assert_eq!(policy.collapsed_tool_names, vec!["WebFetch"]);
}

#[test]
fn get_tool_spec_load_collector_preserves_collapsed_runtime_contract() {
    let collapsed_tools = vec!["WebFetch".to_string(), "GetFileDiff".to_string()];
    let observations = vec![
        GetToolSpecLoadObservation {
            tool_name: GET_TOOL_SPEC_TOOL_NAME,
            loaded_tool_name: Some("WebFetch"),
            is_error: false,
        },
        GetToolSpecLoadObservation {
            tool_name: GET_TOOL_SPEC_TOOL_NAME,
            loaded_tool_name: Some("Read"),
            is_error: false,
        },
        GetToolSpecLoadObservation {
            tool_name: GET_TOOL_SPEC_TOOL_NAME,
            loaded_tool_name: Some("GetFileDiff"),
            is_error: true,
        },
        GetToolSpecLoadObservation {
            tool_name: "Read",
            loaded_tool_name: Some("WebFetch"),
            is_error: false,
        },
        GetToolSpecLoadObservation {
            tool_name: GET_TOOL_SPEC_TOOL_NAME,
            loaded_tool_name: Some("WebFetch"),
            is_error: false,
        },
    ];

    let loaded = collect_loaded_collapsed_tool_names(&observations, &collapsed_tools, GET_TOOL_SPEC_TOOL_NAME);

    assert_eq!(loaded, vec!["WebFetch".to_string()]);
}

#[test]
fn collapsed_tool_stub_definition_preserves_prompt_visible_guardrail() {
    let stub = build_collapsed_tool_stub_definition("WebFetch", "Fetch a URL and return readable content.");

    assert_eq!(stub.name, "WebFetch");
    assert!(stub.description.contains("Fetch a URL"));
    assert!(stub
        .description
        .contains("THIS IS A COLLAPSED TOOL. Before first use, call GetToolSpec({\"tool_name\":\"WebFetch\"}) to load its schema. After that, you can call WebFetch directly."));
    assert_eq!(
        stub.parameters,
        json!({
            "type": "object",
            "additionalProperties": true,
            "properties": {}
        })
    );
}

#[test]
fn tool_manifest_sorting_preserves_prompt_visible_order() {
    let mut definitions = vec![
        ToolManifestDefinition::new("ControlHub", "control", json!({ "type": "object" })),
        ToolManifestDefinition::new("Read", "read", json!({ "type": "object" })),
        ToolManifestDefinition::new("ExternalTool", "external", json!({ "type": "object" })),
        ToolManifestDefinition::new("GetToolSpec", "spec", json!({ "type": "object" })),
        ToolManifestDefinition::new("Task", "task", json!({ "type": "object" })),
    ];

    sort_tool_manifest_definitions(&mut definitions);

    assert_eq!(
        definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Task", "Read", "GetToolSpec", "ControlHub", "ExternalTool"]
    );
}

#[test]
fn prompt_visible_manifest_builder_preserves_expanded_and_collapsed_contract() {
    let definitions = build_prompt_visible_tool_manifest_definitions(&[
        PromptVisibleToolManifestItem::Collapsed {
            name: "WebFetch".to_string(),
            short_description: "Fetch readable web content.".to_string(),
        },
        PromptVisibleToolManifestItem::Expanded(ToolManifestDefinition::new(
            "Read",
            "Read files from the workspace.",
            json!({ "type": "object", "properties": { "path": { "type": "string" } } }),
        )),
        PromptVisibleToolManifestItem::Expanded(ToolManifestDefinition::new(
            "Bash",
            "Run shell commands.",
            json!({ "type": "object", "properties": { "command": { "type": "string" } } }),
        )),
    ]);

    assert_eq!(
        definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Bash", "Read", "WebFetch"]
    );
    assert_eq!(definitions[0].description, "Run shell commands.");
    assert_eq!(
        definitions[0].parameters["properties"]["command"]["type"],
        json!("string")
    );
    assert!(definitions[2]
        .description
        .contains("THIS IS A COLLAPSED TOOL. Before first use, call GetToolSpec({\"tool_name\":\"WebFetch\"}) to load its schema. After that, you can call WebFetch directly."));
}

#[test]
fn get_tool_spec_contract_preserves_input_schema_and_validation() {
    let schema = tool_spec_input_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["tool_name"]));
    assert_eq!(schema["properties"]["tool_name"]["type"], "string");
    assert!(schema["properties"]["tool_name"]["description"]
        .as_str()
        .unwrap_or_default()
        .contains("canonical casing"));

    let missing = validate_get_tool_spec_input(&json!({}));
    assert!(!missing.result);
    assert_eq!(
        missing.message.as_deref(),
        Some("tool_name is required and cannot be empty")
    );
    assert_eq!(missing.error_code, Some(400));

    let empty = validate_get_tool_spec_input(&json!({ "tool_name": "" }));
    assert!(!empty.result);
    assert_eq!(
        empty.message.as_deref(),
        Some("tool_name is required and cannot be empty")
    );
    assert_eq!(empty.error_code, Some(400));

    assert!(validate_get_tool_spec_input(&json!({ "tool_name": "Git" })).result);
}

#[test]
fn get_tool_spec_contract_preserves_static_metadata_and_use_message() {
    assert_eq!(
        tool_spec_short_description(),
        "Discover collapsed tools and read their detailed definitions."
    );
    assert!(tool_spec_is_readonly());
    assert!(get_tool_spec_is_concurrency_safe(Some(&json!({
        "tool_name": "WebFetch"
    }))));
    assert!(!get_tool_spec_needs_permissions(Some(&json!({
        "tool_name": "WebFetch"
    }))));
    assert_eq!(
        render_get_tool_spec_tool_use_message(&json!({ "tool_name": "Git" })),
        "Reading tool spec for 'Git'."
    );
    assert_eq!(
        render_get_tool_spec_tool_use_message(&json!({})),
        "Reading tool spec for '?'."
    );
}

#[test]
fn get_tool_spec_contract_escapes_assistant_detail_for_xml_sections() {
    let detail = build_get_tool_spec_assistant_detail(
        "Use <danger> & keep output valid.",
        &json!({
            "type": "object",
            "properties": {
                "query": {
                    "description": "Match <tag> & symbols"
                }
            }
        }),
    );

    assert!(detail.contains("<description>\nUse &lt;danger&gt; &amp; keep output valid."));
    assert!(detail.contains("\"description\":\"Match &lt;tag&gt; &amp; symbols\""));
    assert!(!detail.contains("Use <danger> & keep output valid."));
}

#[test]
fn get_tool_spec_contract_preserves_duplicate_load_hint() {
    assert_eq!(
        build_get_tool_spec_duplicate_load_hint("WebFetch"),
        "Tool 'WebFetch' is already loaded in the current conversation. Do not call GetToolSpec again for it. Use 'WebFetch' directly."
    );
}

#[test]
fn get_tool_spec_contract_builds_duplicate_load_result() {
    let result = build_get_tool_spec_duplicate_load_result("WebFetch");

    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = result
    else {
        panic!("expected normal tool result");
    };

    assert_eq!(data["tool_name"], "WebFetch");
    assert_eq!(data["already_loaded"], true);
    assert_eq!(
        result_for_assistant.as_deref(),
        Some(
            "Tool 'WebFetch' is already loaded in the current conversation. Do not call GetToolSpec again for it. Use 'WebFetch' directly."
        )
    );
    assert_eq!(image_attachments, None);
}

#[test]
fn get_tool_spec_contract_builds_detail_result() {
    let result = build_get_tool_spec_detail_result(&northhing_agent_tools::GetToolSpecDetail {
        tool_name: "Git".to_string(),
        description: "Use <repo> & inspect changes.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Run <safe> git commands"
                }
            }
        }),
    });

    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = result
    else {
        panic!("expected normal tool result");
    };

    assert_eq!(data["tool_name"], "Git");
    assert_eq!(data["description"], "Use <repo> & inspect changes.");
    assert_eq!(data["input_schema"]["properties"]["command"]["type"], "string");
    let assistant = result_for_assistant.expect("assistant detail");
    assert!(assistant.contains("Use &lt;repo&gt; &amp; inspect changes."));
    assert!(assistant.contains("Run &lt;safe&gt; git commands"));
    assert_eq!(image_attachments, None);
}

#[test]
fn get_tool_spec_contract_plans_duplicate_load_without_core_context() {
    let input = json!({ "tool_name": "WebFetch" });
    let plan = northhing_agent_tools::resolve_get_tool_spec_execution_plan(&input, &["WebFetch".to_string()])
        .expect("duplicate load should be planned");

    let GetToolSpecExecutionPlan::DuplicateLoad(result) = plan else {
        panic!("expected duplicate-load plan");
    };

    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = result
    else {
        panic!("expected normal tool result");
    };

    assert_eq!(data["tool_name"], "WebFetch");
    assert_eq!(data["already_loaded"], true);
    assert!(result_for_assistant
        .as_deref()
        .unwrap_or_default()
        .contains("already loaded in the current conversation"));
    assert_eq!(image_attachments, None);
}

#[test]
fn get_tool_spec_contract_plans_detail_load_without_resolving_product_detail() {
    let input = json!({ "tool_name": "Git" });
    let plan = northhing_agent_tools::resolve_get_tool_spec_execution_plan(&input, &["WebFetch".to_string()])
        .expect("detail load should be planned");

    let GetToolSpecExecutionPlan::LoadDetail { tool_name } = plan else {
        panic!("expected detail-load plan");
    };

    assert_eq!(tool_name, "Git");
}

#[test]
fn get_tool_spec_contract_rejects_missing_tool_name_in_execution_plan() {
    let err = northhing_agent_tools::resolve_get_tool_spec_execution_plan(&json!({}), &[])
        .expect_err("missing tool name should be rejected");

    assert_eq!(err, GetToolSpecExecutionError::MissingToolName);
    assert_eq!(err.to_string(), "tool_name is required");
}
