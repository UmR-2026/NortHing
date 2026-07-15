/// Tool card rendering — InlineTool + BlockTool dual-layer system
///
/// Inspired by opencode TUI's InlineTool/BlockTool pattern:
/// - InlineTool: single-line for simple/exploratory tools (Read, Grep, Glob, LS, etc.)
/// - BlockTool: multi-line with left border for complex tools (Bash, Edit, Write, Task, etc.)
/// - Phase-aware: same tool can switch from Inline (pending) to Block (has output)
///
/// Split (R38a) — facade + 3 sibling sub-modules:
/// - `block_assembly`: shared block assembly, helpers, status icon
/// - `block_render`: per-tool block renderers (Bash, Edit, Write, Delete, Task, Todo, Question, Plan, Generic)
/// - `hmos_block`: HarmonyOS-specific compilation block
///
/// Public API: `clear_tool_card_cache`, `render_tool_card`, `ToolCardRenderOutput`.
/// All other items are internal (cross-sibling `pub(super)`).
use std::collections::HashMap;

use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::ListItem,
};

use super::string_utils::truncate_str;
use super::theme::{tool_icon, StyleKind, Theme};
use crate::chat_state::{ToolDisplayState, ToolDisplayStatus};
use block_assembly::{extract_key_params, param_str, param_str_opt, wrap_display_lines};

pub mod block_assembly;
pub mod block_render;
pub mod hmos_block;

// Wildcard re-export: items in sibling files are `pub(super)` (visible to
// the parent `tool_cards` module), so they are picked up by `pub use` here.
// This keeps the public surface ergonomic — callers reference
// `crate::ui::tool_cards::render_tool_card` and friends without
// spelling out `block_render::` / `hmos_block::` / `block_assembly::`.
//
// `pub use foo::*` on a glob whose items are only `pub(super)` does not
// raise their visibility — rustc warns on this. Silence it explicitly; the
// re-export still works for in-module references and matches the pattern
// used in R25/R27/R29/R31/R37 splits.
#[allow(unused_imports)]
pub use block_assembly::*;
#[allow(unused_imports)]
pub use block_render::*;
#[allow(unused_imports)]
pub use hmos_block::*;

// ============ Tool Card Render Cache ============

/// Cache key for a tool card render result
#[derive(Hash, Eq, PartialEq, Clone)]
struct ToolCardCacheKey {
    tool_id: String,
    expanded: bool,
    focused: bool,
    width: u16,
}

// Thread-local cache for completed tool card renders.
// Only caches tools in terminal states (Success/Failed/Rejected/Cancelled).
// Cleared when the session changes.
thread_local! {
    static TOOL_CARD_CACHE: std::cell::RefCell<HashMap<ToolCardCacheKey, ToolCardRenderOutput>> =
        std::cell::RefCell::new(HashMap::new());
}

#[derive(Clone)]
pub struct ToolCardRenderOutput {
    pub items: Vec<ListItem<'static>>,
    pub plain_lines: Vec<String>,
}

