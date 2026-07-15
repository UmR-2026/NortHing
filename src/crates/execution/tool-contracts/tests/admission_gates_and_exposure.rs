//! Group 4: collapsed_tool_usage_gate, tool_allowed_list_gate,
//! tool_execution_admission_gate, remote_posix_path, host_path,
//! unified_tool_path, dynamic_tool_provider, tool_exposure tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn collapsed_tool_usage_gate_preserves_get_tool_spec_unlock_contract() {
    let collapsed_tools = vec!["WebFetch".to_string()];
    let loaded_collapsed_tools = Vec::new();

    let err = validate_collapsed_tool_usage(
        "WebFetch",
        &collapsed_tools,
        &loaded_collapsed_tools,
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .expect_err("collapsed tool should require GetToolSpec unlock");
    assert_eq!(
        err.to_string(),
        "Tool 'WebFetch' is collapsed. Call GetToolSpec first with {\"tool_name\":\"WebFetch\"} to read its full usage instructions and input schema, then try again."
    );

    let loaded_collapsed_tools = vec!["WebFetch".to_string()];
    validate_collapsed_tool_usage(
        "WebFetch",
        &collapsed_tools,
        &loaded_collapsed_tools,
        GET_TOOL_SPEC_TOOL_NAME,
    )
    .expect("loaded collapsed tool should be executable");

    validate_collapsed_tool_usage(GET_TOOL_SPEC_TOOL_NAME, &collapsed_tools, &[], GET_TOOL_SPEC_TOOL_NAME)
        .expect("GetToolSpec itself is the unlock path");
}

#[test]
fn tool_allowed_list_gate_preserves_pipeline_rejection_contract() {
    validate_tool_allowed_by_list("Read", &[]).expect("empty allowed-list should preserve allow-all behavior");

    let allowed_tools = vec!["Read".to_string(), "GetToolSpec".to_string()];
    validate_tool_allowed_by_list("Read", &allowed_tools).expect("listed tool should be allowed");

    let err = validate_tool_allowed_by_list("Bash", &allowed_tools).expect_err("unlisted tool should be rejected");
    assert_eq!(
        err.to_string(),
        "Tool 'Bash' is not in the allowed list: [\"Read\", \"GetToolSpec\"]"
    );
}

#[test]
fn tool_execution_admission_gate_preserves_pipeline_rejection_order() {
    let mut restrictions = ToolRuntimeRestrictions::default();
    restrictions.denied_tool_names.insert("WebFetch".to_string());

    let request = ToolExecutionAdmissionRequest {
        tool_name: "WebFetch",
        allowed_tools: &["Read".to_string()],
        runtime_tool_restrictions: &restrictions,
        collapsed_tools: &["WebFetch".to_string()],
        loaded_collapsed_tools: &[],
        get_tool_spec_tool_name: GET_TOOL_SPEC_TOOL_NAME,
    };

    let err = validate_tool_execution_admission(request)
        .expect_err("allowed-list should be evaluated before runtime restrictions");

    assert!(matches!(err, ToolExecutionAdmissionRejection::AllowedList(_)));
    assert_eq!(
        err.to_string(),
        "Tool 'WebFetch' is not in the allowed list: [\"Read\"]"
    );

    let request = ToolExecutionAdmissionRequest {
        tool_name: "WebFetch",
        allowed_tools: &["WebFetch".to_string()],
        runtime_tool_restrictions: &restrictions,
        collapsed_tools: &["WebFetch".to_string()],
        loaded_collapsed_tools: &[],
        get_tool_spec_tool_name: GET_TOOL_SPEC_TOOL_NAME,
    };

    let err = validate_tool_execution_admission(request)
        .expect_err("runtime restrictions should run before collapsed unlock");

    assert!(matches!(err, ToolExecutionAdmissionRejection::RuntimeRestriction(_)));
    assert_eq!(err.to_string(), "Tool 'WebFetch' is denied by runtime restrictions");

    let request = ToolExecutionAdmissionRequest {
        tool_name: "WebFetch",
        allowed_tools: &["WebFetch".to_string()],
        runtime_tool_restrictions: &ToolRuntimeRestrictions::default(),
        collapsed_tools: &["WebFetch".to_string()],
        loaded_collapsed_tools: &[],
        get_tool_spec_tool_name: GET_TOOL_SPEC_TOOL_NAME,
    };

    let err = validate_tool_execution_admission(request)
        .expect_err("collapsed tool should require GetToolSpec after access gates pass");

    assert!(matches!(err, ToolExecutionAdmissionRejection::Collapsed(_)));
    assert!(err
        .to_string()
        .contains("Call GetToolSpec first with {\"tool_name\":\"WebFetch\"}"));
}

