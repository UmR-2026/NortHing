//! Snapshot manager — tool-wrapping path.
//!
//! Owns `WrappedTool` (the `Tool` trait impl that intercepts file-modifying
//! tools, records an entry into the snapshot system, runs the original tool,
//! then completes the snapshot entry) plus the two free functions that
//! produce wrapped tool lists for the registry: `wrap_tool_for_snapshot_tracking`
//! and `get_snapshot_wrapped_tools`.
//!
//! Capture / invalidate / query / lock paths live in their respective sibling
//! impl blocks. This file is the largest sibling (~370 lines) because
//! `WrappedTool` delegates every `Tool` trait method to the inner tool and
//! the `call_impl` override is the heart of the snapshot integration.
//!
//! This is an R46c split sibling of `manager.rs`.

use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, warn};

use crate::agentic::tools::framework::{DynamicToolInfo, Tool, ToolExposure, ToolResult, ToolUseContext};
use crate::agentic::tools::registry::ToolRegistry;
use crate::service::remote_ssh::workspace_state::is_remote_path;

/// Ensures the registry always exposes the same tool implementation that will be
/// executed at runtime. File-modifying tools are wrapped once at registration time
/// so tool definitions, permission checks, and execution all share one source of truth.
pub fn wrap_tool_for_snapshot_tracking(tool: Arc<dyn Tool>) -> Arc<dyn Tool> {
    if WrappedTool::is_file_modification_tool_name(tool.name()) {
        Arc::new(WrappedTool::new(tool))
    } else {
        tool
    }
}

/// Compatibility helper that returns a fresh snapshot-aware tool list.
pub fn snapshot_wrapped_tools() -> Vec<Arc<dyn Tool>> {
    ToolRegistry::new().all_tools()
}

/// Wrapped tool
///
/// Wraps file-modification tools with snapshot functionality.
struct WrappedTool {
    original_tool: Arc<dyn Tool>,
}

impl WrappedTool {
    fn new(original_tool: Arc<dyn Tool>) -> Self {
        Self { original_tool }
    }

    fn is_file_modification_tool_name(tool_name: &str) -> bool {
        [
            "Write",
            "Edit",
            "Delete",
            "write_file",
            "edit_file",
            "create_file",
            "delete_file",
            "rename_file",
            "move_file",
            "search_replace",
        ]
        .contains(&tool_name)
    }
}

#[async_trait]
impl Tool for WrappedTool {
    fn name(&self) -> &str {
        self.original_tool.name()
    }

    async fn description(&self) -> crate::util::errors::NortHingResult<String> {
        Ok(self.original_tool.description().await?)
    }

    async fn description_with_context(
        &self,
        context: Option<&ToolUseContext>,
    ) -> crate::util::errors::NortHingResult<String> {
        self.original_tool.description_with_context(context).await
    }

    fn short_description(&self) -> String {
        self.original_tool.short_description()
    }

    fn default_exposure(&self) -> ToolExposure {
        self.original_tool.default_exposure()
    }

    fn input_schema(&self) -> Value {
        self.original_tool.input_schema()
    }

    async fn input_schema_for_model(&self) -> Value {
        self.original_tool.input_schema_for_model().await
    }

    async fn input_schema_for_model_with_context(
        &self,
        context: Option<&crate::agentic::tools::framework::ToolUseContext>,
    ) -> Value {
        self.original_tool.input_schema_for_model_with_context(context).await
    }

    fn input_json_schema(&self) -> Option<Value> {
        self.original_tool.input_json_schema()
    }

    fn dynamic_provider_id(&self) -> Option<&str> {
        self.original_tool.dynamic_provider_id()
    }

    fn dynamic_tool_info(&self) -> Option<DynamicToolInfo> {
        self.original_tool.dynamic_tool_info()
    }

    fn user_facing_name(&self) -> String {
        self.original_tool.user_facing_name().to_string()
    }

    async fn is_enabled(&self) -> bool {
        self.original_tool.is_enabled().await
    }

