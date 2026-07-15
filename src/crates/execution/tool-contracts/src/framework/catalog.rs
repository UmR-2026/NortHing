//! Tool registry-item / catalog traits and context-aware resolution runtimes.
//!
//! R37b sibling: `ToolRegistryItem`, `ContextualToolManifestItem`, catalog and
//! GetToolSpec provider traits, contextual visible-tool / manifest resolution,
//! and the catalog / GetToolSpec runtimes. Split verbatim from `framework.rs`.

use super::*;
use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashSet;
use std::marker::PhantomData;

#[async_trait]
pub trait ToolRegistryItem: Send + Sync {
    fn name(&self) -> &str;

    async fn description(&self) -> Result<String, String>;

    fn input_schema(&self) -> Value;

    fn short_description(&self) -> String {
        self.name().to_string()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Expanded
    }

    fn is_readonly(&self) -> bool {
        false
    }

    async fn is_enabled(&self) -> bool {
        true
    }

    async fn input_schema_for_model(&self) -> Value {
        self.input_schema()
    }

    fn dynamic_provider_id(&self) -> Option<&str> {
        None
    }

    fn dynamic_tool_info(&self) -> Option<DynamicToolInfo> {
        self.dynamic_provider_id().map(|provider_id| DynamicToolInfo {
            provider_id: provider_id.to_string(),
            provider_kind: None,
            mcp: None,
        })
    }
}

#[async_trait]
pub trait ContextualToolManifestItem<Context>: ToolRegistryItem
where
    Context: Sync,
{
    async fn is_available_in_context(&self, _context: &Context) -> bool {
        true
    }

    /// Return the tool description that will be sent to the AI provider.
    ///
    /// # Prefix-cache stability contract
    ///
    /// The byte output of this method must be identical across every round of
    /// the same session for the same logical tool configuration. Any variation
    /// in the returned string invalidates the provider-side prefix cache for
    /// all bytes that follow the tool-spec block, which can significantly
    /// increase per-round cost.
    ///
    /// Acceptable variation:
    /// - Remote vs local workspace, which changes at session start and then stays stable.
    /// - Model capability flags such as vision support, which are stable per session.
    /// - User-initiated config changes such as theme or locale.
    ///
    /// Forbidden variation:
    /// - Timestamps, request IDs, UUIDs, or any non-deterministic data.
    /// - Session-specific paths that change mid-session.
    /// - Anything that varies between API calls within the same session.
    async fn description_with_context(&self, _context: &Context) -> Result<String, String> {
        self.description().await
    }

    /// Return the JSON schema sent to the AI provider.
    ///
    /// Subject to the same prefix-cache stability contract as
    /// [`Self::description_with_context`]: output must be byte-stable across
    /// rounds of the same session for the same tool configuration.
    async fn input_schema_for_model_with_context(&self, _context: &Context) -> Value {
        self.input_schema_for_model().await
    }
}

#[async_trait]
pub trait ToolCatalogSnapshotProvider<Tool: ?Sized>: Send + Sync {
    async fn tool_snapshot(&self) -> Vec<ToolRef<Tool>>;
}

#[async_trait]
pub trait GetToolSpecCatalogProvider<Tool: ?Sized, Context>: Send + Sync
where
    Context: Sync,
{
    async fn collapsed_tools_for_get_tool_spec(&self, context: Option<&Context>) -> Result<Vec<ToolRef<Tool>>, String>;
}

pub fn summarize_get_tool_spec_collapsed_tools<Tool: ToolRegistryItem + ?Sized>(
    collapsed_tools: &[ToolRef<Tool>],
) -> Vec<GetToolSpecCollapsedToolSummary> {
    collapsed_tools
        .iter()
        .map(|tool| GetToolSpecCollapsedToolSummary {
            name: tool.name().to_string(),
            short_description: tool.short_description(),
        })
        .collect()
}

