//! KernelToolsApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::tools::{ToolInfoDto, ToolPort, UserInputRequestDto, UserInputResponseDto};

#[async_trait]
impl northhing_kernel_api::KernelToolsApi for super::KernelFacade {
    /// Lists all registered tools in the tool pipeline (including collapsed tools).
    ///
    /// Note: Collapsed vs. expanded status is a prompt surface exposure policy,
    /// not a catalog visibility semantic; all registered tools are returned here.
    async fn list_tools(&self) -> Result<Vec<ToolInfoDto>, KernelError> {
        let coordinator = self.coordinator()?;
        let tools = {
            let registry = coordinator.tool_pipeline.tool_registry.read().await;
            registry.all_tools()
        };

        let mut dtos = Vec::with_capacity(tools.len());
        for tool in tools {
            let name = tool.name().to_string();
            // Catalog listing is a read-only discovery probe; failure to resolve
            // a single tool's description should degrade to an empty string rather
            // than failing the entire listing.
            let description = tool.description().await.unwrap_or_default();
            let input_schema = Some(tool.input_schema());
            dtos.push(ToolInfoDto {
                id: name.clone(),
                name,
                description,
                input_schema,
            });
        }

        dtos.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(dtos)
    }

    async fn register_tool(&self, _tool: std::sync::Arc<dyn ToolPort>) -> Result<(), KernelError> {
        // NEEDS_CONTEXT: ACP tool registration requires tool pipeline wiring.
        Err(KernelError::Internal("not yet wired: register_tool".to_string()))
    }

    async fn request_user_input(
        &self,
        _request: UserInputRequestDto,
    ) -> Result<UserInputResponseDto, KernelError> {
        // NEEDS_CONTEXT: user input flow requires UI integration.
        Err(KernelError::Internal("not yet wired: request_user_input".to_string()))
    }

    async fn respond_to_tool_confirmation(
        &self,
        tool_id: &str,
        approved: bool,
        reason: Option<String>,
    ) -> Result<(), KernelError> {
        let coordinator = match self.coordinator() {
            Ok(c) => c,
            Err(e) => {
                return Err(KernelError::Runtime(format!(
                    "kernel facade not initialized — init_core not called: {e}"
                )));
            }
        };
        if approved {
            coordinator.confirm_tool(tool_id, None).await
        } else {
            coordinator
                .reject_tool(tool_id, reason.unwrap_or_default())
                .await
        }
        .map_err(|e| KernelError::Runtime(format!("respond_to_tool_confirmation failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::KernelToolsApi;

    #[tokio::test]
    async fn test_respond_to_tool_confirmation_returns_runtime_err_before_init() {
        let facade = super::super::KernelFacade::new();
        let result = facade
            .respond_to_tool_confirmation("tool-123", true, None)
            .await;
        match result {
            Err(KernelError::Runtime(_)) => {}
            Err(other) => panic!("expected KernelError::Runtime before init, got {:?}", other),
            Ok(_) => panic!("respond_to_tool_confirmation should return Err before init"),
        }
    }
}