/// Clear the tool card render cache (call on session switch or /clear)
pub fn clear_tool_card_cache() {
    TOOL_CARD_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// Check if a tool is in a terminal (cacheable) state
fn is_terminal_status(status: &ToolDisplayStatus) -> bool {
    matches!(
        status,
        ToolDisplayStatus::Success
            | ToolDisplayStatus::Failed
            | ToolDisplayStatus::Rejected
            | ToolDisplayStatus::Cancelled
    )
}

// ============ Display Mode ============

/// Tool display mode — determines rendering strategy
#[derive(Debug, Clone, Copy, PartialEq)]
enum ToolDisplayMode {
    /// Single-line: icon + text (Read, Grep, Glob, LS, WebSearch, etc.)
    Inline,
    /// Multi-line with left border (Bash output, Edit diff, Task details, etc.)
    Block,
}

/// Determine display mode based on tool name and current state.
/// Phase-aware: same tool can switch from Inline (pending) to Block (has output).
fn tool_display_mode(tool_name: &str, tool_state: &ToolDisplayState) -> ToolDisplayMode {
    match normalize_tool_name(tool_name) {
        // Always inline tools
        "Read" | "Grep" | "Glob" | "LS" | "WebSearch" | "WebFetch" | "Skill" | "ReadLints" | "Git" | "GetFileDiff"
        | "IdeControl" | "MermaidInteractive" | "ContextCompression" | "AnalyzeImage" => ToolDisplayMode::Inline,

        // Phase-aware: Inline when pending, Block when has output/result
        "Bash" => {
            if tool_state.result.is_some()
                || matches!(
                    tool_state.status,
                    ToolDisplayStatus::Running | ToolDisplayStatus::Streaming
                )
            {
                ToolDisplayMode::Block
            } else {
                ToolDisplayMode::Inline
            }
        }
        "HmosCompilation" => {
            if matches!(
                tool_state.status,
                ToolDisplayStatus::Running | ToolDisplayStatus::Streaming | ToolDisplayStatus::Failed
            ) || tool_state.result.is_some()
            {
                ToolDisplayMode::Block
            } else {
                ToolDisplayMode::Inline
            }
        }
        "Edit" | "Write" | "Delete" => {
            if tool_state.result.is_some() {
                ToolDisplayMode::Block
            } else {
                ToolDisplayMode::Inline
            }
        }
        // Task always renders as Block — even during early detection / params streaming,
        // we want to show the subagent card with real-time progress rather than inline "Delegating...".
        "Task" => ToolDisplayMode::Block,

        // Always block tools
        "TodoWrite" | "AskUserQuestion" | "CreatePlan" => ToolDisplayMode::Block,

        // MCP tools: inline when pending, block when has output
        _ if tool_name.starts_with("mcp_") => {
            if tool_state.result.is_some() {
                ToolDisplayMode::Block
            } else {
                ToolDisplayMode::Inline
            }
        }

        // Unknown tools: inline
        _ => ToolDisplayMode::Inline,
    }
}

/// Normalize tool name to canonical form (supports both old and new naming)
fn normalize_tool_name(name: &str) -> &str {
    match name {
        "read_file" | "read_file_tool" => "Read",
        "write_file" | "write_file_tool" => "Write",
        "search_replace" => "Edit",
        "bash_tool" | "run_terminal_cmd" => "Bash",
        "codebase_search" => "Glob",
        "grep" => "Grep",
        "list_dir" | "ls" => "LS",
        other => other,
    }
}

// ============ Public API ============

/// Render a tool card. Returns a list of ListItems for the chat message list.
///
/// Parameters:
/// - `tool_state`: current tool display state
/// - `theme`: UI theme
/// - `expanded`: whether this block tool is expanded (for output truncation)
/// - `focused`: whether this tool card is currently focused (for border highlight)
/// - `spinner_frame`: current spinner animation frame (for running tools)
/// - `available_width`: terminal width available for rendering (for split diff)
pub fn render_tool_card(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> ToolCardRenderOutput {
    // Check cache for completed tools
    if is_terminal_status(&tool_state.status) {
        let key = ToolCardCacheKey {
            tool_id: tool_state.tool_id.clone(),
            expanded,
            focused,
            width: available_width,
        };
        let cached = TOOL_CARD_CACHE.with(|cache| cache.borrow().get(&key).cloned());
        if let Some(rendered) = cached {
            return rendered;
        }

        // Render and cache
        let rendered = render_tool_card_inner(tool_state, theme, expanded, focused, spinner_frame, available_width);
        let rendered_clone = rendered.clone();
        TOOL_CARD_CACHE.with(|cache| {
            cache.borrow_mut().insert(key, rendered_clone);
        });
        return rendered;
    }

    // Non-terminal tools: render without caching
    render_tool_card_inner(tool_state, theme, expanded, focused, spinner_frame, available_width)
}

/// Internal render function (no caching)
fn render_tool_card_inner(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> ToolCardRenderOutput {
    let canonical = normalize_tool_name(&tool_state.tool_name);
    let mode = tool_display_mode(&tool_state.tool_name, tool_state);

    let mut items = Vec::new();
    let mut plain_lines = Vec::new();

    // Add a top spacing line to visually separate consecutive tool cards
    items.push(ListItem::new(Line::from(Span::raw("".to_string()))));
    plain_lines.push(String::new());

    match mode {
        ToolDisplayMode::Inline => {
            let rendered = render_inline_dispatch(canonical, tool_state, theme, spinner_frame, available_width);
            items.extend(rendered.items);
            plain_lines.extend(rendered.plain_lines);
        }
        ToolDisplayMode::Block => {
            let rendered = render_block_dispatch(
                canonical,
                tool_state,
                theme,
                expanded,
                focused,
                spinner_frame,
                available_width,
            );
            items.extend(rendered.items);
            plain_lines.extend(rendered.plain_lines);
        }
    }

    if plain_lines.len() < items.len() {
        plain_lines.resize(items.len(), String::new());
    } else if plain_lines.len() > items.len() {
        plain_lines.truncate(items.len());
    }

    ToolCardRenderOutput { items, plain_lines }
}

// ============ Inline Tool Rendering ============

/// Dispatch to the appropriate inline renderer
fn render_inline_dispatch(
    canonical: &str,
    tool_state: &ToolDisplayState,
    theme: &Theme,
    spinner_frame: &str,
    available_width: u16,
) -> ToolCardRenderOutput {
    let icon = tool_icon(&tool_state.tool_name);
    let is_complete = matches!(
        tool_state.status,
        ToolDisplayStatus::Success
            | ToolDisplayStatus::Failed
            | ToolDisplayStatus::Rejected
            | ToolDisplayStatus::Cancelled
    );
    let is_error = matches!(tool_state.status, ToolDisplayStatus::Failed);
    let is_rejected = matches!(tool_state.status, ToolDisplayStatus::Rejected);
    let is_confirmation = matches!(tool_state.status, ToolDisplayStatus::ConfirmationNeeded);

    if !is_complete && !is_confirmation {
        // Pending state: spinner + pending text
        let pending_text = inline_pending_text(canonical, tool_state);
        return ToolCardRenderOutput {
            items: vec![ListItem::new(Line::from(vec![
                Span::raw("   ".to_string()),
                Span::styled(format!("{} ", spinner_frame), theme.style(StyleKind::Primary)),
                Span::styled(pending_text.clone(), theme.style(StyleKind::Muted)),
            ]))],
            plain_lines: vec![format!("   {} {}", spinner_frame, pending_text)],
        };
    }

    // Icon style: independent color for normal, error color for failures
    let icon_style = if is_error || is_rejected {
        theme.style(StyleKind::Error)
    } else if is_confirmation {
        theme.style(StyleKind::Warning)
    } else {
        theme.style(StyleKind::InlineIcon)
    };

    // Content style: muted for completed (consistent with thinking), error for failures
    let content_style = if is_error {
        theme.style(StyleKind::Error)
    } else if is_rejected {
        theme.style(StyleKind::Error).add_modifier(Modifier::CROSSED_OUT)
    } else if is_confirmation {
        theme.style(StyleKind::Warning)
    } else {
        theme.style(StyleKind::Muted)
    };

    // Display icon: use error icon for failures
    let display_icon = if is_error || is_rejected {
        "\u{2717}".to_string()
    } else {
        icon.to_string()
    };

    let content = inline_complete_text(canonical, tool_state);
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

    let mut items = vec![ListItem::new(Line::from(vec![
        Span::raw("   ".to_string()),
        Span::styled(display_icon.clone(), icon_style),
        Span::raw(" ".to_string()),
        Span::styled(content.clone(), content_style),
        Span::raw("  ".to_string()),
        Span::styled(duration_text.clone(), theme.style(StyleKind::Muted)),
    ]))];
    let mut plain_lines = vec![if duration_text.is_empty() {
        format!("   {} {}", display_icon, content)
    } else {
        format!("   {} {}  {}", display_icon, content, duration_text)
    }];

    // Show error on a second line if failed (not rejected)
    if is_error {
        if let Some(ref result) = tool_state.result {
            let max_width = available_width.saturating_sub(5).max(1) as usize;
            let wrapped = wrap_display_lines(result, max_width);
            let max_lines = 3usize;
            for line in wrapped.iter().take(max_lines) {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("     ".to_string()),
                    Span::styled(line.clone(), theme.style(StyleKind::Error)),
                ])));
                plain_lines.push(format!("     {}", line));
            }
            if wrapped.len() > max_lines {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("     ".to_string()),
                    Span::styled(
                        format!("\u{2026} ({} more lines)", wrapped.len() - max_lines),
                        theme.style(StyleKind::Muted),
                    ),
                ])));
                plain_lines.push(format!("     … ({} more lines)", wrapped.len() - max_lines));
            }
        }
    }

    ToolCardRenderOutput { items, plain_lines }
}

