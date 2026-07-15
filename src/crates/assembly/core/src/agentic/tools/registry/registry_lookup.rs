use super::registry_types::ToolRegistry;
use crate::agentic::tools::framework::{DynamicToolInfo, Tool};
use std::sync::Arc;
use tracing::trace;

impl ToolRegistry {
    /// Get tool
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.inner.get_tool(name)
    }

    pub fn get_dynamic_tool_info(&self, name: &str) -> Option<DynamicToolInfo> {
        self.inner.get_dynamic_tool_info(name)
    }

    pub fn is_tool_collapsed(&self, name: &str) -> bool {
        self.inner.is_tool_collapsed(name)
    }

    pub fn collapsed_tool_names(&self) -> Vec<String> {
        self.inner.collapsed_tool_names()
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.inner.tool_names()
    }

    /// Get all tools
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        trace!(
            "ToolRegistry::get_all_tools() called: total={}",
            self.tool_names().len()
        );
        self.inner.all_tools()
    }
}