pub async fn build_get_tool_spec_catalog_description_from_provider<Tool, Context, Provider>(
    provider: &Provider,
    context: Option<&Context>,
) -> Result<Option<String>, String>
where
    Tool: ToolRegistryItem + ?Sized,
    Context: Sync,
    Provider: GetToolSpecCatalogProvider<Tool, Context> + ?Sized,
{
    let collapsed_tools = provider.collapsed_tools_for_get_tool_spec(context).await?;
    let summaries = summarize_get_tool_spec_collapsed_tools(&collapsed_tools);
    Ok(build_get_tool_spec_catalog_description(&summaries))
}

pub async fn resolve_readonly_enabled_tools<Tool: ToolRegistryItem + ?Sized>(
    tool_snapshot: &[ToolRef<Tool>],
) -> Vec<ToolRef<Tool>> {
    let mut readonly_tools = Vec::new();

    for tool in tool_snapshot {
        if tool.is_readonly() && tool.is_enabled().await {
            readonly_tools.push(tool.clone());
        }
    }

    readonly_tools
}

pub struct ToolCatalogRuntime<'a, Tool: ?Sized, Context, Provider: ?Sized> {
    provider: &'a Provider,
    get_tool_spec_tool_name: &'a str,
    _marker: PhantomData<fn(&Tool, &Context)>,
}

impl<'a, Tool: ?Sized, Context, Provider: ?Sized> ToolCatalogRuntime<'a, Tool, Context, Provider> {
    pub fn new(provider: &'a Provider, get_tool_spec_tool_name: &'a str) -> Self {
        Self {
            provider,
            get_tool_spec_tool_name,
            _marker: PhantomData,
        }
    }
}

impl<'a, Tool, Context, Provider> ToolCatalogRuntime<'a, Tool, Context, Provider>
where
    Tool: ToolRegistryItem + ?Sized,
    Provider: ToolCatalogSnapshotProvider<Tool> + ?Sized,
{
    pub async fn readonly_enabled_tools(&self) -> Vec<ToolRef<Tool>> {
        let tool_snapshot = self.provider.tool_snapshot().await;
        resolve_readonly_enabled_tools(&tool_snapshot).await
    }
}

impl<'a, Tool, Context, Provider> ToolCatalogRuntime<'a, Tool, Context, Provider>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: ToolCatalogSnapshotProvider<Tool> + ?Sized,
{
    pub async fn visible_tools(
        &self,
        allowed_tools: &[String],
        exposure_overrides: &IndexMap<String, ToolExposure>,
        context: &Context,
    ) -> ContextualVisibleTools<Tool> {
        resolve_contextual_visible_tools_from_provider(
            self.provider,
            allowed_tools,
            exposure_overrides,
            context,
            self.get_tool_spec_tool_name,
        )
        .await
    }

    pub async fn tool_manifest(
        &self,
        allowed_tools: &[String],
        exposure_overrides: &IndexMap<String, ToolExposure>,
        context: &Context,
    ) -> ContextualToolManifest<Tool> {
        resolve_contextual_tool_manifest_from_provider(
            self.provider,
            allowed_tools,
            exposure_overrides,
            context,
            self.get_tool_spec_tool_name,
        )
        .await
    }
}

