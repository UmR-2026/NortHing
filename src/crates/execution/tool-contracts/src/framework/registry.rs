//! Static tool providers, decorators, registry assembly, and tool registry.
//!
//! R37b sibling: tool ref/decorator aliases, snapshot decorator, static tool
//! provider contracts and materialization, `ToolRuntimeAssembly`, and
//! `ToolRegistry` with its dynamic-tool provider impl. Split verbatim from
//! `framework.rs`.

use super::*;
use crate::{DynamicToolDescriptor, DynamicToolProvider, PortError, PortErrorKind, PortResult, ToolDecorator};
use async_trait::async_trait;
use indexmap::IndexMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct DynamicToolMetadata {
    provider_id: String,
    info: DynamicToolInfo,
}

struct IdentityToolDecorator;

impl<Tool> ToolDecorator<Tool> for IdentityToolDecorator {
    fn decorate(&self, tool: Tool) -> Tool {
        tool
    }
}

pub type ToolRef<Tool> = Arc<Tool>;
pub type ToolDecoratorRef<Tool> = Arc<dyn ToolDecorator<ToolRef<Tool>>>;

pub trait SnapshotToolWrapper<Tool: ?Sized>: Send + Sync {
    fn wrap_for_snapshot_tracking(&self, tool: ToolRef<Tool>) -> ToolRef<Tool>;
}

pub type SnapshotToolWrapperRef<Tool> = Arc<dyn SnapshotToolWrapper<Tool>>;

pub struct SnapshotToolDecorator<Tool: ?Sized> {
    wrapper: SnapshotToolWrapperRef<Tool>,
}

impl<Tool: ?Sized> SnapshotToolDecorator<Tool> {
    pub fn new(wrapper: SnapshotToolWrapperRef<Tool>) -> Self {
        Self { wrapper }
    }
}

impl<Tool: ?Sized> ToolDecorator<ToolRef<Tool>> for SnapshotToolDecorator<Tool> {
    fn decorate(&self, tool: ToolRef<Tool>) -> ToolRef<Tool> {
        self.wrapper.wrap_for_snapshot_tracking(tool)
    }
}

pub trait StaticToolProvider<Tool: ?Sized>: Send + Sync {
    fn provider_id(&self) -> &'static str;

    fn tools(&self) -> Vec<ToolRef<Tool>>;
}

pub trait StaticToolProviderPlan {
    fn provider_id(&self) -> &'static str;

    fn tool_names(&self) -> &'static [&'static str];
}

pub trait StaticToolProviderFactory<Tool: ?Sized> {
    fn materialize_tool(&self, tool_name: &str) -> Option<ToolRef<Tool>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticToolMaterializationError {
    UnknownTool {
        provider_id: &'static str,
        tool_name: &'static str,
    },
}

impl std::fmt::Display for StaticToolMaterializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool { provider_id, tool_name } => {
                write!(f, "unknown static tool {tool_name} in provider group {provider_id}")
            }
        }
    }
}

impl std::error::Error for StaticToolMaterializationError {}

pub struct StaticToolProviderGroup<Tool: ?Sized> {
    provider_id: &'static str,
    tools: Vec<ToolRef<Tool>>,
}

impl<Tool: ?Sized> std::fmt::Debug for StaticToolProviderGroup<Tool> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticToolProviderGroup")
            .field("provider_id", &self.provider_id)
            .field("tool_count", &self.tools.len())
            .finish()
    }
}

impl<Tool: ?Sized> StaticToolProviderGroup<Tool> {
    pub fn new(provider_id: &'static str, tools: Vec<ToolRef<Tool>>) -> Self {
        Self { provider_id, tools }
    }
}

impl<Tool: ?Sized + Send + Sync> StaticToolProvider<Tool> for StaticToolProviderGroup<Tool> {
    fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    fn tools(&self) -> Vec<ToolRef<Tool>> {
        self.tools.clone()
    }
}

pub fn materialize_static_tool_provider_groups<Tool, Plan, Factory>(
    plans: &[Plan],
    factory: &Factory,
) -> Result<Vec<StaticToolProviderGroup<Tool>>, StaticToolMaterializationError>
where
    Tool: ?Sized,
    Plan: StaticToolProviderPlan,
    Factory: StaticToolProviderFactory<Tool> + ?Sized,
{
    let mut providers = Vec::new();
    for plan in plans {
        let provider_id = plan.provider_id();
        let mut tools = Vec::new();
        for tool_name in plan.tool_names() {
            let tool = factory
                .materialize_tool(tool_name)
                .ok_or(StaticToolMaterializationError::UnknownTool { provider_id, tool_name })?;
            tools.push(tool);
        }
        providers.push(StaticToolProviderGroup::new(provider_id, tools));
    }
    Ok(providers)
}

pub struct ToolRuntimeAssembly<Tool: ToolRegistryItem + ?Sized> {
    tool_decorator: ToolDecoratorRef<Tool>,
}

impl<Tool: ToolRegistryItem + ?Sized> Default for ToolRuntimeAssembly<Tool> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Tool: ToolRegistryItem + ?Sized> ToolRuntimeAssembly<Tool> {
    pub fn new() -> Self {
        Self::with_tool_decorator(Arc::new(IdentityToolDecorator))
    }

