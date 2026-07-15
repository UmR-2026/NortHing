//! `Grep` tool — `GrepOptions` builder for the local ripgrep execution path.
//!
//! Translates the raw `serde_json::Value` input into a fully-populated
//! `GrepOptions` instance, including display-base resolution and context
//! line configuration.

use std::str::FromStr;

use serde_json::Value;
use tool_runtime::search::grep_search::{GrepOptions, OutputMode};

use crate::agentic::tools::framework::ToolUseContext;
use crate::util::errors::{NortHingError, NortHingResult};

impl super::tool::GrepTool {
    pub(super) fn build_grep_options(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<GrepOptions> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("pattern is required".to_string()))?;

        let search_path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let resolved = context.resolve_tool_path(search_path)?;
        let resolved_path = resolved.resolved_path.clone();

        let case_insensitive = input.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let multiline = input.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_mode_str = input
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");
        let output_mode = OutputMode::from_str(output_mode_str).map_err(|e| NortHingError::tool(e.to_string()))?;
        let show_line_numbers = input
            .get("-n")
            .and_then(|v| v.as_bool())
            .unwrap_or(output_mode_str == "content");
        let context_c = input
            .get("context")
            .or_else(|| input.get("-C"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let before_context = input.get("-B").and_then(|v| v.as_u64()).map(|v| v as usize);
        let after_context = input.get("-A").and_then(|v| v.as_u64()).map(|v| v as usize);
        let head_limit = Self::resolve_head_limit(input);
        let offset = Self::resolve_offset(input);
        let glob_patterns = Self::parse_glob_patterns(input.get("glob").and_then(|v| v.as_str()));
        let file_type = input.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());

        let mut options = GrepOptions::new(pattern, resolved_path)
            .case_insensitive(case_insensitive)
            .multiline(multiline)
            .output_mode(output_mode)
            .show_line_numbers(show_line_numbers);

        if resolved.is_runtime_artifact() {
            if let Some(runtime_root) = &resolved.runtime_root {
                options = options.display_base(runtime_root.to_string_lossy().to_string());
            }
        } else if let Some(display_base) = Self::display_base(context) {
            options = options.display_base(display_base);
        }

        if let Some(c) = context_c {
            options = options.context(c);
        }
        if let Some(b) = before_context {
            options = options.before_context(b);
        }
        if let Some(a) = after_context {
            options = options.after_context(a);
        }
        if let Some(h) = head_limit {
            options = options.head_limit(h);
        }
        if offset > 0 {
            options = options.offset(offset);
        }
        if !glob_patterns.is_empty() {
            options = options.globs(glob_patterns);
        }
        if let Some(t) = file_type {
            options = options.file_type(t);
        }

        Ok(options)
    }
}
