pub use northhing_agent_tools::{
    build_collapsed_tool_stub_definition, build_get_tool_spec_assistant_detail, build_get_tool_spec_detail_result,
    build_get_tool_spec_duplicate_load_hint, build_get_tool_spec_duplicate_load_result, build_northhing_runtime_uri,
    build_prompt_visible_tool_manifest_definitions, build_tool_path_policy_denial_message,
    build_tool_runtime_artifact_reference, build_tool_session_runtime_artifact_reference,
    collect_loaded_collapsed_tool_names, get_tool_spec_is_concurrency_safe, get_tool_spec_needs_permissions,
    is_northhing_runtime_uri, is_remote_posix_path_within_root, is_tool_path_allowed_by_resolved_roots,
    normalize_host_path, normalize_runtime_relative_path, parse_northhing_runtime_uri,
    posix_resolve_path_with_workspace, posix_style_path_is_absolute, render_get_tool_spec_tool_use_message,
    resolve_contextual_tool_manifest, resolve_contextual_tool_manifest_from_provider, resolve_get_tool_spec_detail,
    resolve_get_tool_spec_detail_from_provider, resolve_get_tool_spec_execution_result_from_provider,
    resolve_host_path_with_workspace, resolve_readonly_enabled_tools, resolve_tool_manifest_policy,
    resolve_tool_path_with_context, resolve_workspace_tool_path, sort_tool_manifest_definitions,
    summarize_get_tool_spec_collapsed_tools, tool_path_is_effectively_absolute, tool_spec_input_schema,
    tool_spec_is_readonly, tool_spec_short_description, validate_collapsed_tool_usage, validate_get_tool_spec_input,
    validate_tool_allowed_by_list, validate_tool_execution_admission, DynamicMcpToolInfo, DynamicToolInfo,
    GetToolSpecCollapsedToolSummary, GetToolSpecExecutionError, GetToolSpecExecutionPlan, GetToolSpecLoadObservation,
    GetToolSpecRuntime, InputValidator, PromptVisibleToolManifestItem, ToolContextFacts,
    ToolExecutionAdmissionRejection, ToolExecutionAdmissionRequest, ToolExposure, ToolImageAttachment,
    ToolManifestDefinition, ToolManifestPolicyTool, ToolPathBackend, ToolPathOperation, ToolPathResolution,
    ToolRenderOptions, ToolResult, ToolRuntimeRestrictions, ToolWorkspaceKind, ValidationResult,
    GET_TOOL_SPEC_TOOL_NAME,
};
pub use northhing_agent_tools::{
    build_invalid_tool_call_error_message, build_tool_call_truncation_recovery_notice,
    build_tool_execution_error_presentation, build_user_steering_interrupted_presentation, is_write_like_tool_name,
    render_tool_result_for_assistant, truncate_raw_tool_arguments_preview_to, truncate_tool_arguments_preview,
    TOOL_ERROR_ARGUMENTS_PREVIEW_BYTES, USER_STEERING_INTERRUPTED_MESSAGE,
};
pub use northhing_agent_tools::{
    build_persisted_tool_output_message, count_tool_result_lines, file_tool_guidance_message,
    generate_tool_result_preview, is_file_tool_guidance_message, sanitize_tool_result_file_component,
    select_tool_result_indices_for_persistence, tool_result_is_persisted_output, PersistedToolOutput,
    ToolResultPersistenceCandidate, FILE_TOOL_GUIDANCE_PREFIX, PERSISTED_OUTPUT_TAG, TOOL_RESULT_PREVIEW_CHARS,
};
pub use northhing_agent_tools::{
    file_read_facts_are_fresh, file_read_facts_content_matches, normalize_tool_file_content, FileReadFreshnessFacts,
};
pub use northhing_agent_tools::{
    materialize_static_tool_provider_groups, ContextualToolManifestItem, DynamicToolDescriptor, DynamicToolProvider,
    GetToolSpecCatalogProvider, PortResult, PortableToolContextProvider, StaticToolMaterializationError,
    StaticToolProvider, StaticToolProviderFactory, StaticToolProviderGroup, StaticToolProviderPlan, ToolCatalogRuntime,
    ToolCatalogSnapshotProvider, ToolDecorator, ToolDecoratorRef, ToolRegistry, ToolRegistryItem, ToolRuntimeAssembly,
};
pub use serde_json::json;
pub use std::path::PathBuf;
pub use std::sync::Arc;

pub struct TestProviderPlan {
    pub provider_id: &'static str,
    pub tool_names: &'static [&'static str],
}

impl StaticToolProviderPlan for TestProviderPlan {
    fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    fn tool_names(&self) -> &'static [&'static str] {
        self.tool_names
    }
}

#[derive(Clone)]
pub struct RegistryMarkerTool {
    pub name: String,
    pub provider_id: Option<String>,
    pub exposure: ToolExposure,
    pub readonly: bool,
    pub enabled: bool,
}

