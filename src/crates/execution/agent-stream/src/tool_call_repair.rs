//! JSON truncation repair for partial tool-call arguments.
//!
//! When the model hits `max_tokens` mid-stream we sometimes see partial JSON
//! (unterminated strings, unclosed objects/arrays). For tools in the
//! safe-recovery list (see `tool_call_types::is_truncation_safe_to_recover`)
//! we can close those brackets and let the call execute; for everything else
//! the partial JSON is surfaced as a parse error so the model can be told to
//! retry.
//!
//! `repair_truncated_json` is the only entry point; it walks the bytes once,
//! tracking string/escape state and an explicit nesting stack of `{` / `[`,
//! then closes them in the reverse order they opened. It refuses to fabricate
//! missing values when the truncation occurs mid-pair (e.g. trailing `,` or
//! `:`) since blindly closing in those states would silently corrupt
//! semantics.

use tracing::warn;

/// Attempt to repair a JSON document that was truncated mid-stream (typically
/// because the model hit `max_tokens`). Closes any open string literal and any
/// unclosed `{`/`[` brackets in their correct nesting order. Returns `None`
/// when the truncation occurs at a position where we would have to invent a
/// missing value (e.g. trailing `,` or `:`) since blindly closing in those
/// states would silently corrupt the semantics.
pub(crate) fn repair_truncated_json(raw: &str) -> Option<String> {
    let mut in_string = false;
    let mut escape = false;
    let mut stack: Vec<u8> = Vec::new();
    let mut last_significant: Option<u8> = None;

    for &b in raw.as_bytes() {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            match b {
                b'\\' => escape = true,
                b'"' => {
                    in_string = false;
                    last_significant = Some(b'"');
                }
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => {
                in_string = true;
                last_significant = Some(b'"');
            }
            b'{' => {
                stack.push(b'{');
                last_significant = Some(b'{');
            }
            b'[' => {
                stack.push(b'[');
                last_significant = Some(b'[');
            }
            b'}' => {
                if stack.pop() != Some(b'{') {
                    return None;
                }
                last_significant = Some(b'}');
            }
            b']' => {
                if stack.pop() != Some(b'[') {
                    return None;
                }
                last_significant = Some(b']');
            }
            b' ' | b'\t' | b'\n' | b'\r' => {}
            other => last_significant = Some(other),
        }
    }

    // Nothing to repair (parser failed for some other reason).
    if !in_string && stack.is_empty() {
        return None;
    }

    // Refuse to fabricate values when truncated mid-pair.
    if !in_string {
        if let Some(b',') | Some(b':') = last_significant {
            return None;
        }
    }

    let mut out = String::with_capacity(raw.len() + stack.len() + 1);
    out.push_str(raw);
    if in_string {
        out.push('"');
    }
    while let Some(c) = stack.pop() {
        out.push(match c {
            b'{' => '}',
            b'[' => ']',
            _ => {
                warn!(
                    "repair_truncated_json: unexpected stack character {}, returning None",
                    c
                );
                return None;
            }
        });
    }
    Some(out)
}
