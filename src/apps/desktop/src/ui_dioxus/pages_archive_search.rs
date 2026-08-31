// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W12-2 (2026-08-31) — Archive session full-text search, export, and rename pure helpers.

use northhing_kernel_api::session::{MessageDto, SessionSearchHitDto};

/// Maximum snippet display length in Unicode characters before truncation.
pub const MAX_SNIPPET_CHARS: usize = 120;

/// Maximum allowed session-title length, counted in Unicode scalar values (chars), not UTF-8 bytes.
/// CJK ideographs are 3 UTF-8 bytes each; counting bytes would silently truncate ~26-char titles.
pub const MAX_SESSION_NAME_CHARS: usize = 80;

/// Reason a candidate session title was rejected.
#[derive(Debug, PartialEq, Eq)]
pub enum RenameError {
    /// `name.trim().is_empty()` — nothing to save (or only whitespace).
    Empty,
    /// `name.chars().count() > MAX_SESSION_NAME_CHARS`.
    TooLong,
}

/// Pure validator for the rename input box. Returns `Ok(())` when the trimmed title is
/// non-empty AND at most `MAX_SESSION_NAME_CHARS` Unicode chars; otherwise the specific reason.
pub fn validate_rename(name: &str) -> Result<(), RenameError> {
    if name.trim().is_empty() {
        return Err(RenameError::Empty);
    }
    if name.chars().count() > MAX_SESSION_NAME_CHARS {
        return Err(RenameError::TooLong);
    }
    Ok(())
}

/// Formats a session timestamp into `%Y-%m-%d %H:%M` or fallback string.
pub fn fmt_ts(ts: i64) -> String {
    let secs = ts / 1000;
    let nanos = ((ts % 1000) * 1_000_000) as u32;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        format!("{}", ts)
    }
}

/// Formats a session's messages as a Markdown document.
///
/// # Panics
/// Never panics – falls back to placeholder strings on malformed content.
pub fn format_session_export(session_id: &str, messages: &[MessageDto]) -> String {
    let header = format!(
        "# Session Export\n\n**Session ID:** `{session_id}`\n**Exported:** {}\n**Messages:** {}\n\n---\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        messages.len()
    );

    let body = messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                northhing_kernel_api::session::MessageRoleDto::User => "User",
                northhing_kernel_api::session::MessageRoleDto::Assistant => "Assistant",
                northhing_kernel_api::session::MessageRoleDto::Tool => "Tool",
                northhing_kernel_api::session::MessageRoleDto::System => "System",
            };
            let ts = fmt_ts(msg.timestamp);

            let content = match &msg.content {
                northhing_kernel_api::session::MessageContentDto::Text(t) => t.clone(),
                northhing_kernel_api::session::MessageContentDto::Multimodal { text, .. } => text.clone(),
                northhing_kernel_api::session::MessageContentDto::ToolResult {
                    tool_name,
                    result,
                    is_error,
                    ..
                } => {
                    let err_tag = if *is_error { " [ERROR]" } else { "" };
                    let summary = result.as_str().unwrap_or_default();
                    format!("[Tool: {tool_name}{err_tag}] {summary}")
                }
                northhing_kernel_api::session::MessageContentDto::Mixed { text, tool_calls, .. } => {
                    let tc = if tool_calls.is_empty() {
                        String::new()
                    } else {
                        let names: Vec<&str> = tool_calls.iter().map(|t| t.tool_name.as_str()).collect();
                        format!("\n**Tool calls:** {}", names.join(", "))
                    };
                    format!("{text}{tc}")
                }
            };

            format!("### [{ts}] {role}\n\n{content}\n")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{header}{body}")
}

/// Sorts search hits: title matches first, then timestamp_ms descending.
pub fn sort_search_hits(query: &str, mut hits: Vec<SessionSearchHitDto>) -> Vec<SessionSearchHitDto> {
    let q_lower = query.trim().to_lowercase();
    hits.sort_by(|a, b| {
        let a_title = if q_lower.is_empty() {
            false
        } else {
            a.session_name.to_lowercase().contains(&q_lower)
        };
        let b_title = if q_lower.is_empty() {
            false
        } else {
            b.session_name.to_lowercase().contains(&q_lower)
        };

        match (a_title, b_title) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.timestamp_ms.cmp(&a.timestamp_ms),
        }
    });
    hits
}

/// Truncates a snippet to at most `max_chars` Unicode scalar values (chars),
/// appending `...` when truncated.
pub fn truncate_snippet(snippet: &str, max_chars: usize) -> String {
    let trimmed = snippet.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}

