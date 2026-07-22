//! KernelToolsApi implementation.

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::tools::{ToolInfoDto, ToolPort, UserInputRequestDto, UserInputResponseDto};

#[async_trait]
impl northhing_kernel_api::KernelToolsApi for super::KernelFacade {
    async fn list_tools(&self) -> Result<Vec<ToolInfoDto>, KernelError> {
        // NEEDS_CONTEXT: tool registry is not exposed through a simple passthrough.
        Err(KernelError::Internal("not yet wired: list_tools".to_string()))
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
}