/// Generate pending text for inline tools
fn inline_pending_text(canonical: &str, tool_state: &ToolDisplayState) -> String {
    match canonical {
        "Read" => "Reading file...".to_string(),
        "Write" => "Preparing write...".to_string(),
        "Edit" => "Preparing edit...".to_string(),
        "Delete" => "Preparing delete...".to_string(),
        "Bash" => "Writing command...".to_string(),
        "Grep" => "Searching content...".to_string(),
        "Glob" => "Finding files...".to_string(),
        "LS" => "Listing directory...".to_string(),
        "WebSearch" => "Searching web...".to_string(),
        "WebFetch" => "Fetching from the web...".to_string(),
        "Task" => "Delegating...".to_string(),
        "TodoWrite" => "Updating todos...".to_string(),
        "HmosCompilation" => "Compiling HarmonyOS project...".to_string(),
        "Skill" => "Loading skill...".to_string(),
        "Git" => "Running git...".to_string(),
        "ReadLints" => "Checking lints...".to_string(),
        "AskUserQuestion" => "Asking questions...".to_string(),
        "CreatePlan" => "Creating plan...".to_string(),
        "GetFileDiff" => "Computing diff...".to_string(),
        _ => {
            if tool_state.tool_name.starts_with("mcp_") {
                // Parse mcp_{server}_{tool} to show a cleaner name
                let parts: Vec<&str> = tool_state.tool_name.splitn(3, '_').collect();
                let tool = if parts.len() >= 3 {
                    parts[2]
                } else {
                    &tool_state.tool_name
                };
                if let Some(ref msg) = tool_state.progress_message {
                    msg.clone()
                } else {
                    format!("Running MCP tool {}...", tool)
                }
            } else if let Some(ref msg) = tool_state.progress_message {
                msg.clone()
            } else {
                format!("Running {}...", tool_state.tool_name)
            }
        }
    }
}

