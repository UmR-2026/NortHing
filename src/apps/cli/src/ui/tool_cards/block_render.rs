//! Per-tool block renderers — Bash, Edit, Write, Delete, Task, Todo, Question,
//! Plan, and the generic fallback block.
//!
//! Split (R38a) — extracted from `tool_cards.rs` so the facade (`tool_cards.rs`)
//! stays a thin dispatcher, and each per-tool renderer lives in its own logical
//! group. Cross-sibling helpers (`assemble_block`, `block_content_max_width`,
//! `wrap_display_lines`, `param_str`, `param_str_opt`, `extract_key_params`,
//! `capitalize_first`, `status_icon_and_style`) live in `block_assembly`. The
//! HMOS-specific block (which has its own heavy metadata unpacking) lives in
//! `hmos_block`.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::diff_render::{self, DiffViewMode};
use super::super::string_utils::{strip_ansi_codes, wrap_to_display_width};
use super::super::syntax_highlight::{self, HighlightTheme};
use super::super::theme::{StyleKind, Theme};
use super::block_assembly::{
    assemble_block, block_content_max_width, capitalize_first, extract_key_params, param_str, param_str_opt,
    wrap_display_lines,
};
use crate::chat_state::{ToolDisplayState, ToolDisplayStatus};

/// Render a Bash tool as a block (command + output + expand/collapse)
pub(super) fn render_bash_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let command = param_str(&tool_state.parameters, &["command"]);
    let description = param_str_opt(&tool_state.parameters, &["description"]);
    let workdir = param_str_opt(&tool_state.parameters, &["working_directory", "workdir"]);
    let is_running = matches!(
        tool_state.status,
        ToolDisplayStatus::Running | ToolDisplayStatus::Streaming
    );

    // Title: "Shell" or "Shell in ~/path"
    let base_title = description.unwrap_or_else(|| "Shell".to_string());
    let title = match workdir {
        Some(ref wd) if !wd.is_empty() && wd != "." => {
            if base_title.contains(wd) {
                base_title
            } else {
                format!("{} in {}", base_title, wd)
            }
        }
        _ => base_title,
    };

    // Command line with syntax highlighting
    let hl_theme = HighlightTheme::Dark; // TODO: derive from theme
    let cmd_line = syntax_highlight::highlight_bash_command(&command, hl_theme);
    let mut cmd_spans = vec![Span::styled("$ ", theme.style(StyleKind::CommandText))];
    cmd_spans.extend(cmd_line.spans);

    let mut content_lines = vec![Line::from(cmd_spans)];

    // Extract output: prefer metadata.output (structured), fallback to result (display summary)
    let output_text = tool_state
        .metadata
        .as_ref()
        .and_then(|m| m.get("output"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| tool_state.result.clone());

    if let Some(ref output) = output_text {
        let output = output.trim();
        if !output.is_empty() {
            // Strip ANSI escape codes from the entire output first
            let clean_output = strip_ansi_codes(output);
            let mut output_lines: Vec<String> = Vec::new();
            let max_line_width = block_content_max_width(available_width);
            for line in clean_output.lines() {
                let sanitized = line.replace('\t', "    ");
                output_lines.extend(wrap_to_display_width(&sanitized, max_line_width));
            }
            let max_lines = if expanded { usize::MAX } else { 10 };

            for line in output_lines.iter().take(max_lines) {
                content_lines.push(Line::from(Span::raw(line.clone())));
            }

            if output_lines.len() > 10 && !expanded {
                content_lines.push(Line::from(Span::styled(
                    format!("… ({} more lines, Ctrl+O to expand)", output_lines.len() - 10),
                    theme.style(StyleKind::Muted),
                )));
            } else if expanded && output_lines.len() > 10 {
                content_lines.push(Line::from(Span::styled(
                    "Ctrl+O to collapse".to_string(),
                    theme.style(StyleKind::Muted),
                )));
            }
        }
    }

    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else {
        None
    };

    assemble_block(
        &title,
        content_lines,
        theme,
        is_running,
        error,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render an Edit tool as a block (file path + diff preview)
pub(super) fn render_edit_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let file_path = param_str(&tool_state.parameters, &["file_path", "target_file", "path"]);

    let mut content_lines = Vec::new();

    // Try to show diff from old_string/new_string parameters
    let old_str = tool_state.parameters.get("old_string").and_then(|v| v.as_str());
    let new_str = tool_state.parameters.get("new_string").and_then(|v| v.as_str());

    // Compute stats for title
    let (additions, deletions) = match (old_str, new_str) {
        (Some(old), Some(new)) => diff_render::diff_stats(old, new),
        _ => (0, 0),
    };

    // Title with stats
    let title = if additions > 0 || deletions > 0 {
        format!("Edit {} (+{}, -{})", file_path, additions, deletions)
    } else {
        format!("Edit {}", file_path)
    };

    if let (Some(old), Some(new)) = (old_str, new_str) {
        let max = if expanded { usize::MAX } else { 8 };
        // Use the block's available width minus border overhead (~8 chars)
        let diff_width = available_width.saturating_sub(8);
        let diff_lines = diff_render::render_diff(old, new, theme, max, DiffViewMode::Auto, diff_width);
        content_lines.extend(diff_lines);

        let total_changes = additions + deletions;
        if total_changes > max && !expanded {
            content_lines.push(Line::from(Span::styled(
                format!("… (more changes, Ctrl+O to expand)"),
                theme.style(StyleKind::Muted),
            )));
        } else if expanded && total_changes > 8 {
            content_lines.push(Line::from(Span::styled(
                "Ctrl+O to collapse".to_string(),
                theme.style(StyleKind::Muted),
            )));
        }
    }

    // Show result summary (now a clean display_summary, not raw JSON)
    if let Some(ref result) = tool_state.result {
        if !result.is_empty() {
            let max_width = block_content_max_width(available_width);
            for line in wrap_display_lines(result, max_width) {
                content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Success))));
            }
        }
    }

    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else {
        None
    };

    assemble_block(
        &title,
        content_lines,
        theme,
        false,
        error,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a Write tool as a block (file path + syntax-highlighted content preview)
pub(super) fn render_write_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let file_path = param_str(&tool_state.parameters, &["file_path", "target_file", "path"]);

    let mut content_lines = Vec::new();

    // Show content preview with syntax highlighting and line numbers
    if let Some(content) = tool_state
        .parameters
        .get("contents")
        .or_else(|| tool_state.parameters.get("content"))
        .and_then(|v| v.as_str())
    {
        let total_lines = content.lines().count();
        let max = if expanded { usize::MAX } else { 8 };
        let ext = syntax_highlight::ext_from_path(&file_path);
        let hl_theme = HighlightTheme::Dark; // TODO: derive from theme

        // Use syntax highlighting with line numbers
        let highlighted = syntax_highlight::highlight_code_with_line_numbers(
            content,
            ext,
            hl_theme,
            theme.style(StyleKind::DiffLineNumber),
            theme.style(StyleKind::Muted),
        );

        for line in highlighted.into_iter().take(max) {
            content_lines.push(line);
        }

        if total_lines > 8 && !expanded {
            content_lines.push(Line::from(Span::styled(
                format!("… ({} more lines, Ctrl+O to expand)", total_lines - 8),
                theme.style(StyleKind::Muted),
            )));
        } else if expanded && total_lines > 8 {
            content_lines.push(Line::from(Span::styled(
                "Ctrl+O to collapse".to_string(),
                theme.style(StyleKind::Muted),
            )));
        }

        // Title with line count
        let title = format!("Write {} ({} lines)", file_path, total_lines);

        if let Some(ref result) = tool_state.result {
            if !result.is_empty() {
                let max_width = block_content_max_width(available_width);
                for line in wrap_display_lines(result, max_width) {
                    content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Success))));
                }
            }
        }

        let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
            tool_state.result.as_deref()
        } else {
            None
        };

        return assemble_block(
            &title,
            content_lines,
            theme,
            false,
            error,
            focused,
            tool_state,
            spinner_frame,
            available_width,
        );
    }

    // Fallback: no content available
    let title = format!("Write {}", file_path);

    if let Some(ref result) = tool_state.result {
        if !result.is_empty() {
            let max_width = block_content_max_width(available_width);
            for line in wrap_display_lines(result, max_width) {
                content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Success))));
            }
        }
    }

    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else {
        None
    };

    assemble_block(
        &title,
        content_lines,
        theme,
        false,
        error,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a Delete tool as a block
pub(super) fn render_delete_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let file_path = param_str(&tool_state.parameters, &["file_path", "target_file", "path"]);
    let title = format!("Delete {}", file_path);

    let mut content_lines = Vec::new();
    if let Some(ref result) = tool_state.result {
        if !result.is_empty() {
            let max_width = block_content_max_width(available_width);
            for line in wrap_display_lines(result, max_width) {
                content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Muted))));
            }
        }
    }

    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else {
        None
    };

    assemble_block(
        &title,
        content_lines,
        theme,
        false,
        error,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a Task tool as a block (sub-agent type + description + real-time progress)
pub(super) fn render_task_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let subagent_type =
        param_str_opt(&tool_state.parameters, &["subagent_type"]).unwrap_or_else(|| "Unknown".to_string());
    let description = param_str_opt(&tool_state.parameters, &["description"]).unwrap_or_else(|| "Task".to_string());
    let is_running = matches!(
        tool_state.status,
        ToolDisplayStatus::Running | ToolDisplayStatus::Streaming
    );

    let title = format!("{} Task", capitalize_first(&subagent_type));

    // Build description line with tool call count (if available)
    let desc_text = if let Some(ref progress) = tool_state.subagent_progress {
        if progress.tool_count > 0 {
            format!("{} ({} toolcalls)", description, progress.tool_count)
        } else {
            description.clone()
        }
    } else {
        description.clone()
    };

    let mut content_lines = vec![Line::from(Span::styled(desc_text, theme.style(StyleKind::Muted)))];

    // Show real-time subagent progress (current tool being executed)
    if is_running {
        if let Some(ref progress) = tool_state.subagent_progress {
            if let Some(ref tool_name) = progress.current_tool_name {
                let progress_text = if let Some(ref title) = progress.current_tool_title {
                    format!("└ {} {}", capitalize_first(tool_name), title)
                // └
                } else {
                    format!("└ {}", capitalize_first(tool_name)) // └
                };
                content_lines.push(Line::from(Span::styled(progress_text, theme.style(StyleKind::Muted))));
            }
        }
    }

    // Show final result when completed
    if let Some(ref result) = tool_state.result {
        let max_width = block_content_max_width(available_width).saturating_sub(2).max(1);
        for line in wrap_display_lines(result, max_width) {
            content_lines.push(Line::from(Span::styled(
                format!("└ {}", line), // └
                theme.style(StyleKind::Success),
            )));
        }
    }

    assemble_block(
        &title,
        content_lines,
        theme,
        is_running,
        None,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a TodoWrite tool as a block (todo list with upgraded icons)
pub(super) fn render_todo_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let mut content_lines = Vec::new();

    if let Some(todos) = tool_state.parameters.get("todos").and_then(|v| v.as_array()) {
        for todo in todos {
            let status = todo.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
            let content = todo.get("content").and_then(|v| v.as_str()).unwrap_or("");

            let (marker, marker_style, content_style) = match status {
                "completed" => (
                    "✓", // ✓
                    theme.style(StyleKind::Success),
                    theme.style(StyleKind::Muted).add_modifier(Modifier::CROSSED_OUT),
                ),
                "in_progress" => (
                    "●", // ●
                    theme.style(StyleKind::Warning),
                    theme.style(StyleKind::Warning),
                ),
                "cancelled" => (
                    "—", // —
                    theme.style(StyleKind::Muted),
                    theme.style(StyleKind::Muted).add_modifier(Modifier::CROSSED_OUT),
                ),
                _ => (
                    "○", // ○
                    theme.style(StyleKind::Primary),
                    Style::default(),
                ),
            };

            content_lines.push(Line::from(vec![
                Span::styled(format!("{} ", marker), marker_style),
                Span::styled(content.to_string(), content_style),
            ]));
        }
    }

    if content_lines.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "Updating todos...",
            theme.style(StyleKind::Muted),
        )));
    }

    assemble_block(
        "Todos",
        content_lines,
        theme,
        false,
        None,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render an AskUserQuestion tool as a block
pub(super) fn render_question_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let mut content_lines = Vec::new();

    // If completed, show answer summary instead of options
    if tool_state.status == ToolDisplayStatus::Success {
        if let Some(ref result) = tool_state.result {
            // Try to parse the result as JSON to show answers
            if let Ok(result_val) = serde_json::from_str::<serde_json::Value>(result) {
                if let Some(obj) = result_val.as_object() {
                    for (key, val) in obj {
                        let answer_text = match val {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Array(arr) => {
                                let items: Vec<String> =
                                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                                items.join(", ")
                            }
                            _ => val.to_string(),
                        };
                        let key_prefix = format!("{}: ", key);
                        let value_width = block_content_max_width(available_width)
                            .saturating_sub(key_prefix.len())
                            .max(1);
                        let wrapped_answers = wrap_display_lines(&answer_text, value_width);
                        if let Some(first) = wrapped_answers.first() {
                            content_lines.push(Line::from(vec![
                                Span::styled(key_prefix.clone(), theme.style(StyleKind::Muted)),
                                Span::styled(first.clone(), theme.style(StyleKind::Success)),
                            ]));
                        }
                        for line in wrapped_answers.iter().skip(1) {
                            content_lines.push(Line::from(vec![
                                Span::styled(" ".repeat(key_prefix.len()), theme.style(StyleKind::Muted)),
                                Span::styled(line.clone(), theme.style(StyleKind::Success)),
                            ]));
                        }
                    }
                }
                if content_lines.is_empty() {
                    content_lines.push(Line::from(Span::styled(
                        "Answered".to_string(),
                        theme.style(StyleKind::Success),
                    )));
                }
            } else {
                let max_width = block_content_max_width(available_width);
                for line in wrap_display_lines(result, max_width) {
                    content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Success))));
                }
            }
        } else {
            content_lines.push(Line::from(Span::styled(
                "Answered".to_string(),
                theme.style(StyleKind::Success),
            )));
        }
    } else if tool_state.status == ToolDisplayStatus::Running {
        if let Some(questions) = tool_state.parameters.get("questions").and_then(|v| v.as_array()) {
            for q in questions {
                let question_text = q.get("question").and_then(|v| v.as_str()).unwrap_or("?");
                content_lines.push(Line::from(Span::styled(
                    question_text.to_string(),
                    theme.style(StyleKind::Info),
                )));
            }
        }
        content_lines.push(Line::from(Span::styled(
            "Waiting for your answer...".to_string(),
            theme.style(StyleKind::Warning),
        )));
    } else {
        if let Some(questions) = tool_state.parameters.get("questions").and_then(|v| v.as_array()) {
            for q in questions {
                let prompt = q
                    .get("question")
                    .and_then(|v| v.as_str())
                    .or_else(|| q.get("prompt").and_then(|v| v.as_str()))
                    .unwrap_or("?");
                content_lines.push(Line::from(Span::styled(
                    prompt.to_string(),
                    theme.style(StyleKind::Info),
                )));

                if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                    for opt in options {
                        let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("?");
                        content_lines.push(Line::from(vec![
                            Span::raw("  ".to_string()),
                            Span::styled("• ".to_string(), theme.style(StyleKind::Muted)),
                            Span::raw(label.to_string()),
                        ]));
                    }
                }
            }
        }

        if content_lines.is_empty() {
            content_lines.push(Line::from(Span::styled(
                "Asking questions...",
                theme.style(StyleKind::Muted),
            )));
        }
    }

    assemble_block(
        "Questions",
        content_lines,
        theme,
        false,
        None,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a CreatePlan tool as a block
pub(super) fn render_plan_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let mut content_lines = Vec::new();

    let title_text = param_str_opt(&tool_state.parameters, &["title", "name"]).unwrap_or_else(|| "Plan".to_string());

    if let Some(steps) = tool_state.parameters.get("steps").and_then(|v| v.as_array()) {
        for (i, step) in steps.iter().enumerate() {
            let desc = step
                .as_str()
                .or_else(|| step.get("description").and_then(|v| v.as_str()))
                .unwrap_or("...");
            content_lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), theme.style(StyleKind::Muted)),
                Span::raw(desc.to_string()),
            ]));
        }
    }

    if let Some(ref result) = tool_state.result {
        let max_width = block_content_max_width(available_width);
        for line in wrap_display_lines(result, max_width) {
            content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Success))));
        }
    }

    if content_lines.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "Creating plan...",
            theme.style(StyleKind::Muted),
        )));
    }

    assemble_block(
        &title_text,
        content_lines,
        theme,
        false,
        None,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}

