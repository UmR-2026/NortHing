// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chat state helpers
//!
//! Free-standing helper functions used across the chat state siblings:
//! `extract_fallback_summary`, `extract_tool_title`, `truncate_string`.

/// Extract a human-readable summary from a tool result JSON Value.
/// Used as fallback when `display_summary` is not provided (e.g. MCP tools, old data).
pub(super) fn extract_fallback_summary(result: &serde_json::Value) -> String {
    if let Some(obj) = result.as_object() {
        // Try common text fields first
        for key in &[
            "display_summary",
            "result_for_assistant",
            "output",
            "result",
            "content",
            "message",
        ] {
            if let Some(text) = obj.get(*key).and_then(|v| v.as_str()) {
                if !text.is_empty() && text.len() < 200 {
                    return text.to_string();
                } else if !text.is_empty() {
                    let truncated: String = text.chars().take(200).collect();
                    return format!("{}...", truncated);
                }
            }
        }

        // Try success field
        if let Some(true) = obj.get("success").and_then(|v| v.as_bool()) {
            return "Done".to_string();
        }

        // Try extracting key parameter values
        let priority_keys = ["path", "file_path", "query", "pattern", "command", "url"];
        for key in &priority_keys {
            if let Some(s) = obj.get(*key).and_then(|v| v.as_str()) {
                if !s.is_empty() && s.len() < 100 {
                    return s.to_string();
                }
            }
        }
    }

    // If it's a plain string
    if let Some(text) = result.as_str() {
        if text.len() < 200 {
            return text.to_string();
        }
        let truncated: String = text.chars().take(200).collect();
        return format!("{}...", truncated);
    }

    "Done".to_string()
}

/// Extract a short title from tool parameters for subagent progress display.
/// Returns a concise description like the file path, command, or query.
pub(super) fn extract_tool_title(tool_name: &str, params: &serde_json::Value) -> Option<String> {
    let obj = params.as_object()?;

    // Tool-specific extraction for common tools
    match tool_name {
        "Read" | "Write" | "Edit" | "Delete" | "GetFileDiff" => {
            obj.get("path").and_then(|v| v.as_str()).map(|s| truncate_string(s, 50))
        }
        "Bash" => obj
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| truncate_string(s, 50)),
        "Grep" => obj
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|s| truncate_string(s, 40)),
        "Glob" | "LS" => obj
            .get("glob_pattern")
            .or_else(|| obj.get("target_directory"))
            .and_then(|v| v.as_str())
            .map(|s| truncate_string(s, 50)),
        "WebSearch" => obj
            .get("search_term")
            .and_then(|v| v.as_str())
            .map(|s| truncate_string(s, 40)),
        "WebFetch" => obj.get("url").and_then(|v| v.as_str()).map(|s| truncate_string(s, 50)),
        _ => {
            // Generic: try common parameter names
            for key in &["path", "file_path", "command", "query", "pattern", "url", "description"] {
                if let Some(s) = obj.get(*key).and_then(|v| v.as_str()) {
                    if !s.is_empty() {
                        return Some(truncate_string(s, 50));
                    }
                }
            }
            None
        }
    }
}

/// Truncate a string to a maximum number of characters, adding "..." if truncated.
pub(super) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}
