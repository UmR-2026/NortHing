//! ControlHubTool envelope and dispatch-error helpers.
//!
//!
//! R18 split: extracted from `control_hub_tool_helpers.rs` (the 162-line
//! post-R17 file) into a new leaf module. These 4 helpers are
//! consumed only by the facade `call_impl` — they are NOT a
//! general-purpose utility bucket. `parse_browser_kind` stays in
//! `control_hub_tool_browser.rs` (browser-specific) and
//! `control_hub_tool_helpers.rs` is deleted.

use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::control_hub::{ControlHubError, ErrorCode};

// parse_bracket_code_prefix — used by map_dispatch_error + tests.

/// Parse a leading `"[CODE] rest"` prefix produced by the front-end
/// front-end error prefix so we can recover the structured `ErrorCode`
/// in the backend instead of falling back to the heuristic classifier.
/// Returns `(code, rest_without_prefix)` or `None` if the input is not in
/// that shape.
pub(super) fn parse_bracket_code_prefix(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if !s.starts_with('[') {
        return None;
    }
    let end = s.find(']')?;
    let code = s[1..end].trim();
    if code.is_empty() {
        return None;
    }
    // Make sure the bracketed token actually looks like a code
    // (UPPER_SNAKE_CASE) to avoid swallowing other bracketed prefixes.
    if !code
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    let rest = s[end + 1..].trim_start();
    Some((code, rest))
}

// parse_hints_suffix — used by map_dispatch_error + tests.

/// Split `"message\nHints: a | b"` into `(message, ["a", "b"])`. If there is
/// no `Hints:` block, returns `(input, [])`.
pub(super) fn parse_hints_suffix(input: &str) -> (String, Vec<String>) {
    if let Some(idx) = input.rfind("\nHints:") {
        let (msg, hints_block) = input.split_at(idx);
        let hints_str = hints_block.trim_start_matches("\nHints:").trim();
        let hints = hints_str
            .split('|')
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect();
        (msg.trim().to_string(), hints)
    } else {
        (input.trim().to_string(), Vec::new())
    }
}

// envelope_wrap_results — used by facade impl Tool call_impl().

/// Re-wrap each [`ToolResult`] returned by a legacy handler into the unified
/// `{ ok: true, domain, action, data }` envelope so the model gets a consistent
/// shape across every domain. Image attachments are preserved.
pub(super) fn envelope_wrap_results(domain: &str, action: &str, results: Vec<ToolResult>) -> Vec<ToolResult> {
    results
        .into_iter()
        .map(|r| match r {
            ToolResult::Result {
                data,
                result_for_assistant,
                image_attachments,
            } => {
                let summary = result_for_assistant.clone();
                let mut body = json!({
                    "ok": true,
                    "domain": domain,
                    "action": action,
                    "data": data,
                });
                if let Some(s) = result_for_assistant.as_ref() {
                    if let Some(obj) = body.as_object_mut() {
                        obj.insert("summary".to_string(), Value::String(s.clone()));
                    }
                }
                ToolResult::Result {
                    data: body,
                    result_for_assistant: summary,
                    image_attachments,
                }
            }
            other => other,
        })
        .collect()
}

// map_dispatch_error — used by facade call_impl + tests.

/// Best-effort classification of a legacy `NortHingError` into a structured
/// ControlHub error. Domain handlers should be migrated to return structured
/// envelopes directly; this is the safety net for the transition.
pub(super) fn map_dispatch_error(domain: &str, _action: &str, err: NortHingError) -> ControlHubError {
    let msg = err.to_string();

    // Frontend bridges may send back `[CODE] message\nHints: a | b` strings —
    // parse that prefix back into a structured ControlHubError so the model
    // sees the *actual* error code and hints instead of an INTERNAL fallback.
    // `NortHingError::Tool` wraps the message with `"Tool error: "`, so we try
    // both the raw form and the form after stripping that wrapper.
    let strip_candidate = msg
        .strip_prefix("Tool error: ")
        .or_else(|| msg.strip_prefix("Service error: "))
        .or_else(|| msg.strip_prefix("Agent error: "))
        .unwrap_or(msg.as_str());
    if let Some((code_str, rest)) =
        parse_bracket_code_prefix(strip_candidate).or_else(|| parse_bracket_code_prefix(&msg))
    {
        let (message, hints) = parse_hints_suffix(rest);
        let code = ErrorCode::from_str(code_str).unwrap_or(ErrorCode::FrontendError);
        let mut err = ControlHubError::new(code, message);
        for h in hints {
            err = err.with_hint(h);
        }
        return err;
    }

    let lower = msg.to_lowercase();
    let code = if lower.contains("not found") {
        ErrorCode::NotFound
    } else if lower.contains("ambiguous") {
        ErrorCode::Ambiguous
    } else if lower.contains("permission") || lower.contains("not allowed") {
        ErrorCode::PermissionDenied
    } else if lower.contains("timed out") || lower.contains("timeout") {
        ErrorCode::Timeout
    } else if lower.contains("stale") || lower.contains("take a fresh") {
        ErrorCode::StaleRef
    } else if lower.contains("refused") || lower.contains("guard") {
        ErrorCode::GuardRejected
    } else if lower.contains("only available in") || lower.contains("not available") {
        ErrorCode::NotAvailable
    } else if domain == "terminal" && lower.contains("session") {
        ErrorCode::MissingSession
    } else if domain == "browser"
        && (lower.contains("no longer connected")
            || lower.contains("tab was likely closed")
            || lower.contains("page was closed"))
    {
        ErrorCode::WrongTab
    } else {
        ErrorCode::Internal
    };
    ControlHubError::new(code, msg)
}
