use crate::agentic::tools::framework::{Tool, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult};
use crate::util::errors::NortHingResult;
use async_trait::async_trait;
use serde_json::Value;
use tool_runtime::fs::WriteLocalFileMode;

mod write_error;
mod write_execute;
mod write_format;
mod write_validate;

pub struct FileWriteTool;

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWriteTool {
    pub fn new() -> Self {
        Self
    }
}

// Re-export submodule functions for Tool trait delegation.
use self::write_execute::call_impl;
use self::write_validate::{description, input_schema, parse_mode_value, validate_input};

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(description())
    }

    fn short_description(&self) -> String {
        "Write or overwrite a file.".to_string()
    }

    async fn description_with_context(&self, _context: Option<&ToolUseContext>) -> NortHingResult<String> {
        Ok(description())
    }

    fn input_schema(&self) -> Value {
        input_schema()
    }

    async fn input_schema_for_model(&self) -> Value {
        input_schema()
    }

    async fn input_schema_for_model_with_context(&self, _context: Option<&ToolUseContext>) -> Value {
        input_schema()
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    async fn validate_input(&self, input: &Value, context: Option<&ToolUseContext>) -> ValidationResult {
        validate_input(input, context).await
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let mode = parse_mode_value(input.get("mode").and_then(|v| v.as_str())).unwrap_or(WriteLocalFileMode::Write);
        if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
            if options.verbose {
                let content_len = input
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.len())
                    .unwrap_or(0);
                match mode {
                    WriteLocalFileMode::Write => {
                        format!("Writing {} characters to {}", content_len, file_path)
                    }
                    WriteLocalFileMode::Append => {
                        format!("Appending {} characters to {}", content_len, file_path)
                    }
                }
            } else {
                match mode {
                    WriteLocalFileMode::Write => format!("Write {}", file_path),
                    WriteLocalFileMode::Append => format!("Append {}", file_path),
                }
            }
        } else {
            "Writing file".to_string()
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        call_impl(input, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::write_error::preflight_write_error;
    use super::FileWriteTool;
    use crate::agentic::tools::file_tool_guidance::{
        file_tool_guidance_message, is_file_tool_guidance_message, FILE_TOOL_GUIDANCE_PREFIX,
    };
    use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
    use crate::agentic::tools::ToolRuntimeRestrictions;
    use crate::agentic::WorkspaceBinding;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn local_context(root: PathBuf) -> ToolUseContext {
        ToolUseContext {
            tool_call_id: None,
            agent_type: None,
            session_id: None,
            dialog_turn_id: None,
            workspace: Some(WorkspaceBinding::new(None, root)),
            unlocked_collapsed_tools: Vec::new(),
            custom_data: HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: ToolRuntimeRestrictions::default(),
            runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
            actor_runtime: None,
        }
    }

    #[test]
    fn guidance_prefix_helpers_round_trip() {
        let message = file_tool_guidance_message("Use Read first.");
        assert!(is_file_tool_guidance_message(&message));
        assert_eq!(
            message.strip_prefix(FILE_TOOL_GUIDANCE_PREFIX).unwrap(),
            "Use Read first."
        );
    }

    #[tokio::test]
    async fn preflight_write_error_allows_new_file_target() {
        let root = std::env::temp_dir().join(format!("northhing-write-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp workspace");

        let error = preflight_write_error(&local_context(root.clone()), "new.txt").await;

        let _ = std::fs::remove_dir_all(&root);

        assert!(error.is_none());
    }

    #[tokio::test]
    async fn preflight_write_error_allows_existing_file_without_read_state_tracking() {
        let root = std::env::temp_dir().join(format!("northhing-write-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp workspace");
        std::fs::write(root.join("existing.md"), "already here").expect("create existing file");

        let error = preflight_write_error(&local_context(root.clone()), "existing.md").await;

        let _ = std::fs::remove_dir_all(&root);

        assert!(error.is_none());
    }

    #[tokio::test]
    async fn call_impl_treats_identical_existing_content_as_success() {
        let root = std::env::temp_dir().join(format!("northhing-write-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp workspace");
        std::fs::write(root.join("existing.md"), "same content").expect("create existing file");

        let tool = FileWriteTool::new();
        let results = tool
            .call(
                &json!({ "file_path": "existing.md", "content": "same content" }),
                &local_context(root.clone()),
            )
            .await
            .expect("identical retry should be idempotent");

        let _ = std::fs::remove_dir_all(&root);

        let ToolResult::Result {
            data,
            result_for_assistant,
            ..
        } = &results[0]
        else {
            panic!("expected result");
        };
        assert_eq!(data["success"], true);
        assert_eq!(data["bytes_written"], 0);
        assert_eq!(data["lines_written"], 0);
        assert_eq!(data["status"], "already_exists_same_content");
        assert!(result_for_assistant
            .as_deref()
            .unwrap_or_default()
            .contains("identical content"));
    }

    #[tokio::test]
    async fn call_impl_overwrites_different_existing_content() {
        let root = std::env::temp_dir().join(format!("northhing-write-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp workspace");
        std::fs::write(root.join("existing.md"), "old content").expect("create existing file");

        let tool = FileWriteTool::new();
        let results = tool
            .call(
                &json!({ "file_path": "existing.md", "content": "new content" }),
                &local_context(root.clone()),
            )
            .await
            .expect("write should overwrite existing files");

        let written = std::fs::read_to_string(root.join("existing.md")).expect("read file");
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(written, "new content");

        let ToolResult::Result { data, .. } = &results[0] else {
            panic!("expected result");
        };
        assert_eq!(data["status"], "overwritten");
        assert_eq!(data["bytes_written"], "new content".len());
        assert_eq!(data["lines_written"], 1);
    }

    #[tokio::test]
    async fn call_impl_appends_when_mode_is_append() {
        let root = std::env::temp_dir().join(format!("northhing-write-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp workspace");
        std::fs::write(root.join("existing.md"), "old").expect("create existing file");

        let tool = FileWriteTool::new();
        let results = tool
            .call(
                &json!({ "file_path": "existing.md", "content": "\nnew", "mode": "a" }),
                &local_context(root.clone()),
            )
            .await
            .expect("append should succeed");

        let written = std::fs::read_to_string(root.join("existing.md")).expect("read file");
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(written, "old\nnew");

        let ToolResult::Result { data, .. } = &results[0] else {
            panic!("expected result");
        };
        assert_eq!(data["status"], "appended");
        assert_eq!(data["mode"], "a");
        assert_eq!(data["bytes_written"], "\nnew".len());
    }

    #[tokio::test]
    async fn schema_requires_file_path_and_content() {
        let tool = FileWriteTool::new();

        let schema = tool.input_schema_for_model().await;

        assert_eq!(schema["required"], serde_json::json!(["file_path", "content"]));
        assert!(schema["properties"].get("content").is_some());
        assert_eq!(schema["properties"]["mode"]["enum"], serde_json::json!(["w", "a"]));
    }

    #[tokio::test]
    async fn validate_input_requires_content() {
        let tool = FileWriteTool::new();

        let validation = tool.validate_input(&json!({ "file_path": "new.txt" }), None).await;

        assert!(!validation.result);
        assert_eq!(validation.message.as_deref(), Some("content is required"));
    }

    #[tokio::test]
    async fn validate_input_rejects_invalid_mode() {
        let tool = FileWriteTool::new();

        let validation = tool
            .validate_input(
                &json!({ "file_path": "new.txt", "content": "hello", "mode": "x" }),
                None,
            )
            .await;

        assert!(!validation.result);
        assert_eq!(
            validation.message.as_deref(),
            Some("mode must be either 'w' (overwrite) or 'a' (append), got 'x'")
        );
    }
}
