//! Group 7: generic_readonly_enabled_filter, manifest_policy_tools_from_registry_snapshot,
//! contextual_manifest_resolver (2 tests), tool_catalog_runtime_facade,
//! get_tool_spec_detail_resolver, get_tool_spec_catalog_provider (2 tests),
//! get_tool_spec_runtime_facade (3 tests), classifies_detail_errors,
//! generic_tool_registry dynamic descriptor / stale dynamic metadata tests.

mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn generic_readonly_enabled_filter_preserves_registry_order() {
    let tools = vec![
        registry_marker_tool_with_access("Read", None, ToolExposure::Expanded, true, true),
        registry_marker_tool_with_access("Write", None, ToolExposure::Expanded, false, true),
        registry_marker_tool_with_access("DisabledReadonly", None, ToolExposure::Expanded, true, false),
        registry_marker_tool_with_access("WebFetch", None, ToolExposure::Collapsed, true, true),
    ];

    let readonly_names = resolve_readonly_enabled_tools(&tools)
        .await
        .iter()
        .map(|tool| tool.name().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        readonly_names,
        vec!["Read".to_string(), "WebFetch".to_string()],
        "readonly filtering must keep registry order and skip disabled or mutating tools"
    );
}

#[test]
fn manifest_policy_tools_from_registry_snapshot_preserve_exposure_and_availability() {
    let tools = vec![
        registry_marker_tool("Read", None),
        registry_marker_tool_with_exposure("WebFetch", None, ToolExposure::Collapsed),
        registry_marker_tool_with_exposure("Git", None, ToolExposure::Collapsed),
    ];
    let available_tool_names = ["Read".to_string(), "Git".to_string()].into_iter().collect();

    let policy_tools = northhing_agent_tools::build_tool_manifest_policy_tools(&tools, &available_tool_names);

    assert_eq!(
        policy_tools,
        vec![
            ToolManifestPolicyTool {
                name: "Read".to_string(),
                default_exposure: ToolExposure::Expanded,
                available: true,
            },
            ToolManifestPolicyTool {
                name: "WebFetch".to_string(),
                default_exposure: ToolExposure::Collapsed,
                available: false,
            },
            ToolManifestPolicyTool {
                name: "Git".to_string(),
                default_exposure: ToolExposure::Collapsed,
                available: true,
            },
        ]
    );
}

#[tokio::test]
async fn contextual_manifest_resolver_preserves_runtime_visible_manifest_contract() {
    let tools = vec![
        contextual_manifest_tool("Read", ToolExposure::Expanded, None),
        contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None),
        contextual_manifest_tool("Git", ToolExposure::Collapsed, Some("other-agent")),
        contextual_manifest_tool(GET_TOOL_SPEC_TOOL_NAME, ToolExposure::Expanded, None),
    ];

    let manifest = resolve_contextual_tool_manifest(
        &tools,
        &["Read".to_string(), "WebFetch".to_string(), "Git".to_string()],
        &Default::default(),
        &ManifestTestContext { agent: "agentic" },
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .await;

    assert_eq!(
        manifest.allowed_tool_names,
        vec![
            "Read".to_string(),
            "WebFetch".to_string(),
            "Git".to_string(),
            GET_TOOL_SPEC_TOOL_NAME.to_string(),
        ],
        "GetToolSpec insertion must preserve the runtime allowed-list contract"
    );
    assert_eq!(
        manifest.collapsed_tool_names,
        vec!["WebFetch".to_string()],
        "unavailable collapsed tools must not leak into the prompt-visible unlock catalog"
    );
    assert_eq!(
        manifest
            .expanded_tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Read", GET_TOOL_SPEC_TOOL_NAME],
        "expanded tool handles must follow the resolved runtime policy"
    );
    assert_eq!(
        manifest
            .collapsed_tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["WebFetch"],
        "collapsed tool handles must follow the resolved runtime policy"
    );
    assert_eq!(
        manifest
            .tool_definitions
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Read", "WebFetch", GET_TOOL_SPEC_TOOL_NAME],
        "prompt-visible manifest ordering must stay stable when the owner moves"
    );

    let read = manifest
        .tool_definitions
        .iter()
        .find(|tool| tool.name == "Read")
        .expect("expanded Read manifest");
    assert_eq!(read.description, "Read description for agentic");
    assert_eq!(read.parameters["properties"]["agent"]["const"], "agentic");

    let web_fetch = manifest
        .tool_definitions
        .iter()
        .find(|tool| tool.name == "WebFetch")
        .expect("collapsed WebFetch stub");
    assert!(web_fetch
        .description
        .contains("THIS IS A COLLAPSED TOOL. Before first use, call GetToolSpec({\"tool_name\":\"WebFetch\"}) to load its schema. After that, you can call WebFetch directly."));
    assert_eq!(web_fetch.parameters["additionalProperties"], true);
    assert_eq!(web_fetch.parameters["properties"], json!({}));
}

