//! `Grep` tool — local ripgrep execution path.
//!
//! Owns the `call_local` dispatcher invoked from the `Tool` impl when neither
//! the remote backend nor the indexed workspace-search service can serve the
//! request. Bridges the synchronous `grep_search` call into a blocking task
//! and forwards progress updates through the global event system so the
//! assistant UI can stream search progress.

use std::sync::Arc;

use serde_json::{json, Value};
use tool_runtime::search::grep_search::{grep_search, GrepSearchResult, ProgressCallback};

use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};

impl super::tool::GrepTool {
    pub(super) async fn call_local(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let resolved = context.resolve_tool_path(search_path)?;

        let grep_options = self.build_grep_options(input, context)?;
        let pattern = grep_options.pattern.clone();
        let path = resolved.logical_path.clone();
        let output_mode = grep_options.output_mode.to_string();

        let event_system = crate::infrastructure::events::event_system::global_event_system();
        let tool_use_id = context
            .tool_call_id
            .clone()
            .unwrap_or_else(|| format!("grep_{}", uuid::Uuid::new_v4()));
        let tool_name = self.name().to_string();

        let tool_use_id_clone = tool_use_id.clone();
        let tool_name_clone = tool_name.clone();
        let event_system_clone = event_system.clone();
        let progress_callback: ProgressCallback = Arc::new(move |files_processed, file_count, total_matches| {
            let progress_message = format!(
                "Scanned {} files | Found {} matching files ({} matches)",
                files_processed, file_count, total_matches
            );

            let event = crate::infrastructure::events::event_system::BackendEvent::ToolExecutionProgress(
                crate::util::types::event::ToolExecutionProgressInfo {
                    tool_use_id: tool_use_id_clone.clone(),
                    tool_name: tool_name_clone.clone(),
                    progress_message,
                    percentage: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            );

            let event_system = event_system_clone.clone();
            tokio::spawn(async move {
                let _ = event_system.emit(event).await;
            });
        });

        let search_result =
            tokio::task::spawn_blocking(move || grep_search(grep_options, Some(progress_callback), Some(500))).await;

        let GrepSearchResult {
            file_count,
            total_matches,
            result_text,
            applied_limit,
            applied_offset,
        } = match search_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => return Err(NortHingError::tool(e)),
            Err(e) => return Err(NortHingError::tool(format!("grep search failed: {}", e))),
        };

        Ok(vec![ToolResult::Result {
            data: json!({
                "pattern": pattern,
                "path": path,
                "output_mode": output_mode,
                "file_count": file_count,
                "total_matches": total_matches,
                "applied_limit": applied_limit,
                "applied_offset": applied_offset,
                "result": result_text,
            }),
            result_for_assistant: Some(result_text),
            image_attachments: None,
        }])
    }
}
