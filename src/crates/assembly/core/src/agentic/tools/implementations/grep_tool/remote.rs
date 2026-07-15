//! `Grep` tool — remote workspace shell-based grep fallback.
//!
//! Used when the search target resolves to a remote workspace backend and the
//! higher-level workspace-search path is unavailable. Builds a remote grep
//! command via `tool_runtime::search::grep_search` helpers and renders the
//! captured stdout into the same JSON shape the local path emits.

use std::str::FromStr;

use serde_json::{json, Value};
use tool_runtime::search::grep_search::{
    build_remote_grep_command, count_remote_grep_matches, render_remote_grep_result_text, OutputMode,
    RemoteGrepCommandRequest,
};

use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::util::errors::{NortHingError, NortHingResult};

impl super::tool::GrepTool {
    pub(super) async fn call_remote(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let ws_shell = context
            .ws_shell()
            .ok_or_else(|| NortHingError::tool("Workspace shell not available".to_string()))?;

        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("pattern is required".to_string()))?;

        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let resolved = context.resolve_tool_path(search_path)?;
        let resolved_path = resolved.resolved_path.clone();

        let case_insensitive = input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let head_limit = Self::resolve_head_limit(input);
        let offset = Self::resolve_offset(input);
        let output_mode = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");
        let output_mode_enum = OutputMode::from_str(output_mode).map_err(|e| NortHingError::tool(e.to_string()))?;
        let show_line_numbers = input
            .get("-n")
            .and_then(|v| v.as_bool())
            .unwrap_or(output_mode == "content");
        let context_c = input
            .get("context")
            .or_else(|| input.get("-C"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let before_context = input.get("-B").and_then(|v| v.as_u64()).map(|v| v as usize);
        let after_context = input.get("-A").and_then(|v| v.as_u64()).map(|v| v as usize);
        let glob_patterns = Self::parse_glob_patterns(input.get("glob").and_then(|v| v.as_str()));
        let file_type = input
            .get("type")
            .and_then(|v| v.as_str())
            .map(|value| value.to_string());

        let full_cmd = build_remote_grep_command(&RemoteGrepCommandRequest {
            pattern: pattern.to_string(),
            path: resolved_path,
            case_insensitive,
            output_mode: output_mode_enum,
            show_line_numbers,
            context: context_c,
            before_context,
            after_context,
            glob_patterns,
            file_type,
            head_limit,
            offset,
        });

        let (stdout, _stderr, _exit_code) = ws_shell
            .exec(&full_cmd, Some(30_000))
            .await
            .map_err(|e| NortHingError::tool(format!("Remote grep failed: {}", e)))?;

        let total_matches = count_remote_grep_matches(&stdout);
        let display_base = Self::display_base(context);
        let result_text = render_remote_grep_result_text(&stdout, pattern, display_base.as_deref());

        Ok(vec![ToolResult::Result {
            data: json!({
                "pattern": pattern,
                "path": resolved.logical_path,
                "output_mode": output_mode,
                "total_matches": total_matches,
                "applied_limit": head_limit,
                "applied_offset": if offset > 0 { Some(offset) } else { None::<usize> },
                "result": result_text,
            }),
            result_for_assistant: Some(result_text),
            image_attachments: None,
        }])
    }
}
