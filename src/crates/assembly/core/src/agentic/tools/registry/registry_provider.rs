use super::registry_types::ToolRegistry;
use northhing_agent_tools::{DynamicToolDescriptor, DynamicToolProvider, PortResult};

#[async_trait::async_trait]
impl DynamicToolProvider for ToolRegistry {
    async fn list_dynamic_tools(&self) -> PortResult<Vec<DynamicToolDescriptor>> {
        self.inner.list_dynamic_tools().await
    }
}
