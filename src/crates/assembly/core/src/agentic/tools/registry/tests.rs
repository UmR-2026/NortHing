// Registry contract tests.
//
// Included from `mod.rs` via `include!("tests.rs")` so the `#[cfg(test)]`
// boundary stays out of the lean facade module.

#[cfg(test)]
mod tests {
    use super::ToolRegistry;
    use super::{
        create_tool_registry, get_readonly_tools, tool_capabilities_summary, ToolCapabilitiesSummary, ToolRef,
    };
    use crate::agentic::tools::framework::{
        DynamicMcpToolInfo, DynamicToolInfo, Tool, ToolResult, ToolUseContext, ValidationResult,
    };
    use crate::agentic::tools::product_runtime::ProductToolRuntime;
    use async_trait::async_trait;
    use northhing_agent_tools::{DynamicToolProvider, ToolDecorator};
    use serde_json::json;
    use serde_json::Value;
    use std::sync::Arc;

    struct DynamicMetadataTool {
        name: String,
        dynamic_info: Option<DynamicToolInfo>,
    }

    #[async_trait]
    impl Tool for DynamicMetadataTool {
        fn name(&self) -> &str {
            &self.name
        }

        async fn description(&self) -> crate::util::errors::NortHingResult<String> {
            Ok("dynamic test tool".to_string())
        }

        fn short_description(&self) -> String {
            "dynamic test tool".to_string()
        }

        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }

        fn dynamic_provider_id(&self) -> Option<&str> {
            self.dynamic_info.as_ref().map(|info| info.provider_id.as_str())
        }

        fn dynamic_tool_info(&self) -> Option<DynamicToolInfo> {
            self.dynamic_info.clone()
        }

        async fn validate_input(&self, _input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
            ValidationResult {
                result: true,
                message: None,
                error_code: None,
                meta: None,
            }
        }