#[async_trait::async_trait]
impl ToolRegistryItem for RegistryMarkerTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn description(&self) -> Result<String, String> {
        Ok("marker tool".to_string())
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({ "type": "object" })
    }

    fn default_exposure(&self) -> ToolExposure {
        self.exposure
    }

    fn is_readonly(&self) -> bool {
        self.readonly
    }

    async fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn input_schema_for_model(&self) -> serde_json::Value {
        self.input_schema()
    }

    fn dynamic_tool_info(&self) -> Option<DynamicToolInfo> {
        self.provider_id.as_ref().map(|provider_id| DynamicToolInfo {
            provider_id: provider_id.clone(),
            provider_kind: None,
            mcp: None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ManifestTestContext {
    pub agent: &'static str,
}

#[derive(Clone)]
pub struct ContextualManifestTool {
    pub name: String,
    pub exposure: ToolExposure,
    pub available_for_agent: Option<&'static str>,
}

#[async_trait::async_trait]
impl ToolRegistryItem for ContextualManifestTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn description(&self) -> Result<String, String> {
        Ok(format!("{} default description", self.name))
    }

    fn short_description(&self) -> String {
        format!("{} short description", self.name)
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({ "type": "object" })
    }

    fn default_exposure(&self) -> ToolExposure {
        self.exposure
    }
}

#[async_trait::async_trait]
impl ContextualToolManifestItem<ManifestTestContext> for ContextualManifestTool {
    async fn is_available_in_context(&self, context: &ManifestTestContext) -> bool {
        self.available_for_agent.is_none_or(|agent| agent == context.agent)
    }

    async fn description_with_context(&self, context: &ManifestTestContext) -> Result<String, String> {
        Ok(format!("{} description for {}", self.name, context.agent))
    }

    async fn input_schema_for_model_with_context(&self, context: &ManifestTestContext) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent": {
                    "const": context.agent
                }
            }
        })
    }
}

pub fn registry_marker_tool(name: &str, provider_id: Option<&str>) -> Arc<RegistryMarkerTool> {
    registry_marker_tool_with_exposure(name, provider_id, ToolExposure::Expanded)
}

pub fn registry_marker_tool_with_exposure(
    name: &str,
    provider_id: Option<&str>,
    exposure: ToolExposure,
) -> Arc<RegistryMarkerTool> {
    registry_marker_tool_with_access(name, provider_id, exposure, false, true)
}

pub fn registry_marker_tool_with_access(
    name: &str,
    provider_id: Option<&str>,
    exposure: ToolExposure,
    readonly: bool,
    enabled: bool,
) -> Arc<RegistryMarkerTool> {
    Arc::new(RegistryMarkerTool {
        name: name.to_string(),
        provider_id: provider_id.map(str::to_string),
        exposure,
        readonly,
        enabled,
    })
}

pub fn contextual_manifest_tool(
    name: &str,
    exposure: ToolExposure,
    available_for_agent: Option<&'static str>,
) -> Arc<ContextualManifestTool> {
    Arc::new(ContextualManifestTool {
        name: name.to_string(),
        exposure,
        available_for_agent,
    })
}

pub struct ContextualManifestSnapshotProvider {
    pub tools: Vec<Arc<ContextualManifestTool>>,
}

pub struct RegistryMarkerSnapshotProvider {
    pub tools: Vec<Arc<RegistryMarkerTool>>,
}

pub struct ErroringGetToolSpecProvider;

#[async_trait::async_trait]
impl ToolCatalogSnapshotProvider<ContextualManifestTool> for ContextualManifestSnapshotProvider {
    async fn tool_snapshot(&self) -> Vec<Arc<ContextualManifestTool>> {
        self.tools.clone()
    }
}

#[async_trait::async_trait]
impl ToolCatalogSnapshotProvider<RegistryMarkerTool> for RegistryMarkerSnapshotProvider {
    async fn tool_snapshot(&self) -> Vec<Arc<RegistryMarkerTool>> {
        self.tools.clone()
    }
}

#[async_trait::async_trait]
impl GetToolSpecCatalogProvider<ContextualManifestTool, ManifestTestContext> for ContextualManifestSnapshotProvider {
    async fn collapsed_tools_for_get_tool_spec(
        &self,
        context: Option<&ManifestTestContext>,
    ) -> Result<Vec<Arc<ContextualManifestTool>>, String> {
        let tools = match context {
            Some(context) => {
                let mut tools = Vec::new();
                for tool in &self.tools {
                    if tool.default_exposure() == ToolExposure::Collapsed && tool.is_available_in_context(context).await
                    {
                        tools.push(tool.clone());
                    }
                }
                tools
            }
            None => self
                .tools
                .iter()
                .filter(|tool| tool.default_exposure() == ToolExposure::Collapsed)
                .cloned()
                .collect(),
        };

        Ok(tools)
    }
}

#[async_trait::async_trait]
impl GetToolSpecCatalogProvider<ContextualManifestTool, ManifestTestContext> for ErroringGetToolSpecProvider {
    async fn collapsed_tools_for_get_tool_spec(
        &self,
        _context: Option<&ManifestTestContext>,
    ) -> Result<Vec<Arc<ContextualManifestTool>>, String> {
        Err("provider should not be called for duplicate-load execution".to_string())
    }
}

pub struct RegistryMarkerProvider {
    pub provider_id: &'static str,
    pub tools: Vec<Arc<RegistryMarkerTool>>,
}

impl StaticToolProvider<RegistryMarkerTool> for RegistryMarkerProvider {
    fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    fn tools(&self) -> Vec<Arc<RegistryMarkerTool>> {
        self.tools.clone()
    }
}