/// Maps role strings from backend search hits to localized UI display labels.
pub fn search_hit_role_label(role: &str) -> &'static str {
    match role.to_lowercase().as_str() {
        "user" => "用户",
        "assistant" => "助手",
        "tool" => "工具",
        "system" => "系统",
        _ => "消息",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hit(id: &str, name: &str, snippet: &str, ts: i64) -> SessionSearchHitDto {
        SessionSearchHitDto {
            session_id: id.to_string(),
            session_name: name.to_string(),
            message_id: format!("msg-{id}"),
            role: "user".to_string(),
            snippet: snippet.to_string(),
            timestamp_ms: ts,
        }
    }

    #[test]
    fn test_sort_search_hits_title_match_prioritized_over_timestamp() {
        let h1 = make_hit("s1", "Session Alpha", "some text matching query", 1000);
        let h2 = make_hit("s2", "Search Target", "body text", 500);

        let hits = vec![h1, h2];
        let sorted = sort_search_hits("target", hits);

        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].session_id, "s2", "Title match must rank first");
        assert_eq!(sorted[1].session_id, "s1");
    }

    #[test]
    fn test_sort_search_hits_timestamp_desc_within_same_category() {
        let h1 = make_hit("s1", "Alpha Target", "body 1", 100);
        let h2 = make_hit("s2", "Beta Target", "body 2", 300);
        let h3 = make_hit("s3", "Other Session", "contains target in body", 50);
        let h4 = make_hit("s4", "Another Session", "also target in body", 200);

        let hits = vec![h1, h2, h3, h4];
        let sorted = sort_search_hits("target", hits);

        let ids: Vec<&str> = sorted.iter().map(|h| h.session_id.as_str()).collect();
        assert_eq!(ids, vec!["s2", "s1", "s4", "s3"]);
    }

    #[test]
    fn test_sort_search_hits_empty_query_falls_back_to_timestamp_desc() {
        let h1 = make_hit("s1", "Session 1", "text", 100);
        let h2 = make_hit("s2", "Session 2", "text", 500);
        let h3 = make_hit("s3", "Session 3", "text", 300);

        let hits = vec![h1, h2, h3];
        let sorted = sort_search_hits("", hits);

        let ids: Vec<&str> = sorted.iter().map(|h| h.session_id.as_str()).collect();
        assert_eq!(ids, vec!["s2", "s3", "s1"]);
    }

    #[test]
    fn test_truncate_snippet_preserves_short() {
        assert_eq!(truncate_snippet("hello world", 20), "hello world");
        assert_eq!(truncate_snippet("  trimmed text  ", 20), "trimmed text");
    }

    #[test]
    fn test_truncate_snippet_cjk_counts_chars_not_bytes() {
        let cjk_str = "这是一段测试文本用来验证中文字符截断逻辑是否正确";
        let truncated = truncate_snippet(cjk_str, 10);
        assert_eq!(truncated, "这是一段测试文本用来...");
        assert_eq!(truncated.chars().count(), 13);
    }

    #[test]
    fn test_truncate_snippet_exact_boundary() {
        let s = "12345";
        assert_eq!(truncate_snippet(s, 5), "12345");
        assert_eq!(truncate_snippet(s, 4), "1234...");
    }

    #[test]
    fn test_search_hit_role_label_mapping() {
        assert_eq!(search_hit_role_label("user"), "用户");
        assert_eq!(search_hit_role_label("User"), "用户");
        assert_eq!(search_hit_role_label("assistant"), "助手");
        assert_eq!(search_hit_role_label("Assistant"), "助手");
        assert_eq!(search_hit_role_label("tool"), "工具");
        assert_eq!(search_hit_role_label("system"), "系统");
        assert_eq!(search_hit_role_label("unknown"), "消息");
    }

    #[test]
    fn format_session_export_empty_messages() {
        let out = format_session_export("s1", &[]);
        assert!(out.contains("Session ID"));
        assert!(out.contains("Messages:** 0"));
    }

    #[test]
    fn format_session_export_includes_content() {
        let msgs = vec![MessageDto {
            id: "m1".into(),
            role: northhing_kernel_api::session::MessageRoleDto::User,
            content: northhing_kernel_api::session::MessageContentDto::Text("hello".into()),
            metadata: None,
            timestamp: 1_700_000_000_000,
        }];
        let out = format_session_export("s1", &msgs);
        assert!(out.contains("hello"));
        assert!(out.contains("User"));
    }

    #[test]
    fn validate_rename_accepts_ascii_under_limit() {
        assert!(validate_rename("hello world").is_ok());
        let s = "a".repeat(MAX_SESSION_NAME_CHARS);
        assert!(validate_rename(&s).is_ok());
    }

    #[test]
    fn validate_rename_rejects_empty_and_whitespace() {
        assert_eq!(validate_rename(""), Err(RenameError::Empty));
        assert_eq!(validate_rename("   "), Err(RenameError::Empty));
        assert_eq!(validate_rename("\t\n"), Err(RenameError::Empty));
    }

    #[test]
    fn validate_rename_rejects_too_long_ascii() {
        let s = "a".repeat(MAX_SESSION_NAME_CHARS + 1);
        assert_eq!(validate_rename(&s), Err(RenameError::TooLong));
    }

    #[test]
    fn validate_rename_accepts_cjk_at_char_limit() {
        let s: String = "测".repeat(MAX_SESSION_NAME_CHARS);
        assert_eq!(s.len(), MAX_SESSION_NAME_CHARS * 3, "sanity: 80 CJK chars = 240 bytes");
        assert!(
            validate_rename(&s).is_ok(),
            "80 CJK chars must pass; only bytes tripped the old check"
        );
    }

    #[test]
    fn validate_rename_rejects_cjk_over_char_limit() {
        let s: String = "测".repeat(MAX_SESSION_NAME_CHARS + 1);
        assert_eq!(validate_rename(&s), Err(RenameError::TooLong));
    }
}