#[tokio::test]
async fn contextual_manifest_resolver_accepts_snapshot_provider_boundary() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![
            contextual_manifest_tool("Read", ToolExposure::Expanded, None),
            contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None),
            contextual_manifest_tool("Git", ToolExposure::Collapsed, Some("other-agent")),
            contextual_manifest_tool(GET_TOOL_SPEC_TOOL_NAME, ToolExposure::Expanded, None),
        ],
    };

    let manifest = resolve_contextual_tool_manifest_from_provider(
        &provider,
        &["Read".to_string(), "WebFetch".to_string(), "Git".to_string()],
        &Default::default(),
        &ManifestTestContext { agent: "agentic" },
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .await;

    assert_eq!(
        manifest.allowed_tool_names,
        vec![
            "Read".to_string(),
            "WebFetch".to_string(),
            "Git".to_string(),
            GET_TOOL_SPEC_TOOL_NAME.to_string(),
        ],
        "provider-backed resolution must preserve allowed-list semantics"
    );
    assert_eq!(
        manifest
            .tool_definitions
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Read", "WebFetch", GET_TOOL_SPEC_TOOL_NAME],
        "provider-backed resolution must preserve prompt-visible manifest ordering"
    );
    assert_eq!(
        manifest.collapsed_tool_names,
        vec!["WebFetch".to_string()],
        "provider-backed resolution must preserve context-aware availability filtering"
    );
}

#[tokio::test]
async fn tool_catalog_runtime_facade_owns_manifest_and_readonly_paths() {
    let manifest_provider = ContextualManifestSnapshotProvider {
        tools: vec![
            contextual_manifest_tool("Read", ToolExposure::Expanded, None),
            contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None),
            contextual_manifest_tool("Git", ToolExposure::Collapsed, Some("other-agent")),
            contextual_manifest_tool(GET_TOOL_SPEC_TOOL_NAME, ToolExposure::Expanded, None),
        ],
    };

    let runtime = ToolCatalogRuntime::<ContextualManifestTool, ManifestTestContext, _>::new(
        &manifest_provider,
        GET_TOOL_SPEC_TOOL_NAME,
    );

    let visible_tools = runtime
        .visible_tools(
            &["Read".to_string(), "WebFetch".to_string(), "Git".to_string()],
            &Default::default(),
            &ManifestTestContext { agent: "agentic" },
        )
        .await;
    assert_eq!(
        visible_tools.allowed_tool_names,
        vec![
            "Read".to_string(),
            "WebFetch".to_string(),
            "Git".to_string(),
            GET_TOOL_SPEC_TOOL_NAME.to_string(),
        ],
        "runtime facade must preserve allowed-list insertion"
    );
    assert_eq!(
        visible_tools
            .expanded_tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Read", GET_TOOL_SPEC_TOOL_NAME],
        "runtime facade must preserve expanded handle order"
    );
    assert_eq!(
        visible_tools.collapsed_tool_names,
        vec!["WebFetch".to_string()],
        "runtime facade must preserve context-aware collapsed filtering"
    );

    let manifest = runtime
        .tool_manifest(
            &["Read".to_string(), "WebFetch".to_string(), "Git".to_string()],
            &Default::default(),
            &ManifestTestContext { agent: "agentic" },
        )
        .await;
    assert_eq!(
        manifest
            .tool_definitions
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Read", "WebFetch", GET_TOOL_SPEC_TOOL_NAME],
        "runtime facade must preserve prompt-visible manifest order"
    );

    let readonly_provider = RegistryMarkerSnapshotProvider {
        tools: vec![
            registry_marker_tool_with_access("Read", None, ToolExposure::Expanded, true, true),
            registry_marker_tool_with_access("Write", None, ToolExposure::Expanded, false, true),
            registry_marker_tool_with_access("Disabled", None, ToolExposure::Expanded, true, false),
            registry_marker_tool_with_access("WebFetch", None, ToolExposure::Collapsed, true, true),
        ],
    };
    let readonly_runtime = ToolCatalogRuntime::<RegistryMarkerTool, ManifestTestContext, _>::new(
        &readonly_provider,
        GET_TOOL_SPEC_TOOL_NAME,
    );
    let readonly_names = readonly_runtime
        .readonly_enabled_tools()
        .await
        .into_iter()
        .map(|tool| tool.name().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        readonly_names,
        vec!["Read".to_string(), "WebFetch".to_string()],
        "runtime facade must preserve readonly enabled filtering order"
    );
}

