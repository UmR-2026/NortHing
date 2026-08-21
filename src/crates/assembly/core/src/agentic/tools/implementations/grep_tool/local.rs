//! `Grep` tool — local ripgrep execution path.
//!
//! Owns the `call_local` dispatcher invoked from the `Tool` impl when neither
//! the remote backend nor the indexed workspace-search service can serve the
//! request. Bridges the synchronous `grep_search` call into a blocking task
//! and forwards progress updates through the global event system so the
//! assistant UI can stream search progress.

use serde_json::{json, Value};
use tool_runtime::search::grep_search::{grep_search, GrepSearchResult};

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

        let search_result = tokio::task::spawn_blocking(move || grep_search(grep_options, None, Some(500))).await;

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
