use super::registry_types::{ProductToolDecoratorRef, ToolRegistry};
use crate::agentic::tools::framework::Tool;
use crate::agentic::tools::product_runtime::resolve_product_readonly_enabled_tools;
use crate::util::errors::NortHingResult;
use northhing_agent_tools::ToolRegistry as AgentToolRegistry;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock as TokioRwLock;
use tracing::info;

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        crate::agentic::tools::product_runtime::ProductToolRuntime::default().create_registry()
    }

    /// Create a registry with an injected decoration boundary.
    ///
    /// The default production decorator preserves snapshot-aware wrapping while
    /// allowing future owner crates to replace this concrete service coupling
    /// through the `northhing-runtime-ports` interface.
    pub fn with_tool_decorator(tool_decorator: ProductToolDecoratorRef) -> Self {
        crate::agentic::tools::product_runtime::ProductToolRuntime::with_tool_decorator(tool_decorator)
            .create_registry()
    }

    pub(in crate::agentic::tools) fn from_inner(inner: AgentToolRegistry<dyn Tool>) -> Self {
        Self { inner }
    }
}

// Global tool registry instance
static GLOBAL_TOOL_REGISTRY: OnceLock<Arc<TokioRwLock<ToolRegistry>>> = OnceLock::new();

/// Get global tool registry
pub fn global_tool_registry() -> Arc<TokioRwLock<ToolRegistry>> {
    GLOBAL_TOOL_REGISTRY
        .get_or_init(|| {
            info!("Initializing global tool registry");
            Arc::new(TokioRwLock::new(ToolRegistry::new()))
        })
        .clone()
}

/// Get all tools from the snapshot-aware global registry.
pub async fn all_tools() -> Vec<Arc<dyn Tool>> {
    let registry = global_tool_registry();
    let registry_lock = registry.read().await;
    registry_lock.all_tools()
}

/// Get readonly tools
pub async fn get_readonly_tools() -> NortHingResult<Vec<Arc<dyn Tool>>> {
    Ok(resolve_product_readonly_enabled_tools().await)
}

/// Create default tool registry - factory function
pub fn create_tool_registry() -> ToolRegistry {
    ToolRegistry::new()
}

/// Backward-compatible alias for callers that expect MCP tools to be included.
pub async fn get_all_registered_tools() -> Vec<Arc<dyn Tool>> {
    all_tools().await
}

/// Get all registered tool names
pub async fn get_all_registered_tool_names() -> Vec<String> {
    let all_tools = get_all_registered_tools().await;
    all_tools.into_iter().map(|tool| tool.name().to_string()).collect()
}

pub async fn get_readonly_registered_tool_names() -> Vec<String> {
    get_readonly_tools()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|tool| tool.name().to_string())
        .collect()
}