        async fn call_impl(
            &self,
            _input: &Value,
            _context: &ToolUseContext,
        ) -> crate::util::errors::NortHingResult<Vec<ToolResult>> {
            Ok(Vec::new())
        }
    }

    fn dynamic_tool(name: &str, provider_id: Option<&str>) -> ToolRef {
        Arc::new(DynamicMetadataTool {
            name: name.to_string(),
            dynamic_info: provider_id.map(|provider_id| DynamicToolInfo {
                provider_id: provider_id.to_string(),
                provider_kind: None,
                mcp: None,
            }),
        })
    }

    fn mcp_dynamic_tool(
        name: &str,
        _provider_id: Option<&str>,
        server_id: &str,
        server_name: &str,
        tool_name: &str,
    ) -> ToolRef {
        Arc::new(DynamicMetadataTool {
            name: name.to_string(),
            dynamic_info: Some(DynamicToolInfo {
                provider_id: server_id.to_string(),
                provider_kind: Some("mcp".to_string()),
                mcp: Some(DynamicMcpToolInfo {
                    server_id: server_id.to_string(),
                    server_name: server_name.to_string(),
                    tool_name: tool_name.to_string(),
                }),
            }),
        })
    }

    #[derive(Debug, Clone)]
    struct MarkerToolDecorator;

    impl ToolDecorator<ToolRef> for MarkerToolDecorator {
        fn decorate(&self, tool: ToolRef) -> ToolRef {
            Arc::new(DecoratedMarkerTool {
                name: tool.name().to_string(),
                exposure: tool.default_exposure(),
                readonly: tool.is_readonly(),
            })
        }
    }

    struct DecoratedMarkerTool {
        name: String,
        exposure: crate::agentic::tools::framework::ToolExposure,
        readonly: bool,
    }

    #[async_trait]
    impl Tool for DecoratedMarkerTool {
        fn name(&self) -> &str {
            &self.name
        }

        async fn description(&self) -> crate::util::errors::NortHingResult<String> {
            Ok("decorated test tool".to_string())
        }

        fn short_description(&self) -> String {
            "decorated test tool".to_string()
        }

        fn default_exposure(&self) -> crate::agentic::tools::framework::ToolExposure {
            self.exposure
        }

        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }

        fn is_readonly(&self) -> bool {
            self.readonly
        }

        async fn call_impl(
            &self,
            _input: &Value,
            _context: &ToolUseContext,
        ) -> crate::util::errors::NortHingResult<Vec<ToolResult>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn registry_includes_webfetch_tool() {
        let registry = create_tool_registry();
        assert!(registry.get_tool("WebFetch").is_some());
    }

    #[test]
    fn registry_includes_cron_tool() {
        let registry = create_tool_registry();
        assert!(registry.get_tool("Cron").is_some());
    }

    #[test]
    fn registry_preserves_builtin_tool_manifest_for_owner_migration() {
        let registry = create_tool_registry();
        let expected_names = vec![
            "LS",
            "Read",
            "Glob",
            "Grep",
            "Write",
            "Edit",
            "Delete",
            "ExecCommand",
            "WriteStdin",
            "ExecControl",
            "GetTime",
            "Task",
            "Skill",
            "AskUserQuestion",
            "TodoWrite",
            "get_goal",
            "create_goal",
            "update_goal",
            "CreatePlan",
            "submit_code_review",
            "GetToolSpec",
            "GetFileDiff",
            "Log",
            "SessionControl",
            "SessionMessage",
            "SessionHistory",
            "Cron",
            "WebSearch",
            "WebFetch",
            "ListMCPResources",
            "ReadMCPResource",
            "ListMCPPrompts",
            "GetMCPPrompt",
            "GenerativeUI",
            "Git",
            "ReviewPlatform",
            "InitMiniApp",
            "ControlHub",
            "ComputerUse",
            "Playbook",
        ];

        assert_eq!(
            registry.tool_names(),
            expected_names,
            "builtin tool manifest must stay stable before moving registry ownership"
        );
        let runtime_names = registry
            .all_tools()
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            runtime_names,
            registry.tool_names(),
            "runtime tool collection order must match registry key order"
        );
    }

    #[test]
    fn product_capability_provider_plan_covers_registry_manifest_in_order() {
        let assembly = northhing_product_capabilities::default_product_capability_assembly();
        let provider_tools = assembly
            .tool_provider_group_plan()
            .iter()
            .flat_map(|group| group.tool_names())
            .map(|tool_name| tool_name.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            provider_tools,
            create_tool_registry().tool_names(),
            "provider-based assembly must preserve the existing builtin registry order"
        );
    }

    #[test]
    fn product_capability_provider_plan_keeps_owner_group_order() {
        let assembly = northhing_product_capabilities::default_product_capability_assembly();
        let provider_ids = assembly
            .tool_provider_group_plan()
            .iter()
            .map(|group| group.provider_id())
            .collect::<Vec<_>>();

        assert_eq!(
            provider_ids,
            vec!["core.basic", "core.agent", "core.session", "core.integration"],
            "provider groups must stay stable until concrete tool-pack owners exist"
        );
    }

    #[test]
    fn product_tool_runtime_preserves_core_owned_registry_contract() {
        let runtime = ProductToolRuntime::default();
        let assembled_registry = runtime.create_registry();
        let compatibility_registry = create_tool_registry();

        assert_eq!(
            assembled_registry.tool_names(),
            compatibility_registry.tool_names(),
            "runtime assembly must preserve legacy create_tool_registry output"
        );
        assert_eq!(
            assembled_registry.collapsed_tool_names(),
            compatibility_registry.collapsed_tool_names(),
            "runtime assembly must preserve product collapsed-tool catalog"
        );

        for tool_name in ["Write", "Edit", "Delete"] {
            let tool = assembled_registry
                .get_tool(tool_name)
                .unwrap_or_else(|| panic!("{tool_name} tool should be registered"));
            let assistant_text = tool.render_result_for_assistant(&json!({
                "success": true,
                "file_path": "workspace/demo.txt"
            }));

            assert!(
                assistant_text.contains("snapshot system"),
                "runtime assembly must preserve snapshot wrapping for {tool_name}"
            );
        }
    }

    #[test]
    fn product_tool_runtime_owner_preserves_registry_contract() {
        let runtime = ProductToolRuntime::default();
        let owner_registry = runtime.create_registry();
        let compatibility_registry = create_tool_registry();

        assert_eq!(
            owner_registry.tool_names(),
            compatibility_registry.tool_names(),
            "product tool runtime owner must preserve legacy registry output"
        );
        assert_eq!(
            owner_registry.collapsed_tool_names(),
            compatibility_registry.collapsed_tool_names(),
            "product tool runtime owner must preserve collapsed-tool exposure"
        );
    }

    #[test]
    fn product_tool_runtime_keeps_custom_decorator_provider_contract() {
        let registry = ProductToolRuntime::with_tool_decorator(Arc::new(MarkerToolDecorator)).create_registry();
        let compatibility_registry = create_tool_registry();

        assert_eq!(
            registry.tool_names(),
            compatibility_registry.tool_names(),
            "custom decorator assembly must keep provider tool order stable"
        );
        assert_eq!(
            registry.collapsed_tool_names(),
            compatibility_registry.collapsed_tool_names(),
            "custom decorator assembly must keep collapsed exposure stable"
        );

        for tool_name in ["Write", "GetToolSpec", "WebFetch"] {
            let tool = registry
                .get_tool(tool_name)
                .unwrap_or_else(|| panic!("{tool_name} tool should be registered"));
            assert_eq!(
                tool.short_description(),
                "decorated test tool",
                "custom decorator must be applied while preserving provider installation"
            );
        }
    }

    #[test]
    fn registry_marks_collapsed_tools_for_get_tool_spec() {
        let registry = create_tool_registry();

        assert!(registry.is_tool_collapsed("WebFetch"));
        assert!(registry.is_tool_collapsed("GetFileDiff"));
        assert!(!registry.is_tool_collapsed("GetToolSpec"));
        assert!(registry.is_tool_collapsed("Git"));
        assert!(registry.is_tool_collapsed("ReviewPlatform"));
        assert!(!registry.is_tool_collapsed("InitMiniApp"));
    }

    #[test]
    fn registry_preserves_collapsed_tool_manifest_for_owner_migration() {
        let registry = create_tool_registry();

        assert_eq!(
            registry.collapsed_tool_names(),
            vec![
                "Task", // TaskTool collapsed 2026-06-23 (commit f225fc0)
                "CreatePlan",
                "GetFileDiff",
                "Log",
                "SessionControl",
                "SessionMessage",
                "SessionHistory",
                "Cron",
                "WebSearch",
                "WebFetch",
                "ListMCPResources",
                "ReadMCPResource",
                "ListMCPPrompts",
                "GetMCPPrompt",
                "GenerativeUI",
                "Git",
                "ReviewPlatform",
                "ControlHub",
                "ComputerUse",
                "Playbook",
            ],
            "collapsed tool manifest must stay stable before moving registry or manifest ownership"
        );
    }

    #[tokio::test]
    async fn registry_preserves_readonly_tool_manifest_for_owner_migration() {
        let readonly_names = super::get_readonly_tools()
            .await
            .expect("readonly tools")
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            readonly_names,
            vec![
                "LS",
                "Read",
                "Glob",
                "Grep",
                "GetTime",
                "Skill",
                "AskUserQuestion",
                "TodoWrite",
                "get_goal",
                "CreatePlan",
                "submit_code_review",
                "GetToolSpec",
                "GetFileDiff",
                "Log",
                "SessionHistory",
                "WebSearch",
                "WebFetch",
                "ListMCPResources",
                "ReadMCPResource",
                "ListMCPPrompts",
                "GetMCPPrompt",
                "GenerativeUI",
                "Playbook",
            ],
            "readonly tool manifest must stay stable before moving registry ownership"
        );
    }

    #[tokio::test]
    async fn dynamic_tool_provider_uses_explicit_provider_metadata() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(dynamic_tool("external_search", Some("github__enterprise/prod")));
        registry.register_tool(dynamic_tool("mcp__encoded__without_metadata", None));
        registry.register_tool(dynamic_tool("docs_lookup", Some("docs/provider")));

        let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");

        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| (descriptor.name.as_str(), descriptor.provider_id.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("external_search", Some("github__enterprise/prod")),
                ("docs_lookup", Some("docs/provider")),
            ],
            "dynamic provider descriptors must keep explicit metadata and registration order"
        );
        assert_eq!(descriptors[0].name, "external_search");
        assert_eq!(descriptors[0].provider_id.as_deref(), Some("github__enterprise/prod"));
    }

    #[tokio::test]
    async fn dynamic_tool_provider_preserves_descriptor_shape_and_order() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(dynamic_tool("external_search", Some("provider-a")));
        registry.register_tool(dynamic_tool("local_docs", Some("provider-b")));

        let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");

        let dynamic_descriptors = descriptors
            .iter()
            .map(|descriptor| {
                (
                    descriptor.name.as_str(),
                    descriptor.description.as_str(),
                    descriptor.input_schema.clone(),
                    descriptor.provider_id.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            dynamic_descriptors,
            vec![
                (
                    "external_search",
                    "dynamic test tool",
                    json!({ "type": "object" }),
                    Some("provider-a"),
                ),
                (
                    "local_docs",
                    "dynamic test tool",
                    json!({ "type": "object" }),
                    Some("provider-b"),
                ),
            ],
            "dynamic descriptor shape and registration order must remain stable before provider owner migration"
        );
    }

    #[tokio::test]
    async fn registering_static_tool_clears_stale_dynamic_metadata_for_same_name() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(dynamic_tool("external_search", Some("provider-a")));
        assert!(
            registry.get_dynamic_tool_info("external_search").is_some(),
            "dynamic metadata should be registered before overwrite"
        );

        registry.register_tool(dynamic_tool("external_search", None));

        assert!(
            registry.get_dynamic_tool_info("external_search").is_none(),
            "stale dynamic metadata must be removed when a static tool overwrites a dynamic tool"
        );
        let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");
        assert!(
            descriptors
                .iter()
                .all(|descriptor| descriptor.name != "external_search"),
            "stale dynamic descriptor must not leak after static overwrite"
        );
    }

    #[tokio::test]
    async fn dynamic_tool_provider_prefers_mcp_registry_metadata() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(mcp_dynamic_tool(
            "mcp__github__search_repos",
            Some("stale-provider-id"),
            "github-server-id",
            "GitHub",
            "search_repos",
        ));

        let descriptors = registry.list_dynamic_tools().await.expect("list dynamic tools");

        let descriptor = descriptors
            .into_iter()
            .find(|item| item.name == "mcp__github__search_repos")
            .expect("mcp descriptor");

        assert_eq!(descriptor.provider_id.as_deref(), Some("github-server-id"));
        assert_eq!(
            registry
                .get_dynamic_tool_info("mcp__github__search_repos")
                .expect("mcp metadata")
                .mcp
                .expect("mcp subtype metadata")
                .tool_name,
            "search_repos"
        );
    }
    #[test]
    fn registry_exposes_controlhub_and_computer_use() {
        let registry = create_tool_registry();
        assert!(
            registry.get_tool("ControlHub").is_some(),
            "ControlHub must remain registered for browser/terminal/meta control"
        );
        assert!(
            registry.get_tool("ComputerUse").is_some(),
            "ComputerUse must be registered as the dedicated desktop automation tool"
        );
    }

    #[test]
    fn registry_wraps_file_modification_tools_for_snapshot_tracking() {
        let registry = create_tool_registry();
        for tool_name in ["Write", "Edit", "Delete"] {
            let tool = registry
                .get_tool(tool_name)
                .unwrap_or_else(|| panic!("{tool_name} tool should be registered"));

            let assistant_text = tool.render_result_for_assistant(&json!({
                "success": true,
                "file_path": "workspace/demo.txt"
            }));

            assert!(
                assistant_text.contains("snapshot system"),
                "expected snapshot wrapper text for {tool_name}, got: {assistant_text}"
            );
        }

        let read_text = registry
            .get_tool("Read")
            .expect("Read tool should be registered")
            .render_result_for_assistant(&json!({
                "content": "hello",
                "file_path": "workspace/demo.txt"
            }));
        assert!(
            !read_text.contains("snapshot system"),
            "readonly tool should not be snapshot wrapped: {read_text}"
        );
    }

    // --- registry_capabilities tests ---

    #[test]
    fn capability_summary_total_matches_registered_count() {
        let summary = tool_capabilities_summary();
        let registry = create_tool_registry();

        assert_eq!(
            summary.total_count,
            registry.tool_names().len(),
            "capability summary total must match registry tool count"
        );
    }

    #[test]
    fn capability_summary_expanded_plus_collapsed_equals_total() {
        let summary = tool_capabilities_summary();

        assert_eq!(
            summary.total_count,
            summary.expanded_names.len() + summary.collapsed_names.len(),
            "expanded + collapsed must equal total"
        );
    }

    #[test]
    fn capability_summary_no_overlap_between_expanded_and_collapsed() {
        let summary = tool_capabilities_summary();

        let expanded_set: std::collections::HashSet<String> = summary.expanded_names.iter().cloned().collect();
        let collapsed_set: std::collections::HashSet<String> = summary.collapsed_names.iter().cloned().collect();

        let overlap: Vec<_> = expanded_set.intersection(&collapsed_set).collect();
        assert!(
            overlap.is_empty(),
            "expanded and collapsed sets must be disjoint, found: {:?}",
            overlap
        );
    }

    #[test]
    fn capability_summary_readonly_is_subset_of_expanded() {
        let summary = tool_capabilities_summary();
        let expanded_set: std::collections::HashSet<String> = summary.expanded_names.iter().cloned().collect();

        for name in &summary.readonly_names {
            assert!(
                expanded_set.contains(name),
                "readonly tool '{}' should be in expanded set (readonly tools must be visible)",
                name
            );
        }
    }

    #[test]
    fn capability_summary_preserves_known_collapsed_tools() {
        let summary = tool_capabilities_summary();
        let expected_collapsed = vec![
            "Task",
            "CreatePlan",
            "GetFileDiff",
            "Log",
            "SessionControl",
            "SessionMessage",
            "SessionHistory",
            "Cron",
            "WebSearch",
            "WebFetch",
            "ListMCPResources",
            "ReadMCPResource",
            "ListMCPPrompts",
            "GetMCPPrompt",
            "GenerativeUI",
            "Git",
            "ReviewPlatform",
            "ControlHub",
            "ComputerUse",
            "Playbook",
        ];

        let collapsed_set: std::collections::HashSet<String> = summary.collapsed_names.iter().cloned().collect();
        for name in expected_collapsed {
            assert!(
                collapsed_set.contains(name),
                "expected collapsed tool '{}' missing from capability summary",
                name
            );
        }
    }
}