    async fn is_available_in_context(&self, context: Option<&ToolUseContext>) -> bool {
        self.original_tool.is_available_in_context(context).await
    }

    fn is_readonly(&self) -> bool {
        self.original_tool.is_readonly()
    }

    fn is_concurrency_safe(&self, input: Option<&Value>) -> bool {
        self.original_tool.is_concurrency_safe(input)
    }

    fn needs_permissions(&self, input: Option<&Value>) -> bool {
        self.original_tool.needs_permissions(input)
    }

    async fn validate_input(
        &self,
        input: &Value,
        context: Option<&ToolUseContext>,
    ) -> crate::agentic::tools::framework::ValidationResult {
        let original_validation = self.original_tool.validate_input(input, context).await;

        if !original_validation.result {
            return original_validation;
        }

        original_validation
    }

    fn render_result_for_assistant(&self, output: &Value) -> String {
        let original_render = self.original_tool.render_result_for_assistant(output);
        format!(
            "{}\n\nModification recorded to snapshot system, can be reviewed and managed in the review panel.",
            original_render
        )
    }

    fn render_tool_use_message(
        &self,
        input: &Value,
        options: &crate::agentic::tools::framework::ToolRenderOptions,
    ) -> String {
        let original_message = self.original_tool.render_tool_use_message(input, options);
        original_message.to_string()
    }

    fn render_tool_use_rejected_message(&self) -> String {
        self.original_tool.render_tool_use_rejected_message().to_string()
    }

    fn render_tool_result_message(&self, output: &Value) -> String {
        let original_message = self.original_tool.render_tool_result_message(output);
        format!("{} recorded to snapshot", original_message)
    }