#[tokio::test]
async fn get_tool_spec_detail_resolver_preserves_contextual_detail_contract() {
    let collapsed_tools = vec![
        contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None),
        contextual_manifest_tool(GET_TOOL_SPEC_TOOL_NAME, ToolExposure::Collapsed, None),
    ];
    let context = ManifestTestContext { agent: "agentic" };

    let summaries = summarize_get_tool_spec_collapsed_tools(&collapsed_tools);
    assert_eq!(
        summaries,
        vec![
            GetToolSpecCollapsedToolSummary {
                name: "WebFetch".to_string(),
                short_description: "WebFetch short description".to_string(),
            },
            GetToolSpecCollapsedToolSummary {
                name: GET_TOOL_SPEC_TOOL_NAME.to_string(),
                short_description: "GetToolSpec short description".to_string(),
            },
        ],
        "catalog summaries must preserve collapsed tool order and short descriptions"
    );

    let detail = resolve_get_tool_spec_detail(&collapsed_tools, "WebFetch", &context, GET_TOOL_SPEC_TOOL_NAME)
        .await
        .expect("collapsed WebFetch detail");

    assert_eq!(detail.tool_name, "WebFetch");
    assert_eq!(detail.description, "WebFetch description for agentic");
    assert_eq!(detail.input_schema["properties"]["agent"]["const"], "agentic");
    assert_eq!(
        detail.to_value(),
        json!({
            "tool_name": "WebFetch",
            "description": "WebFetch description for agentic",
            "input_schema": {
                "type": "object",
                "properties": {
                    "agent": {
                        "const": "agentic"
                    }
                }
            }
        }),
        "detail JSON shape must stay compatible with GetToolSpec execution output"
    );

    let missing = resolve_get_tool_spec_detail(&collapsed_tools, "Git", &context, GET_TOOL_SPEC_TOOL_NAME)
        .await
        .expect_err("missing tool should stay a validation-style error");
    assert_eq!(
        missing,
        "Tool 'Git' is not an available collapsed tool in the current context"
    );

    let self_inspection = resolve_get_tool_spec_detail(
        &collapsed_tools,
        GET_TOOL_SPEC_TOOL_NAME,
        &context,
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .await
    .expect_err("GetToolSpec should not inspect itself");
    assert_eq!(self_inspection, "Tool 'GetToolSpec' cannot inspect itself");
}