/// Generate complete text for inline tools
fn inline_complete_text(canonical: &str, tool_state: &ToolDisplayState) -> String {
    match canonical {
        "Read" => {
            let path = param_str(&tool_state.parameters, &["file_path", "target_file", "path"]);
            format!("Read {}", path)
        }
        "Grep" => {
            let pattern = param_str(&tool_state.parameters, &["pattern"]);
            let path = param_str_opt(&tool_state.parameters, &["path"]);
            let count = tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("total_matches"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .or_else(|| tool_state.result.as_ref().map(|r| r.lines().count()))
                .unwrap_or(0);
            let mut text = format!("Grep \"{}\"", pattern);
            if let Some(p) = path {
                text.push_str(&format!(" in {}", p));
            }
            if count > 0 {
                text.push_str(&format!(" ({} matches)", count));
            }
            text
        }
        "Glob" => {
            let pattern = param_str(&tool_state.parameters, &["glob_pattern", "pattern", "query"]);
            let count = tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("match_count"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .or_else(|| tool_state.result.as_ref().map(|r| r.lines().count()))
                .unwrap_or(0);
            let mut text = format!("Glob \"{}\"", pattern);
            if count > 0 {
                text.push_str(&format!(" ({} matches)", count));
            }
            text
        }
        "LS" => {
            let path = param_str(&tool_state.parameters, &["target_directory", "path"]);
            let count = tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("total"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .or_else(|| tool_state.result.as_ref().map(|r| r.lines().count()))
                .unwrap_or(0);
            let mut text = format!("List {}", if path.is_empty() { "." } else { &path });
            if count > 0 {
                text.push_str(&format!(" ({} items)", count));
            }
            text
        }
        "WebSearch" => {
            let query = param_str(&tool_state.parameters, &["search_term", "query"]);
            format!("Web Search \"{}\"", query)
        }
        "WebFetch" => {
            let url = param_str(&tool_state.parameters, &["url"]);
            format!("WebFetch {}", truncate_str(&url, 60))
        }
        "Skill" => {
            let name = param_str(&tool_state.parameters, &["name", "skill_name"]);
            format!("Skill \"{}\"", name)
        }
        "Git" => {
            let cmd = param_str(&tool_state.parameters, &["command", "subcommand"]);
            format!("Git {}", truncate_str(&cmd, 60))
        }
        "ReadLints" => {
            let paths = param_str_opt(&tool_state.parameters, &["paths"]);
            match paths {
                Some(p) => format!("Lint Check {}", truncate_str(&p, 50)),
                None => "Lint Check".to_string(),
            }
        }
        "GetFileDiff" => {
            let path = param_str(&tool_state.parameters, &["file_path", "path"]);
            format!("File Diff {}", path)
        }
        "IdeControl" => {
            let action = param_str(&tool_state.parameters, &["action", "command"]);
            format!("IDE {}", action)
        }
        "MermaidInteractive" => "Mermaid Diagram".to_string(),
        "ContextCompression" => "Context Compressed".to_string(),
        "AnalyzeImage" => {
            let path = param_str(&tool_state.parameters, &["image_path", "path"]);
            format!("Analyze Image {}", path)
        }
        "HmosCompilation" => {
            let path = param_str(&tool_state.parameters, &["project_abs_path", "project_path"]);
            if path.is_empty() {
                "HarmonyOS Compile".to_string()
            } else {
                format!("HarmonyOS Compile {}", truncate_str(&path, 60))
            }
        }
        _ => {
            if tool_state.tool_name.starts_with("mcp_") {
                // Parse mcp_{server}_{tool} → "tool_name params (server)"
                let parts: Vec<&str> = tool_state.tool_name.splitn(3, '_').collect();
                let (server, tool) = if parts.len() >= 3 {
                    (parts[1], parts[2])
                } else {
                    ("mcp", tool_state.tool_name.as_str())
                };
                let summary = extract_key_params(&tool_state.parameters);
                if summary.is_empty() {
                    format!("{} ({})", tool, server)
                } else {
                    format!("{} {} ({})", tool, truncate_str(&summary, 40), server)
                }
            } else {
                // Unknown tools
                let summary = extract_key_params(&tool_state.parameters);
                if summary.is_empty() {
                    tool_state.tool_name.clone()
                } else {
                    format!("{} {}", tool_state.tool_name, truncate_str(&summary, 50))
                }
            }
        }
    }
}

// ============ Block Tool Dispatch ============

/// Dispatch to the appropriate block renderer
fn render_block_dispatch(
    canonical: &str,
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> ToolCardRenderOutput {
    match canonical {
        "Bash" => render_bash_block(tool_state, theme, expanded, focused, spinner_frame, available_width),
        "Edit" => render_edit_block(tool_state, theme, expanded, focused, spinner_frame, available_width),
        "Write" => render_write_block(tool_state, theme, expanded, focused, spinner_frame, available_width),
        "Delete" => render_delete_block(tool_state, theme, focused, spinner_frame, available_width),
        "Task" => render_task_block(tool_state, theme, focused, spinner_frame, available_width),
        "TodoWrite" => render_todo_block(tool_state, theme, focused, spinner_frame, available_width),
        "AskUserQuestion" => render_question_block(tool_state, theme, focused, spinner_frame, available_width),
        "CreatePlan" => render_plan_block(tool_state, theme, focused, spinner_frame, available_width),
        "HmosCompilation" => {
            render_hmos_compilation_block(tool_state, theme, expanded, focused, spinner_frame, available_width)
        }
        _ => render_generic_block(tool_state, theme, expanded, focused, spinner_frame, available_width),
    }
}
