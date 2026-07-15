//! `Grep` tool — input parameter parsing helpers.
//!
//! Owns the static methods used by every execution path to extract structured
//! fields from the raw `serde_json::Value` input. These helpers are pure and
//! have no IO side effects, which makes them ideal to share across the local,
//! remote, and workspace siblings.

use serde_json::Value;

use crate::agentic::tools::framework::ToolUseContext;

pub(super) const DEFAULT_HEAD_LIMIT: usize = 250;

impl super::tool::GrepTool {
    pub(super) fn explicit_head_limit(input: &Value) -> Option<Option<usize>> {
        input.get("head_limit").and_then(|v| v.as_u64()).map(
            |value| {
                if value == 0 {
                    None
                } else {
                    Some(value as usize)
                }
            },
        )
    }

    pub(super) fn resolve_head_limit(input: &Value) -> Option<usize> {
        Self::explicit_head_limit(input).unwrap_or(Some(DEFAULT_HEAD_LIMIT))
    }

    pub(super) fn backend_max_results(
        input: &Value,
        offset: usize,
        _display_head_limit: Option<usize>,
    ) -> Option<usize> {
        Self::explicit_head_limit(input)
            .flatten()
            .map(|limit| limit.saturating_add(offset))
    }

    pub(super) fn parse_glob_patterns(glob: Option<&str>) -> Vec<String> {
        let Some(glob) = glob else {
            return Vec::new();
        };

        let mut patterns = Vec::new();
        for raw_pattern in glob.split_whitespace() {
            if raw_pattern.contains('{') && raw_pattern.contains('}') {
                patterns.push(raw_pattern.to_string());
            } else {
                patterns.extend(
                    raw_pattern
                        .split(',')
                        .filter(|pattern| !pattern.is_empty())
                        .map(|pattern| pattern.to_string()),
                );
            }
        }
        patterns
    }

    pub(super) fn resolve_offset(input: &Value) -> usize {
        input
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|value| value as usize)
            .unwrap_or(0)
    }

    pub(super) fn display_base(context: &ToolUseContext) -> Option<String> {
        context.workspace.as_ref().map(|workspace| workspace.root_path_string())
    }
}
