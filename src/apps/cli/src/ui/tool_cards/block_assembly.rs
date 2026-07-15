//! Block assembly — shared block-frame layout, parameter/status helpers used
//! by all per-tool block renderers (in `block_render`) and the HMOS-specific
//! compilation block (in `hmos_block`).
//!
//! Split (R38a) — extracted from `tool_cards.rs` so the facade can dispatch
//! to multiple siblings without each carrying its own copy of the helpers.
//! Cross-sibling visibility is via `pub(super)` (sibling-to-sibling only).

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use super::super::string_utils::{truncate_str, wrap_to_display_width};
use super::super::theme::{StyleKind, Theme};
use crate::chat_state::{ToolDisplayState, ToolDisplayStatus};

// ============ Shared width/wrap helpers ============

/// Maximum width of block content lines, accounting for the surrounding box border.
pub(super) fn block_content_max_width(available_width: u16) -> usize {
    available_width.saturating_sub(8).max(1) as usize
}

/// Wrap each line of text to the given display width using the string_utils helper.
pub(super) fn wrap_display_lines(text: &str, max_width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let sanitized = raw.replace('\t', "    ");
        out.extend(wrap_to_display_width(&sanitized, max_width));
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

// ============ Block Assembly ============

/// Assemble a block tool card with a full box frame using Unicode box-drawing characters.
/// The background color fills the entire box width uniformly.
///
/// Layout:
/// ```text
///   ╭──────────────────────────────────────────────╮
///   │  Title                        (1.2s)  ✓      │
///   │    content line 1                             │
///   │    content line 2                             │
///   │    error message (if any)                     │
///   ╰──────────────────────────────────────────────╯
/// ```
pub(super) fn assemble_block(
    title: &str,
    content_lines: Vec<Line<'static>>,
    theme: &Theme,
    is_running: bool,
    error: Option<&str>,
    focused: bool,
    tool_state: &ToolDisplayState,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let mut items = Vec::new();
    let mut plain_lines = Vec::new();

    let border_style = if focused {
        theme.style(StyleKind::BlockBorderActive)
    } else if is_running {
        theme.style(StyleKind::Primary)
    } else {
        theme.style(StyleKind::Border)
    };

    // Background style for the entire block
    let bg_style = if focused {
        theme.style(StyleKind::BlockBackgroundHover)
    } else {
        theme.style(StyleKind::BlockBackground)
    };

    let (status_icon, status_style) = status_icon_and_style(&tool_state.status, theme, spinner_frame);

    // Duration text
    let duration_text = tool_state
        .duration_ms
        .map(|ms| {
            if ms < 1000 {
                format!("{}ms", ms)
            } else {
                format!("{:.1}s", ms as f64 / 1000.0)
            }
        })
        .unwrap_or_default();

    // Box dimensions:
    // Layout: "  ╭─...─╮" => 2 (left margin) + 1 (corner) + inner_width (horizontal lines) + 1 (corner)
    // The inner content area width = available_width - 2 (margin) - 2 (left+right border) = available_width - 4
    let total_w = available_width as usize;
    // Minimum box width
    let box_w = if total_w > 6 { total_w - 2 } else { 20 }; // box width excluding left margin
    let inner_w = if box_w > 2 { box_w - 2 } else { 18 }; // content area inside borders

    // Helper: build a padded line inside the box.
    // Returns: "  │" + content_spans + padding + "│"
    // Content is expected to be pre-wrapped by callers; this layer should not truncate.
    let build_box_line = |content_spans: Vec<Span<'static>>, bs: Style, bgs: Style| -> (ListItem<'static>, String) {
        let used_width: usize = content_spans
            .iter()
            .map(|span| unicode_display_width(span.content.as_ref()))
            .sum();
        let pad = inner_w.saturating_sub(used_width);
        let content_plain = content_spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        let mut spans = Vec::with_capacity(content_spans.len() + 4);
        spans.push(Span::styled("  │".to_string(), bs)); // "  │"
        spans.extend(content_spans);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), bgs));
        }
        spans.push(Span::styled("│".to_string(), bs)); // "│"
        let plain = format!("  │{}{}│", content_plain, " ".repeat(pad));

        (ListItem::new(Line::from(spans)).style(bgs), plain)
    };

    // ── Top border: "  ╭─────...─────╮"
    let horiz_len = if inner_w > 0 { inner_w } else { 1 };
    let top_line = format!("  ╭{}╮", "─".repeat(horiz_len));
    items.push(ListItem::new(Line::from(vec![Span::styled(top_line.clone(), border_style)])).style(bg_style));
    plain_lines.push(top_line);

    // ── Title line
    let title_display = if is_running {
        format!("{} {}", spinner_frame, title)
    } else {
        title.to_string()
    };

    let mut title_content = vec![
        Span::raw("  ".to_string()),
        Span::styled(
            title_display,
            theme.style(StyleKind::Muted).add_modifier(Modifier::BOLD),
        ),
    ];

    if !duration_text.is_empty() {
        title_content.push(Span::raw("  ".to_string()));
        title_content.push(Span::styled(duration_text, theme.style(StyleKind::Muted)));
    }

    title_content.push(Span::raw("  ".to_string()));
    title_content.push(Span::styled(status_icon, status_style));

    let (title_item, title_plain) = build_box_line(title_content, border_style, bg_style);
    items.push(title_item);
    plain_lines.push(title_plain);

    // ── Content lines
    for line in content_lines {
        let mut content = Vec::with_capacity(line.spans.len() + 1);
        content.push(Span::raw("    ".to_string())); // 4-space indent for content
        content.extend(line.spans);
        let (item, plain) = build_box_line(content, border_style, bg_style);
        items.push(item);
        plain_lines.push(plain);
    }

    // ── Error line
    if let Some(err) = error {
        let err_border_style = theme.style(StyleKind::Error);
        let err_max_width = inner_w.saturating_sub(4).max(1);
        for err_line in wrap_display_lines(err, err_max_width) {
            let content = vec![
                Span::raw("    ".to_string()),
                Span::styled(err_line, theme.style(StyleKind::Error)),
            ];
            let (item, plain) = build_box_line(content, err_border_style, bg_style);
            items.push(item);
            plain_lines.push(plain);
        }
    }

    // ── Bottom border: "  ╰─────...─────╯"
    let bottom_line = format!("  ╰{}╯", "─".repeat(horiz_len));
    items.push(ListItem::new(Line::from(vec![Span::styled(bottom_line.clone(), border_style)])).style(bg_style));
    plain_lines.push(bottom_line);

    super::ToolCardRenderOutput { items, plain_lines }
}