#[tokio::test]
async fn get_tool_spec_catalog_provider_preserves_runtime_catalog_contract() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![
            contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None),
            contextual_manifest_tool("Git", ToolExposure::Collapsed, Some("other-agent")),
            contextual_manifest_tool("Read", ToolExposure::Expanded, None),
        ],
    };
    let context = ManifestTestContext { agent: "agentic" };

    let detail = resolve_get_tool_spec_detail_from_provider(&provider, "WebFetch", &context, GET_TOOL_SPEC_TOOL_NAME)
        .await
        .expect("provider-backed detail");
    assert_eq!(detail.tool_name, "WebFetch");
    assert_eq!(detail.description, "WebFetch description for agentic");
}

#[tokio::test]
async fn get_tool_spec_provider_execution_returns_duplicate_result_without_detail_lookup() {
    let context = ManifestTestContext { agent: "agentic" };
    let input = json!({ "tool_name": "WebFetch" });

    let result = resolve_get_tool_spec_execution_result_from_provider(
        &ErroringGetToolSpecProvider,
        &input,
        &["WebFetch".to_string()],
        &context,
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .await
    .expect("duplicate load should not call provider detail lookup");

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

#[tokio::test]
async fn get_tool_spec_provider_execution_returns_detail_result_from_provider() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None)],
    };
    let context = ManifestTestContext { agent: "agentic" };
    let input = json!({ "tool_name": "WebFetch" });

    let result =
        resolve_get_tool_spec_execution_result_from_provider(&provider, &input, &[], &context, GET_TOOL_SPEC_TOOL_NAME)
            .await
            .expect("detail result should come from provider");

    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = result
    else {
        panic!("expected normal tool result");
    };

    assert_eq!(data["tool_name"], "WebFetch");
    assert_eq!(data["description"], "WebFetch description for agentic");
    assert_eq!(data["input_schema"]["properties"]["agent"]["const"], "agentic");
    let assistant = result_for_assistant.expect("assistant detail");
    assert!(assistant.contains("<description>\nWebFetch description for agentic"));
    assert!(assistant.contains("\"agent\""));
    assert!(assistant.contains("\"agentic\""));
    assert_eq!(image_attachments, None);
}

#[tokio::test]
async fn get_tool_spec_runtime_facade_owns_execution_path() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None)],
    };
    let context = ManifestTestContext { agent: "agentic" };
    let input = json!({ "tool_name": "WebFetch" });
    let runtime =
        GetToolSpecRuntime::<ContextualManifestTool, ManifestTestContext, _>::new(&provider, GET_TOOL_SPEC_TOOL_NAME);

    let result = runtime
        .execute(&input, &[], &context)
        .await
        .expect("collapsed tool detail should resolve through runtime facade");

    let ToolResult::Result { data, .. } = result else {
        panic!("expected normal tool result");
    };
    assert_eq!(data["tool_name"], "WebFetch");
    assert_eq!(data["description"], "WebFetch description for agentic");
    assert_eq!(data["input_schema"]["properties"]["agent"]["const"], "agentic");
}

#[tokio::test]
async fn get_tool_spec_runtime_facade_owns_tool_result_vector_adapter_shape() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None)],
    };
    let context = ManifestTestContext { agent: "agentic" };
    let runtime =
        GetToolSpecRuntime::<ContextualManifestTool, ManifestTestContext, _>::new(&provider, GET_TOOL_SPEC_TOOL_NAME);

    let mut results = runtime
        .call_results(&json!({ "tool_name": "WebFetch" }), &[], &context)
        .await
        .expect("runtime facade should produce the Tool impl result vector shape");

    assert_eq!(results.len(), 1);
    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = results.remove(0)
    else {
        panic!("expected normal detail result");
    };
    assert_eq!(data["tool_name"], "WebFetch");
    assert!(result_for_assistant
        .expect("assistant detail")
        .contains("<description>\nWebFetch description for agentic"));
    assert_eq!(image_attachments, None);

    let duplicate_runtime = GetToolSpecRuntime::<ContextualManifestTool, ManifestTestContext, _>::new(
        &ErroringGetToolSpecProvider,
        GET_TOOL_SPEC_TOOL_NAME,
    );
    let duplicate_results = duplicate_runtime
        .call_results(&json!({ "tool_name": "WebFetch" }), &["WebFetch".to_string()], &context)
        .await
        .expect("duplicate-load path should not consult provider detail");

    assert_eq!(duplicate_results.len(), 1);
    let ToolResult::Result {
        data,
        result_for_assistant,
        image_attachments,
    } = &duplicate_results[0]
    else {
        panic!("expected duplicate-load result");
    };
    assert_eq!(data["tool_name"], "WebFetch");
    assert_eq!(
        result_for_assistant.as_deref(),
        Some(
            "Tool 'WebFetch' is already loaded in the current conversation. Do not call GetToolSpec again for it. Use 'WebFetch' directly."
        )
    );
    assert!(image_attachments.is_none());
}