pub async fn resolve_get_tool_spec_detail<Tool, Context>(
    collapsed_tools: &[ToolRef<Tool>],
    tool_name: &str,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> Result<GetToolSpecDetail, String>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
{
    let tool = collapsed_tools
        .iter()
        .find(|tool| tool.name() == tool_name)
        .ok_or_else(|| format!("Tool '{tool_name}' is not an available collapsed tool in the current context"))?;

    if tool.name() == get_tool_spec_tool_name {
        return Err(format!("Tool '{tool_name}' cannot inspect itself"));
    }

    let description = tool
        .description_with_context(context)
        .await
        .unwrap_or_else(|_| format!("Tool: {}", tool.name()));
    let input_schema = tool.input_schema_for_model_with_context(context).await;

    Ok(GetToolSpecDetail {
        tool_name: tool_name.to_string(),
        description,
        input_schema,
    })
}

pub async fn resolve_get_tool_spec_detail_from_provider<Tool, Context, Provider>(
    provider: &Provider,
    tool_name: &str,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> Result<GetToolSpecDetail, String>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: GetToolSpecCatalogProvider<Tool, Context> + ?Sized,
{
    let collapsed_tools = provider.collapsed_tools_for_get_tool_spec(Some(context)).await?;
    resolve_get_tool_spec_detail(&collapsed_tools, tool_name, context, get_tool_spec_tool_name).await
}

pub async fn resolve_get_tool_spec_execution_result_from_provider<Tool, Context, Provider>(
    provider: &Provider,
    input: &Value,
    loaded_collapsed_tools: &[String],
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> Result<ToolResult, GetToolSpecExecutionError>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: GetToolSpecCatalogProvider<Tool, Context> + ?Sized,
{
    match resolve_get_tool_spec_execution_plan(input, loaded_collapsed_tools)? {
        GetToolSpecExecutionPlan::DuplicateLoad(result) => Ok(result),
        GetToolSpecExecutionPlan::LoadDetail { tool_name } => {
            let detail =
                resolve_get_tool_spec_detail_from_provider(provider, tool_name, context, get_tool_spec_tool_name)
                    .await
                    .map_err(GetToolSpecExecutionError::Detail)?;
            Ok(build_get_tool_spec_detail_result(&detail))
        }
    }
}

pub struct GetToolSpecRuntime<'a, Tool: ?Sized, Context, Provider: ?Sized> {
    provider: &'a Provider,
    tool_name: &'a str,
    _marker: PhantomData<fn(&Tool, &Context)>,
}

impl<'a, Tool: ?Sized, Context, Provider: ?Sized> GetToolSpecRuntime<'a, Tool, Context, Provider> {
    pub fn new(provider: &'a Provider, tool_name: &'a str) -> Self {
        Self {
            provider,
            tool_name,
            _marker: PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        self.tool_name
    }

    pub fn short_description(&self) -> String {
        tool_spec_short_description()
    }

    pub fn input_schema(&self) -> Value {
        tool_spec_input_schema()
    }

    pub fn is_readonly(&self) -> bool {
        tool_spec_is_readonly()
    }

    pub fn is_concurrency_safe(&self, input: Option<&Value>) -> bool {
        get_tool_spec_is_concurrency_safe(input)
    }

    pub fn needs_permissions(&self, input: Option<&Value>) -> bool {
        get_tool_spec_needs_permissions(input)
    }

    pub fn render_tool_use_message(&self, input: &Value) -> String {
        render_get_tool_spec_tool_use_message(input)
    }

    pub fn validate_input(&self, input: &Value) -> ValidationResult {
        validate_get_tool_spec_input(input)
    }
}

impl<'a, Tool, Context, Provider> GetToolSpecRuntime<'a, Tool, Context, Provider>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: GetToolSpecCatalogProvider<Tool, Context> + ?Sized,
{
    pub async fn execute(
        &self,
        input: &Value,
        loaded_collapsed_tools: &[String],
        context: &Context,
    ) -> Result<ToolResult, GetToolSpecExecutionError> {
        resolve_get_tool_spec_execution_result_from_provider(
            self.provider,
            input,
            loaded_collapsed_tools,
            context,
            self.tool_name,
        )
        .await
    }

    pub async fn call_results(
        &self,
        input: &Value,
        loaded_collapsed_tools: &[String],
        context: &Context,
    ) -> Result<Vec<ToolResult>, GetToolSpecExecutionError> {
        self.execute(input, loaded_collapsed_tools, context)
            .await
            .map(|result| vec![result])
    }
}

pub async fn resolve_contextual_visible_tools_from_provider<Tool, Context, Provider>(
    provider: &Provider,
    allowed_tools: &[String],
    exposure_overrides: &IndexMap<String, ToolExposure>,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> ContextualVisibleTools<Tool>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: ToolCatalogSnapshotProvider<Tool> + ?Sized,
{
    let tool_snapshot = provider.tool_snapshot().await;
    resolve_contextual_visible_tools(
        &tool_snapshot,
        allowed_tools,
        exposure_overrides,
        context,
        get_tool_spec_tool_name,
    )
    .await
}

pub async fn resolve_contextual_tool_manifest_from_provider<Tool, Context, Provider>(
    provider: &Provider,
    allowed_tools: &[String],
    exposure_overrides: &IndexMap<String, ToolExposure>,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> ContextualToolManifest<Tool>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
    Provider: ToolCatalogSnapshotProvider<Tool> + ?Sized,
{
    let tool_snapshot = provider.tool_snapshot().await;
    resolve_contextual_tool_manifest(
        &tool_snapshot,
        allowed_tools,
        exposure_overrides,
        context,
        get_tool_spec_tool_name,
    )
    .await
}

pub async fn resolve_contextual_visible_tools<Tool, Context>(
    tool_snapshot: &[ToolRef<Tool>],
    allowed_tools: &[String],
    exposure_overrides: &IndexMap<String, ToolExposure>,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> ContextualVisibleTools<Tool>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
{
    let mut available_tool_names = HashSet::new();
    for tool in tool_snapshot {
        if tool.is_available_in_context(context).await {
            available_tool_names.insert(tool.name().to_string());
        }
    }

    let policy_tools = build_tool_manifest_policy_tools(tool_snapshot, &available_tool_names);
    let policy = resolve_tool_manifest_policy(
        &policy_tools,
        allowed_tools,
        exposure_overrides,
        get_tool_spec_tool_name,
    );
    let expanded_tools = tools_by_name(tool_snapshot, &policy.expanded_tool_names);
    let collapsed_tools = tools_by_name(tool_snapshot, &policy.collapsed_tool_names);

    ContextualVisibleTools {
        allowed_tool_names: policy.allowed_tool_names,
        expanded_tools,
        collapsed_tool_names: policy.collapsed_tool_names,
        collapsed_tools,
    }
}

pub async fn resolve_contextual_tool_manifest<Tool, Context>(
    tool_snapshot: &[ToolRef<Tool>],
    allowed_tools: &[String],
    exposure_overrides: &IndexMap<String, ToolExposure>,
    context: &Context,
    get_tool_spec_tool_name: &str,
) -> ContextualToolManifest<Tool>
where
    Tool: ContextualToolManifestItem<Context> + ?Sized,
    Context: Sync,
{
    let visible_tools = resolve_contextual_visible_tools(
        tool_snapshot,
        allowed_tools,
        exposure_overrides,
        context,
        get_tool_spec_tool_name,
    )
    .await;

    let mut manifest_items =
        Vec::with_capacity(visible_tools.expanded_tools.len() + visible_tools.collapsed_tools.len());
    for tool in &visible_tools.expanded_tools {
        let description = tool
            .description_with_context(context)
            .await
            .unwrap_or_else(|_| format!("Tool: {}", tool.name()));
        let parameters = tool.input_schema_for_model_with_context(context).await;

        manifest_items.push(PromptVisibleToolManifestItem::Expanded(ToolManifestDefinition::new(
            tool.name().to_string(),
            description,
            parameters,
        )));
    }

    for tool in &visible_tools.collapsed_tools {
        manifest_items.push(PromptVisibleToolManifestItem::Collapsed {
            name: tool.name().to_string(),
            short_description: tool.short_description(),
        });
    }

    // This prompt-visible tool-definition list is part of the request prefix.
    // Once a turn starts, enrich collapsed tools through GetToolSpec results
    // instead of mutating this list, or later rounds will lose prefix-cache
    // reuse even if the actual tool set is unchanged.
    let tool_definitions = build_prompt_visible_tool_manifest_definitions(&manifest_items);

    ContextualToolManifest {
        allowed_tool_names: visible_tools.allowed_tool_names,
        expanded_tools: visible_tools.expanded_tools,
        collapsed_tool_names: visible_tools.collapsed_tool_names,
        collapsed_tools: visible_tools.collapsed_tools,
        tool_definitions,
    }
}

fn tools_by_name<Tool: ToolRegistryItem + ?Sized>(
    tool_snapshot: &[ToolRef<Tool>],
    tool_names: &[String],
) -> Vec<ToolRef<Tool>> {
    tool_names
        .iter()
        .filter_map(|name| tool_snapshot.iter().find(|tool| tool.name() == name).cloned())
        .collect()
}