#[test]
fn remote_posix_path_contract_keeps_workspace_containment_semantics() {
    assert!(posix_style_path_is_absolute(r"\home\workspace"));
    assert_eq!(
        posix_resolve_path_with_workspace(r"src\lib.rs", Some("/home/project"))
            .expect("relative remote path should resolve"),
        "/home/project/src/lib.rs"
    );
    assert!(is_remote_posix_path_within_root(
        "/home/project/src/lib.rs",
        "/home/project"
    ));
    assert!(!is_remote_posix_path_within_root(
        "/home/project2/src/lib.rs",
        "/home/project"
    ));
}

#[test]
fn host_path_contract_keeps_local_workspace_resolution_semantics() {
    let normalized = normalize_host_path("repo/./src/../README.md");
    assert_eq!(PathBuf::from(normalized), PathBuf::from("repo").join("README.md"));

    let workspace = PathBuf::from("/repo/project");
    let resolved = resolve_host_path_with_workspace("src/main.rs", Some(workspace.as_path()))
        .expect("relative local path should resolve from workspace");
    assert_eq!(PathBuf::from(resolved), workspace.join("src").join("main.rs"));

    let missing = resolve_host_path_with_workspace("src/main.rs", None)
        .expect_err("relative local path should require a workspace");
    assert_eq!(
        missing.to_string(),
        "A workspace path is required to resolve relative path: src/main.rs"
    );
}

#[test]
fn unified_tool_path_contract_selects_host_or_remote_semantics() {
    let local =
        resolve_workspace_tool_path("src/lib.rs", Some("/repo/project"), false).expect("local path should resolve");
    assert_eq!(PathBuf::from(local), PathBuf::from("/repo/project/src/lib.rs"));

    let remote =
        resolve_workspace_tool_path("src/lib.rs", Some("/home/project"), true).expect("remote path should resolve");
    assert_eq!(remote, "/home/project/src/lib.rs");
}

#[test]
fn dynamic_tool_provider_contract_is_available_from_agent_tools_boundary() {
    fn assert_provider_contract<T: DynamicToolProvider>() {}
    fn assert_decorator_contract<T: ToolDecorator<String>>() {}

    struct MarkerProvider;
    #[async_trait::async_trait]
    impl DynamicToolProvider for MarkerProvider {
        async fn list_dynamic_tools(&self) -> PortResult<Vec<DynamicToolDescriptor>> {
            Ok(Vec::new())
        }
    }

    struct MarkerDecorator;
    impl ToolDecorator<String> for MarkerDecorator {
        fn decorate(&self, tool: String) -> String {
            tool
        }
    }

    assert_provider_contract::<MarkerProvider>();
    assert_decorator_contract::<MarkerDecorator>();
}

#[test]
fn tool_exposure_contract_keeps_lightweight_wire_shape() {
    let collapsed = ToolExposure::Collapsed;
    let value = serde_json::to_value(collapsed).expect("serialize exposure");

    assert_eq!(value, json!("Collapsed"));
    assert_eq!(
        serde_json::from_value::<ToolExposure>(value).expect("deserialize exposure"),
        ToolExposure::Collapsed
    );
}