    async fn call_impl(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> crate::util::errors::NortHingResult<Vec<ToolResult>> {
        if Self::is_file_modification_tool_name(self.name()) {
            debug!("Intercepting file modification tool: tool_name={}", self.name());

            match self.handle_file_modification_internal(input, context).await {
                Ok(results) => {
                    return Ok(results);
                }
                Err(e) => {
                    warn!(
                        "Snapshot processing failed, falling back to original tool: tool_name={} error={}",
                        self.name(),
                        e
                    );
                    let error_msg = format!("{}", e);
                    if error_msg.contains("file not found") || error_msg.contains("not exist") {
                        warn!("Possible workspace path mismatch, check snapshot workspace and global workspace consistency");
                    }
                }
            }
        }

        self.original_tool.call(input, context).await
    }
}

impl WrappedTool {
    /// Handles a file-modification tool.
    async fn handle_file_modification_internal(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> crate::util::errors::NortHingResult<Vec<ToolResult>> {
        let session_id = context.session_id.clone().ok_or_else(|| {
            crate::util::errors::NortHingError::Tool("session_id is required in ToolUseContext".to_string())
        })?;

        let raw_path = match self.extract_file_path_simple(input) {
            Ok(path) => path,
            Err(e) => return Err(crate::util::errors::NortHingError::Tool(e.to_string())),
        };

        let snapshot_workspace = context.workspace_root().map(PathBuf::from).ok_or_else(|| {
            crate::util::errors::NortHingError::Tool(
                "workspace is required in ToolUseContext for snapshot tracking".to_string(),
            )
        })?;

        // Remote workspaces: skip snapshot tracking, just execute the tool directly
        if is_remote_path(snapshot_workspace.to_string_lossy().as_ref()).await {
            debug!(
                "Skipping snapshot for remote workspace: workspace={}",
                snapshot_workspace.display()
            );
            return self.original_tool.call(input, context).await;
        }

        let snapshot_manager =
            super::manager_registry::get_or_create_snapshot_manager(snapshot_workspace.clone(), None)
                .await
                .map_err(|e| crate::util::errors::NortHingError::Tool(e.to_string()))?;

        let file_path = if raw_path.is_absolute() {
            raw_path.clone()
        } else {
            snapshot_workspace.join(&raw_path)
        };

        let is_create_tool = matches!(self.name(), "Write" | "write_file" | "create_file");

        // For local workspaces only: verify the file exists before attempting to snapshot
        if !is_remote_path(file_path.to_string_lossy().as_ref()).await && !file_path.exists() && !is_create_tool {
            error!(
                "File not found: file_path={} raw_path={} snapshot_workspace={}",
                file_path.display(),
                raw_path.display(),
                snapshot_workspace.display()
            );

            return Err(crate::util::errors::NortHingError::Tool(format!(
                "File not found: {} (Snapshot workspace: {})",
                file_path.display(),
                snapshot_workspace.display()
            )));
        }

        if is_create_tool && !file_path.exists() {
            debug!("Creating new file: file_path={}", file_path.display());
        }

        let file_existed_before = file_path.exists();
        let operation_type = self.get_operation_type_internal(file_existed_before);
        let turn_index = self.extract_turn_index(context);

        let snapshot_service = snapshot_manager.snapshot_service();
        let snapshot_service = snapshot_service.read().await;
        let intercept_started_at = std::time::Instant::now();
        let operation_id = snapshot_service
            .intercept_file_modification(
                &session_id,
                turn_index,
                self.name(),
                input.clone(),
                &file_path,
                operation_type,
                context.tool_call_id.clone(),
            )
            .await
            .map_err(|e| crate::util::errors::NortHingError::Tool(e.to_string()))?;
        let intercept_ms = crate::util::elapsed_ms_u64(intercept_started_at);

        debug!("Recorded file modification operation: operation_id={}", operation_id);

        let start_time = std::time::Instant::now();
        let results = self.original_tool.call(input, context).await?;
        let tool_call_ms = crate::util::elapsed_ms_u64(start_time);

        let complete_started_at = std::time::Instant::now();
        snapshot_service
            .complete_file_modification(&session_id, &operation_id, tool_call_ms)
            .await
            .map_err(|e| crate::util::errors::NortHingError::Tool(e.to_string()))?;
        let complete_ms = crate::util::elapsed_ms_u64(complete_started_at);
        let total_ms = intercept_ms.saturating_add(tool_call_ms).saturating_add(complete_ms);

        debug!(
            "File modification tool completed: tool_name={}, operation_id={}, total_ms={}, intercept_ms={}, tool_call_ms={}, complete_ms={}, file_path={}",
            self.name(),
            operation_id,
            total_ms,
            intercept_ms,
            tool_call_ms,
            complete_ms,
            file_path.display()
        );
        Ok(results)
    }

    /// Extracts the turn index.
    fn extract_turn_index(&self, context: &ToolUseContext) -> usize {
        context
            .custom_data
            .get("turn_index")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0)
    }

    /// Simplified file path extraction.
    fn extract_file_path_simple(&self, input: &Value) -> crate::service::snapshot::types::SnapshotResult<PathBuf> {
        let possible_fields = ["file_path", "path", "target_file", "filename"];

        for field in &possible_fields {
            if let Some(path_value) = input.get(field) {
                if let Some(path_str) = path_value.as_str() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        Err(crate::service::snapshot::types::SnapshotError::ConfigError(
            "Failed to extract file path from tool input".to_string(),
        ))
    }

    /// Returns the operation type.
    fn get_operation_type_internal(&self, file_existed_before: bool) -> crate::service::snapshot::types::OperationType {
        match self.name() {
            "Write" | "write_file" => {
                if file_existed_before {
                    crate::service::snapshot::types::OperationType::Modify
                } else {
                    crate::service::snapshot::types::OperationType::Create
                }
            }
            "create_file" => crate::service::snapshot::types::OperationType::Create,
            "delete_file" | "Delete" => crate::service::snapshot::types::OperationType::Delete,
            "rename_file" | "move_file" => crate::service::snapshot::types::OperationType::Rename,
            _ => crate::service::snapshot::types::OperationType::Modify,
        }
    }
}