/// Calculate the display width of a string, accounting for Unicode characters.
/// CJK characters count as 2, most others as 1.
fn unicode_display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    UnicodeWidthStr::width(s)
}

/// Status icon and style for block tool headers
pub(super) fn status_icon_and_style(status: &ToolDisplayStatus, theme: &Theme, spinner_frame: &str) -> (String, Style) {
    match status {
        ToolDisplayStatus::Running | ToolDisplayStatus::Streaming => {
            (spinner_frame.to_string(), theme.style(StyleKind::Primary))
        }
        ToolDisplayStatus::Success => ("✓".to_string(), theme.style(StyleKind::Success)), // ✓
        ToolDisplayStatus::Failed => ("✗".to_string(), theme.style(StyleKind::Error)),    // ✗
        ToolDisplayStatus::Queued => ("‖".to_string(), theme.style(StyleKind::Muted)),    // ‖
        ToolDisplayStatus::Waiting => ("…".to_string(), theme.style(StyleKind::Warning)), // …
        ToolDisplayStatus::EarlyDetected | ToolDisplayStatus::ParamsPartial => {
            (spinner_frame.to_string(), theme.style(StyleKind::Muted))
        }
        ToolDisplayStatus::ConfirmationNeeded => ("?".to_string(), theme.style(StyleKind::Warning)),
        ToolDisplayStatus::Confirmed => ("✓".to_string(), theme.style(StyleKind::Success)),
        ToolDisplayStatus::Rejected => ("✗".to_string(), theme.style(StyleKind::Error)),
        ToolDisplayStatus::Cancelled => ("—".to_string(), theme.style(StyleKind::Muted)), // —
        ToolDisplayStatus::Pending => ("—".to_string(), theme.style(StyleKind::Muted)),
    }
}

// ============ Parameter Helpers ============

/// Extract a string parameter by trying multiple key names
pub(super) fn param_str(params: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = params.get(*key).and_then(|v| v.as_str()) {
            return v.to_string();
        }
    }
    "unknown".to_string()
}

/// Extract an optional string parameter
pub(super) fn param_str_opt(params: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = params.get(*key).and_then(|v| v.as_str()) {
            return Some(v.to_string());
        }
    }
    None
}

/// Extract a key parameter summary from JSON params
pub(super) fn extract_key_params(params: &serde_json::Value) -> String {
    if let Some(obj) = params.as_object() {
        let priority_keys = [
            "path",
            "file_path",
            "target_file",
            "query",
            "pattern",
            "command",
            "message",
            "url",
        ];

        for key in &priority_keys {
            if let Some(value) = obj.get(*key) {
                if let Some(s) = value.as_str() {
                    return truncate_str(s, 60);
                }
            }
        }

        for (_key, value) in obj.iter() {
            if let Some(s) = value.as_str() {
                if s.len() < 100 {
                    return truncate_str(s, 60);
                }
            }
        }
    }

    String::new()
}

/// Capitalize the first letter of a string
pub(super) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
