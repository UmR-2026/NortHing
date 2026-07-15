//! `Grep` tool — workspace-search integration (request builder + output rendering).
//!
//! Translates the raw `serde_json::Value` input into a `ContentSearchRequest`
//! consumed by the indexed workspace-search service, and renders the resulting
//! `ContentSearchResult` into the user-visible grep output shape. The
//! standalone renderer helpers stay here as `pub(super)` free functions so
//! the test sibling can reach them through the parent module.

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;
use tool_runtime::search::grep_search::{apply_offset_and_limit, relativize_result_text};

use crate::agentic::tools::framework::ToolUseContext;
use crate::service::search::{ContentSearchOutputMode, ContentSearchRequest, WorkspaceSearchHit, WorkspaceSearchLine};
use crate::util::errors::{NortHingError, NortHingResult};

impl super::tool::GrepTool {
    pub(super) fn build_workspace_search_request(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<(ContentSearchRequest, String, bool, usize, Option<usize>)> {
        let workspace_root = context
            .workspace
            .as_ref()
            .map(|workspace| PathBuf::from(workspace.root_path_string()))
            .ok_or_else(|| NortHingError::tool("Workspace is required for Grep".to_string()))?;

        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("pattern is required".to_string()))?;
        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let resolved_path = context.resolve_workspace_tool_path(search_path)?;
        let resolved_path_buf = PathBuf::from(&resolved_path);
        let output_mode = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches")
            .to_string();
        let show_line_numbers = input
            .get("-n")
            .and_then(|v| v.as_bool())
            .unwrap_or(output_mode == "content");
        let offset = Self::resolve_offset(input);
        let head_limit = Self::resolve_head_limit(input);
        let max_results = Self::backend_max_results(input, offset, head_limit);
        let before_context = input.get("-B").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let after_context = input.get("-A").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let shared_context = input
            .get("context")
            .or_else(|| input.get("-C"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let globs = Self::parse_glob_patterns(input.get("glob").and_then(|v| v.as_str()));
        let file_types = input
            .get("type")
            .and_then(|v| v.as_str())
            .map(|value| vec![value.to_string()])
            .unwrap_or_default();
        let output_mode_enum = match output_mode.as_str() {
            "content" => ContentSearchOutputMode::Content,
            "count" => ContentSearchOutputMode::Count,
            _ => ContentSearchOutputMode::FilesWithMatches,
        };
        let request = ContentSearchRequest {
            repo_root: workspace_root.clone(),
            search_path: (resolved_path_buf != workspace_root).then_some(resolved_path_buf),
            pattern: pattern.to_string(),
            output_mode: output_mode_enum,
            case_sensitive: !input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false),
            use_regex: true,
            whole_word: false,
            multiline: input.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false),
            before_context: if shared_context > 0 {
                shared_context
            } else {
                before_context
            },
            after_context: if shared_context > 0 {
                shared_context
            } else {
                after_context
            },
            max_results,
            globs,
            file_types,
            exclude_file_types: Vec::new(),
        };

        Ok((request, output_mode, show_line_numbers, offset, head_limit))
    }

    pub(super) fn format_workspace_search_output(
        &self,
        output_mode: &str,
        show_line_numbers: bool,
        offset: usize,
        head_limit: Option<usize>,
        result: &crate::service::search::ContentSearchResult,
        display_base: Option<&str>,
    ) -> (String, usize, usize) {
        match output_mode {
            "content" => {
                let mut lines = render_workspace_search_content_lines(&result.hits, show_line_numbers);
                if lines.is_empty() {
                    lines = render_workspace_search_result_lines(&result.outcome.results, show_line_numbers);
                }
                apply_offset_and_limit(&mut lines, offset, head_limit);
                let rendered = relativize_result_text(&lines.join("\n"), display_base);
                let file_count = if result.hits.is_empty() {
                    result
                        .outcome
                        .results
                        .iter()
                        .map(|item| item.path.as_str())
                        .collect::<HashSet<_>>()
                        .len()
                } else {
                    result
                        .hits
                        .iter()
                        .map(|hit| hit.path.as_str())
                        .collect::<HashSet<_>>()
                        .len()
                };
                (rendered, file_count, result.matched_occurrences)
            }
            "count" => {
                let mut lines = result
                    .file_counts
                    .iter()
                    .map(|count| format!("{}:{}", count.path, count.matched_lines))
                    .collect::<Vec<_>>();
                lines.sort();
                let mut lines = lines.into_iter().collect::<Vec<_>>();
                apply_offset_and_limit(&mut lines, offset, head_limit);
                let rendered = relativize_result_text(&lines.join("\n"), display_base);
                (rendered, result.file_counts.len(), result.matched_lines)
            }
            _ => {
                let mut files = result
                    .outcome
                    .results
                    .iter()
                    .map(|item| item.path.clone())
                    .collect::<Vec<_>>();
                files.sort();
                files.dedup();
                apply_offset_and_limit(&mut files, offset, head_limit);
                let rendered = relativize_result_text(&files.join("\n"), display_base);
                let total_matches = files.len();
                (rendered, total_matches, total_matches)
            }
        }
    }
}

pub(super) fn render_workspace_search_result_lines(
    results: &[crate::infrastructure::FileSearchResult],
    show_line_numbers: bool,
) -> Vec<String> {
    results
        .iter()
        .filter_map(|result| {
            let content = result.matched_content.as_deref()?.trim_end();
            if show_line_numbers {
                result
                    .line_number
                    .map(|line| format!("{}:{}:{}", result.path, line, content))
                    .or_else(|| Some(format!("{}:{}", result.path, content)))
            } else {
                Some(format!("{}:{}", result.path, content))
            }
        })
        .collect()
}

pub(super) fn render_workspace_search_content_lines(
    hits: &[WorkspaceSearchHit],
    show_line_numbers: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    for hit in hits {
        for line in &hit.lines {
            match line {
                WorkspaceSearchLine::Match { value } => {
                    let snippet = value.snippet.trim_end();
                    if show_line_numbers {
                        lines.push(format!("{}:{}:{}", hit.path, value.location.line, snippet));
                    } else {
                        lines.push(format!("{}:{}", hit.path, snippet));
                    }
                }
                WorkspaceSearchLine::Context { value } => {
                    let snippet = value.snippet.trim_end();
                    if show_line_numbers {
                        lines.push(format!("{}-{}:{}", hit.path, value.line_number, snippet));
                    } else {
                        lines.push(format!("{}-{}", hit.path, snippet));
                    }
                }
                WorkspaceSearchLine::ContextBreak => lines.push("--".to_string()),
            }
        }
    }
    lines
}