/// Render a generic block tool (fallback for unknown block tools)
pub(super) fn render_generic_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let title = tool_state.tool_name.clone();
    let mut content_lines = Vec::new();

    let summary = extract_key_params(&tool_state.parameters);
    if !summary.is_empty() {
        content_lines.push(Line::from(Span::styled(summary, theme.style(StyleKind::Info))));
    }

    if let Some(ref msg) = tool_state.progress_message {
        for line in wrap_display_lines(msg, block_content_max_width(available_width)) {
            content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Muted))));
        }
    }

    if let Some(ref result) = tool_state.result {
        let lines = wrap_display_lines(result, block_content_max_width(available_width));
        let max = if expanded { usize::MAX } else { 5 };
        for line in lines.iter().take(max) {
            content_lines.push(Line::from(Span::raw(line.clone())));
        }
        if lines.len() > max {
            content_lines.push(Line::from(Span::styled(
                format!("▼ {} more lines (Tab/Click to expand)", lines.len() - max),
                theme.style(StyleKind::Muted),
            )));
        }
    }

    let is_running = matches!(
        tool_state.status,
        ToolDisplayStatus::Running | ToolDisplayStatus::Streaming
    );
    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else {
        None
    };

    assemble_block(
        &title,
        content_lines,
        theme,
        is_running,
        error,
        focused,
        tool_state,
        spinner_frame,
        available_width,
    )
}