#[test]
fn get_tool_spec_runtime_facade_owns_static_tool_surface() {
    let provider = ContextualManifestSnapshotProvider { tools: Vec::new() };
    let runtime =
        GetToolSpecRuntime::<ContextualManifestTool, ManifestTestContext, _>::new(&provider, GET_TOOL_SPEC_TOOL_NAME);

    assert_eq!(runtime.name(), GET_TOOL_SPEC_TOOL_NAME);
    assert_eq!(runtime.short_description(), tool_spec_short_description());
    assert_eq!(runtime.input_schema(), tool_spec_input_schema());
    assert!(runtime.is_readonly());
    assert!(runtime.is_concurrency_safe(None));
    assert!(!runtime.needs_permissions(None));
    assert_eq!(
        runtime.render_tool_use_message(&json!({ "tool_name": "WebFetch" })),
        "Reading tool spec for 'WebFetch'."
    );
    assert!(runtime.validate_input(&json!({ "tool_name": "WebFetch" })).result);
    assert!(!runtime.validate_input(&json!({})).result);
}

#[tokio::test]
async fn get_tool_spec_provider_execution_classifies_detail_errors() {
    let provider = ContextualManifestSnapshotProvider {
        tools: vec![contextual_manifest_tool("WebFetch", ToolExposure::Collapsed, None)],
    };
    let context = ManifestTestContext { agent: "agentic" };
    let input = json!({ "tool_name": "Git" });

    let err =
        resolve_get_tool_spec_execution_result_from_provider(&provider, &input, &[], &context, GET_TOOL_SPEC_TOOL_NAME)
            .await
            .expect_err("missing detail should be classified separately from input errors");

    assert_eq!(
        err,
        GetToolSpecExecutionError::Detail(
            "Tool 'Git' is not an available collapsed tool in the current context".to_string()
        )
    );
    assert_eq!(
        err.to_string(),
        "Tool 'Git' is not an available collapsed tool in the current context"
    );
}

#[tokio::test]
async fn generic_tool_registry_preserves_dynamic_descriptor_contract() {
    let mut registry = ToolRegistry::new();
    registry.register_tool(registry_marker_tool("external_search", Some("provider-a")));
    registry.register_tool(registry_marker_tool("local_docs", Some("provider-b")));
    registry.register_tool(registry_marker_tool("static_tool", None));

    assert_eq!(
        registry.tool_names(),
        vec!["external_search", "local_docs", "static_tool"]
    );
    assert_eq!(
        registry
            .get_dynamic_tool_info("external_search")
            .expect("dynamic metadata")
            .provider_id,
        "provider-a"
    );

    let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");
    assert_eq!(
        descriptors
            .iter()
            .map(|descriptor| (descriptor.name.as_str(), descriptor.provider_id.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("external_search", Some("provider-a")),
            ("local_docs", Some("provider-b")),
        ]
    );
    assert_eq!(descriptors[0].description, "marker tool");
    assert_eq!(descriptors[0].input_schema, json!({ "type": "object" }));
}

#[tokio::test]
async fn generic_tool_registry_clears_stale_dynamic_metadata_on_overwrite() {
    let mut registry = ToolRegistry::new();
    registry.register_tool(registry_marker_tool("external_search", Some("provider-a")));

    registry.register_tool(registry_marker_tool("external_search", None));

    assert!(registry.get_dynamic_tool_info("external_search").is_none());
    let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");
    assert!(descriptors.is_empty());
}
