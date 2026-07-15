//! `GrepTool` definition and `Tool` trait implementation.
//!
//! The struct, `Default` impl, and `Tool` trait impl all live here.
//! `call_impl` is intentionally thin: it inspects the workspace context,
//! decides whether the request routes through the indexed workspace-search
//! service, the remote shell fallback, or the local ripgrep execution path,
//! and forwards to the matching sibling. All concrete logic lives in
//! `options.rs`, `remote.rs`, `workspace.rs`, and `local.rs`.

use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use crate::agentic::tools::framework::{Tool, ToolRenderOptions, ToolResult, ToolUseContext};
use crate::service::search::{
    global_workspace_search_service, remote_workspace_search_service_for_path, workspace_search_feature_enabled,
    workspace_search_runtime_available,
};
use crate::util::errors::NortHingResult;

pub struct GrepTool;

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(r#"A powerful search tool built on ripgrep

Usage:
- Use Grep by default for codebase content search because it preserves workspace-aware permissions and consistent output. Shell out to `grep` or `rg` only when this tool cannot meet the requirement, and prefer explaining why when doing so.
- For simple literal names or symbols, start with the literal text before trying broad regexes.
- Narrow searches with `path`, `glob`, or `type` when you know the likely area or language, and use `head_limit` to keep exploratory searches readable.
- A common workflow is `output_mode: "files_with_matches"` to locate candidate files, followed by `output_mode: "content"` with `-n` and small context when exact lines are needed.
- Supports full regex syntax (e.g., "log.*Error", "function\s+\w+")
- Filter files with glob parameter (e.g., "*.js", "**/*.tsx") or type parameter (e.g., "js", "py", "rust")
- The path parameter may be workspace-relative, an absolute path inside the current workspace, or an exact `northhing://runtime/...` URI returned by another tool
- Omit path to search the current workspace. Do not search host roots or placeholder paths such as `/workspace`.
- Output modes: "content" shows matching lines, "files_with_matches" shows only file paths (default), "count" shows match counts
- Use Task tool for open-ended searches requiring multiple rounds
- Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping (use `interface\{\}` to find `interface{}` in Go code)
- Multiline matching: By default patterns match within single lines only. For cross-line patterns like `struct \{[\s\S]*?field`, use `multiline: true`"#.to_string())
    }

    fn short_description(&self) -> String {
        "Search file contents with ripgrep-powered pattern matching.".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in. Omit to search the current workspace. If provided, use a workspace-relative path, an absolute path inside the current workspace, or an exact northhing://runtime URI."
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\") - maps to rg --glob"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: \"content\" shows matching lines (supports -A/-B/-C context, -n line numbers, head_limit), \"files_with_matches\" shows file paths (supports head_limit), \"count\" shows match counts (supports head_limit). Defaults to \"files_with_matches\"."
                },
                "-B": { "type": "number", "description": "Number of lines to show before each match (rg -B). Requires output_mode: \"content\", ignored otherwise." },
                "-A": { "type": "number", "description": "Number of lines to show after each match (rg -A). Requires output_mode: \"content\", ignored otherwise." },
                "-C": { "type": "number", "description": "Number of lines to show before and after each match (rg -C). Requires output_mode: \"content\", ignored otherwise." },
                "context": { "type": "number", "description": "Alias for -C. Number of lines to show before and after each match." },
                "-n": { "type": "boolean", "description": "Show line numbers in output (rg -n). Requires output_mode: \"content\", ignored otherwise." },
                "-i": { "type": "boolean", "description": "Case insensitive search (rg -i)" },
                "type": { "type": "string", "description": "File type to search (rg --type). Common types: js, py, rust, go, java, etc." },
                "head_limit": { "type": "number", "description": "Limit output to first N lines/entries." },
                "offset": { "type": "number", "description": "Skip the first N lines/entries before applying head_limit." },
                "multiline": { "type": "boolean", "description": "Enable multiline mode where . matches newlines and patterns can span lines (rg -U --multiline-dotall). Default: false." }
            },
            "required": ["pattern"],
            "additionalProperties": false,
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let file_type = input.get("type").and_then(|v| v.as_str());
        let glob_pattern = input.get("glob").and_then(|v| v.as_str());
        let output_mode = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");

        let scope = if search_path == "." {
            "Current workspace".to_string()
        } else {
            search_path.to_string()
        };
        let scope_with_filter = if let Some(ft) = file_type {
            format!("{} (*.{})", scope, ft)
        } else if let Some(gp) = glob_pattern {
            format!("{} ({})", scope, gp)
        } else {
            scope
        };
        let mode_desc = match output_mode {
            "content" => "Show matching content",
            "count" => "Count matches",
            _ => "List matching files",
        };

        format!("Search \"{}\" | {} | {}", pattern, scope_with_filter, mode_desc)
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        // Remote workspace: use shell-based grep/rg
        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let resolved = context.resolve_tool_path(search_path)?;

        if resolved.uses_remote_workspace_backend() {
            if workspace_search_feature_enabled().await {
                let remote_workspace_search_result = async {
                    let (request, output_mode, show_line_numbers, offset, head_limit) =
                        self.build_workspace_search_request(input, context)?;
                    let pattern = request.pattern.clone();
                    let path = request
                        .search_path
                        .as_ref()
                        .map(|path| path.to_string_lossy().to_string())
                        .unwrap_or_else(|| request.repo_root.to_string_lossy().to_string());
                    let repo_root = request.repo_root.to_string_lossy().to_string();
                    let preferred_connection_id = context
                        .workspace
                        .as_ref()
                        .and_then(|workspace| workspace.connection_id())
                        .map(str::to_string);
                    let search_service =
                        remote_workspace_search_service_for_path(&repo_root, preferred_connection_id)
                            .await
                            .map_err(crate::util::errors::NortHingError::tool)?;
                    let search_started_at = Instant::now();
                    let search_result = search_service
                        .search_content(request)
                        .await
                        .map_err(crate::util::errors::NortHingError::tool)?;
                    let display_base = Self::display_base(context);
                    let (result_text, file_count, total_matches) =
                        self.format_workspace_search_output(
                            &output_mode,
                            show_line_numbers,
                            offset,
                            head_limit,
                            &search_result,
                            display_base.as_deref(),
                        );
                    let workspace_search_elapsed_ms = search_started_at.elapsed().as_millis();

                    info!(
                        "Grep tool remote workspace-search result: pattern={}, path={}, output_mode={}, file_count={}, total_matches={}, backend={:?}, repo_phase={:?}, rebuild_recommended={}, dirty_modified={}, dirty_deleted={}, dirty_new={}, candidate_docs={}, matched_lines={}, matched_occurrences={}, workspace_search_ms={}",
                        pattern,
                        path,
                        output_mode,
                        file_count,
                        total_matches,
                        search_result.backend,
                        search_result.repo_status.phase,
                        search_result.repo_status.rebuild_recommended,
                        search_result.repo_status.dirty_files.modified,
                        search_result.repo_status.dirty_files.deleted,
                        search_result.repo_status.dirty_files.new,
                        search_result.candidate_docs,
                        search_result.matched_lines,
                        search_result.matched_occurrences,
                        workspace_search_elapsed_ms,
                    );

                    Ok::<Vec<ToolResult>, crate::util::errors::NortHingError>(vec![ToolResult::Result {
                        data: json!({
                            "pattern": pattern,
                            "path": path,
                            "output_mode": output_mode,
                            "file_count": file_count,
                            "total_matches": total_matches,
                            "backend": search_result.backend,
                            "repo_phase": search_result.repo_status.phase,
                            "rebuild_recommended": search_result.repo_status.rebuild_recommended,
                            "applied_limit": head_limit,
                            "applied_offset": if offset > 0 { Some(offset) } else { None::<usize> },
                            "result": result_text,
                        }),
                        result_for_assistant: Some(result_text),
                        image_attachments: None,
                    }])
                }
                .await;

                match remote_workspace_search_result {
                    Ok(results) => return Ok(results),
                    Err(error) => {
                        tracing::warn!(
                            "Grep tool remote workspace-search failed; falling back to shell grep: {}",
                            error
                        );
                    }
                }
            }
            return self.call_remote(input, context).await;
        }

        if workspace_search_runtime_available().await {
            if let Some(search_service) = global_workspace_search_service() {
                let (request, output_mode, show_line_numbers, offset, head_limit) =
                    self.build_workspace_search_request(input, context)?;
                let pattern = request.pattern.clone();
                let path = request
                    .search_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| request.repo_root.to_string_lossy().to_string());
                let search_started_at = Instant::now();
                let search_result = search_service.search_content(request).await?;
                let display_base = Self::display_base(context);
                let (result_text, file_count, total_matches) = self.format_workspace_search_output(
                    &output_mode,
                    show_line_numbers,
                    offset,
                    head_limit,
                    &search_result,
                    display_base.as_deref(),
                );
                let workspace_search_elapsed_ms = search_started_at.elapsed().as_millis();

                info!(
                    "Grep tool workspace-search result: pattern={}, path={}, output_mode={}, file_count={}, total_matches={}, backend={:?}, repo_phase={:?}, rebuild_recommended={}, dirty_modified={}, dirty_deleted={}, dirty_new={}, candidate_docs={}, matched_lines={}, matched_occurrences={}, workspace_search_ms={}",
                    pattern,
                    path,
                    output_mode,
                    file_count,
                    total_matches,
                    search_result.backend,
                    search_result.repo_status.phase,
                    search_result.repo_status.rebuild_recommended,
                    search_result.repo_status.dirty_files.modified,
                    search_result.repo_status.dirty_files.deleted,
                    search_result.repo_status.dirty_files.new,
                    search_result.candidate_docs,
                    search_result.matched_lines,
                    search_result.matched_occurrences,
                    workspace_search_elapsed_ms,
                );

                return Ok(vec![ToolResult::Result {
                    data: json!({
                        "pattern": pattern,
                        "path": path,
                        "output_mode": output_mode,
                        "file_count": file_count,
                        "total_matches": total_matches,
                        "backend": search_result.backend,
                        "repo_phase": search_result.repo_status.phase,
                        "rebuild_recommended": search_result.repo_status.rebuild_recommended,
                        "applied_limit": head_limit,
                        "applied_offset": if offset > 0 { Some(offset) } else { None::<usize> },
                        "result": result_text,
                    }),
                    result_for_assistant: Some(result_text),
                    image_attachments: None,
                }]);
            }
        }

        self.call_local(input, context).await
    }
}