    pub fn with_tool_decorator(tool_decorator: ToolDecoratorRef<Tool>) -> Self {
        Self { tool_decorator }
    }

    pub fn create_registry_from_static_providers<Provider>(&self, providers: &[Provider]) -> ToolRegistry<Tool>
    where
        Provider: StaticToolProvider<Tool>,
    {
        let mut registry = ToolRegistry::with_tool_decorator(self.tool_decorator.clone());
        for provider in providers {
            registry.install_static_provider(provider);
        }
        registry
    }

    pub fn create_registry_from_static_provider_plans<Plan, Factory>(
        &self,
        plans: &[Plan],
        factory: &Factory,
    ) -> Result<ToolRegistry<Tool>, StaticToolMaterializationError>
    where
        Plan: StaticToolProviderPlan,
        Factory: StaticToolProviderFactory<Tool> + ?Sized,
    {
        let providers = materialize_static_tool_provider_groups(plans, factory)?;
        Ok(self.create_registry_from_static_providers(&providers))
    }
}

pub struct ToolRegistry<Tool: ToolRegistryItem + ?Sized> {
    tools: IndexMap<String, ToolRef<Tool>>,
    dynamic_tools: IndexMap<String, DynamicToolMetadata>,
    tool_decorator: ToolDecoratorRef<Tool>,
}

impl<Tool: ToolRegistryItem + ?Sized> Default for ToolRegistry<Tool> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Tool: ToolRegistryItem + ?Sized> ToolRegistry<Tool> {
    pub fn new() -> Self {
        Self::with_tool_decorator(Arc::new(IdentityToolDecorator))
    }

    pub fn with_tool_decorator(tool_decorator: ToolDecoratorRef<Tool>) -> Self {
        Self {
            tools: IndexMap::new(),
            dynamic_tools: IndexMap::new(),
            tool_decorator,
        }
    }

    pub fn register_tool(&mut self, tool: ToolRef<Tool>) {
        let tool = self.tool_decorator.decorate(tool);
        let name = tool.name().to_string();
        let dynamic_info = tool.dynamic_tool_info().and_then(|info| {
            if info.provider_id.trim().is_empty() {
                None
            } else {
                Some(info)
            }
        });

        if let Some(info) = dynamic_info {
            self.dynamic_tools.insert(
                name.clone(),
                DynamicToolMetadata {
                    provider_id: info.provider_id.clone(),
                    info,
                },
            );
        } else {
            self.dynamic_tools.shift_remove(&name);
        }
        self.tools.insert(name, tool);
    }

    pub fn install_static_provider<Provider>(&mut self, provider: &Provider)
    where
        Provider: StaticToolProvider<Tool> + ?Sized,
    {
        for tool in provider.tools() {
            self.register_tool(tool);
        }
    }

    pub fn unregister_mcp_server_tools(&mut self, server_id: &str) {
        let to_remove = self
            .dynamic_tools
            .iter()
            .filter(|(_, metadata)| {
                metadata
                    .info
                    .mcp
                    .as_ref()
                    .is_some_and(|info| info.server_id == server_id)
            })
            .map(|(tool_name, _)| tool_name.clone())
            .collect::<Vec<_>>();

        for key in to_remove {
            self.tools.shift_remove(&key);
            self.dynamic_tools.shift_remove(&key);
        }
    }

    pub fn unregister_tools_by_prefix(&mut self, prefix: &str) -> usize {
        let to_remove = self
            .tools
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect::<Vec<_>>();
        let count = to_remove.len();

        for key in to_remove {
            self.tools.shift_remove(&key);
            self.dynamic_tools.shift_remove(&key);
        }

        count
    }

    pub fn get_tool(&self, name: &str) -> Option<ToolRef<Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn get_dynamic_tool_info(&self, name: &str) -> Option<DynamicToolInfo> {
        self.dynamic_tools.get(name).map(|metadata| metadata.info.clone())
    }

    pub fn is_tool_collapsed(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .is_some_and(|tool| tool.default_exposure() == ToolExposure::Collapsed)
    }

    pub fn collapsed_tool_names(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter(|&(_name, tool)| tool.default_exposure() == ToolExposure::Collapsed)
            .map(|(name, _tool)| name.clone())
            .collect()
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn all_tools(&self) -> Vec<ToolRef<Tool>> {
        self.tools.values().cloned().collect()
    }
}

#[async_trait]
impl<Tool: ToolRegistryItem + ?Sized> DynamicToolProvider for ToolRegistry<Tool> {
    async fn list_dynamic_tools(&self) -> PortResult<Vec<DynamicToolDescriptor>> {
        let mut descriptors = Vec::new();

        for (name, tool) in self.tools.iter() {
            let Some(metadata) = self.dynamic_tools.get(name) else {
                continue;
            };
            let description = tool
                .description()
                .await
                .map_err(|error| PortError::new(PortErrorKind::Backend, error))?;

            descriptors.push(DynamicToolDescriptor {
                name: tool.name().to_string(),
                description,
                input_schema: tool.input_schema_for_model().await,
                provider_id: Some(metadata.provider_id.clone()),
            });
        }

        Ok(descriptors)
    }
}
