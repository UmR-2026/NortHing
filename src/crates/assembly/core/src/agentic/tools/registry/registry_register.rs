use super::registry_types::{ToolRef, ToolRegistry};
use tracing::{debug, info, warn};

impl ToolRegistry {
    /// Dynamically register MCP tools
    pub fn register_mcp_tools(&mut self, tools: Vec<ToolRef>) {
        let tool_count = tools.len();
        info!("Registering MCP tools: count={}", tool_count);

        let before_count = self.tool_names().len();
        debug!("Tool count before registration: {}", before_count);

        for (index, tool) in tools.into_iter().enumerate() {
            let name = tool.name().to_string();
            debug!("Registering MCP tool [{}/{}]: {}", index + 1, tool_count, name);

            // Check if a tool with the same name already exists
            if self.get_tool(&name).is_some() {
                warn!("Tool already exists, will be overwritten: tool_name={}", name);
            }

            self.register_tool(tool);
            debug!("MCP tool registered: tool_name={}", name);
        }

        let after_count = self.tool_names().len();
        let added_count = after_count - before_count;

        info!(
            "MCP tools registration completed: before={}, after={}, added={}",
            before_count, after_count, added_count
        );
    }

    /// Remove all tools from the MCP server
    pub fn unregister_mcp_server_tools(&mut self, server_id: &str) {
        let removed_tool_names = self
            .tool_names()
            .into_iter()
            .filter(|name| {
                self.get_dynamic_tool_info(name)
                    .and_then(|info| info.mcp)
                    .is_some_and(|mcp| mcp.server_id == server_id)
            })
            .collect::<Vec<_>>();

        self.inner.unregister_mcp_server_tools(server_id);

        for key in removed_tool_names {
            info!("Unregistering dynamic tool: tool_name={}", key);
        }
    }

    /// Remove all tools whose registry name starts with the given prefix.
    pub fn unregister_tools_by_prefix(&mut self, prefix: &str) -> usize {
        let removed_tool_names = self
            .tool_names()
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .collect::<Vec<_>>();
        let count = self.inner.unregister_tools_by_prefix(prefix);

        for key in removed_tool_names {
            info!("Unregistering dynamic tool: tool_name={}", key);
        }

        count
    }

    /// Register a single tool
    pub fn register_tool(&mut self, tool: ToolRef) {
        self.inner.register_tool(tool);
    }
}
