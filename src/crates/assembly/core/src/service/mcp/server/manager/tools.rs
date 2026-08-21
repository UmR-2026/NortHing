use super::*;

impl MCPServerManager {
    pub(super) async fn refresh_mcp_tools(
        &self,
        server_id: &str,
        server_name: &str,
        connection: Arc<MCPConnection>,
    ) -> NortHingResult<usize> {
        self.unregister_mcp_tools(server_id).await;
        self.register_mcp_tools(server_id, server_name, connection).await
    }

    /// Registers MCP tools into the global tool registry using RAII guards.
    pub(super) async fn register_mcp_tools(
        &self,
        server_id: &str,
        server_name: &str,
        connection: Arc<MCPConnection>,
    ) -> NortHingResult<usize> {
        info!(
            "Registering MCP tools: server_name={} server_id={}",
            server_name, server_id
        );

        let mut adapter = MCPToolAdapter::new();

        adapter
            .load_tools_from_server(server_id, server_name, connection)
            .await
            .map_err(|e| {
                error!(
                    "Failed to load tools from MCP server: server_name={} server_id={} error={}",
                    server_name, server_id, e
                );
                e
            })?;

        let tools = adapter.tools();
        let tool_count = tools.len();

        for tool in tools {
            debug!("Loaded MCP tool: name={} server={}", tool.name(), server_name);
        }

        let registry = crate::agentic::tools::registry::global_tool_registry();
        let mut registry_lock = registry.write().await;

        let tools_to_register = adapter.tools().to_vec();
        let guards = registry_lock.register_mcp_tools_guarded(tools_to_register);
        drop(registry_lock);

        self.server_tool_guards
            .write()
            .await
            .insert(server_id.to_string(), guards);

        info!(
            "Registered {} MCP tools (guarded): server_name={} server_id={}",
            tool_count, server_name, server_id
        );

        Ok(tool_count)
    }

    /// Unregisters MCP tools from the global tool registry by releasing the server's registration guards.
    pub(super) async fn unregister_mcp_tools(&self, server_id: &str) {
        let removed_guards = self.server_tool_guards.write().await.remove(server_id);
        drop(removed_guards);

        let registry = crate::agentic::tools::registry::global_tool_registry();
        let mut registry_lock = registry.write().await;
        registry_lock.unregister_mcp_server_tools(server_id);
        info!("Unregistered MCP tools: server_id={}", server_id);
    }
}
