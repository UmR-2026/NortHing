//! HMOS compilation block — HarmonyOS-specific DevEco Studio toolchain
//! compilation block renderer.
//!
//! Split (R38a) — pulled out of `tool_cards.rs` so the generic block
//! renderers in `block_render` are not burdened with the heavy metadata
//! unpacking + filtering logic specific to this tool. Cross-sibling helpers
//! (`assemble_block`, `block_content_max_width`, `wrap_display_lines`,
//! `param_str_opt`) live in `block_assembly`.

use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use super::super::string_utils::{strip_ansi_codes, truncate_str};
use super::super::theme::{StyleKind, Theme};
use super::block_assembly::{assemble_block, block_content_max_width, param_str_opt, wrap_display_lines};
use crate::chat_state::{ToolDisplayState, ToolDisplayStatus};

/// Filter HMOS stderr output: keep ERROR lines and any context lines, but drop
/// the surrounding WARN-only blocks. Returns the filtered line slices.
fn filter_hmos_errors(stderr: &str) -> Vec<&str> {
    if stderr.trim().is_empty() {
        return Vec::new();
    }
    if !stderr.contains("ERROR") {
        return stderr.lines().collect();
    }

    let mut lines = Vec::new();
    let mut skipping_warning_block = false;
    for line in stderr.lines() {
        if line.contains("WARN") {
            skipping_warning_block = true;
            continue;
        }
        if line.contains("ERROR") {
            skipping_warning_block = false;
            lines.push(line);
            continue;
        }
        if !skipping_warning_block {
            lines.push(line);
        }
    }
    lines
}

#[cfg(target_os = "macos")]
const DEVECO_HOME_HELP_FALLBACK: &str = "Set DEVECO_HOME to the DevEco Studio installation directory.\nmacOS example (zsh):\nexport DEVECO_HOME=\"/Applications/DevEco Studio.app/Contents\"\nRestart the terminal after setting it.";

#[cfg(target_os = "windows")]
const DEVECO_HOME_HELP_FALLBACK: &str = "Set DEVECO_HOME to the DevEco Studio installation directory.\nWindows PowerShell example:\n[Environment]::SetEnvironmentVariable(\"DEVECO_HOME\",\"C:\\Program Files\\DevEco Studio\",\"User\")\nRestart the terminal after setting it.";

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
const DEVECO_HOME_HELP_FALLBACK: &str = "Set DEVECO_HOME to the DevEco Studio installation directory.\nLinux example (bash):\nexport DEVECO_HOME=\"$HOME/DevEco-Studio\"\nRestart the terminal after setting it.";

/// Render a HarmonyOS compilation tool as a block
pub(super) fn render_hmos_compilation_block(
    tool_state: &ToolDisplayState,
    theme: &Theme,
    expanded: bool,
    focused: bool,
    spinner_frame: &str,
    available_width: u16,
) -> super::ToolCardRenderOutput {
    let is_running = matches!(
        tool_state.status,
        ToolDisplayStatus::Running | ToolDisplayStatus::Streaming
    );

    let project_path = param_str_opt(&tool_state.parameters, &["project_abs_path", "project_path"])
        .or_else(|| {
            tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("project_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    let product = param_str_opt(&tool_state.parameters, &["product"])
        .or_else(|| {
            tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("product"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "default".to_string());
    let build_mode = param_str_opt(&tool_state.parameters, &["build_mode"])
        .or_else(|| {
            tool_state
                .metadata
                .as_ref()
                .and_then(|m| m.get("build_mode"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "debug".to_string());

    let (success, exit_code, execution_time_ms, deveco_home, stderr, stdout, error_kind, error_message, help) =
        tool_state
            .metadata
            .as_ref()
            .and_then(|m| m.as_object())
            .map(|obj| {
                let success = obj.get("success").and_then(|v| v.as_bool());
                let exit_code = obj.get("exit_code").and_then(|v| v.as_i64());
                let execution_time_ms = obj.get("execution_time_ms").and_then(|v| v.as_u64());
                let deveco_home = obj.get("deveco_home").and_then(|v| v.as_str()).map(|s| s.to_string());
                let stderr = obj.get("stderr").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let stdout = obj.get("stdout").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let error_kind = obj.get("error_kind").and_then(|v| v.as_str()).map(|s| s.to_string());
                let error_message = obj.get("error_message").and_then(|v| v.as_str()).map(|s| s.to_string());
                let help = obj.get("help").and_then(|v| v.as_str()).map(|s| s.to_string());
                (
                    success,
                    exit_code,
                    execution_time_ms,
                    deveco_home,
                    stderr,
                    stdout,
                    error_kind,
                    error_message,
                    help,
                )
            })
            .unwrap_or((None, None, None, None, String::new(), String::new(), None, None, None));

    let mut title = "HarmonyOS Compile".to_string();
    if !project_path.is_empty() {
        title.push_str(&format!(" {}", truncate_str(&project_path, 50)));
    }

    let mut content_lines = Vec::new();

    content_lines.push(Line::from(vec![
        Span::styled("mode: ", theme.style(StyleKind::Muted)),
        Span::styled(
            format!("product={} buildMode={}", product, build_mode),
            theme.style(StyleKind::Info),
        ),
    ]));

    if let Some(home) = deveco_home {
        content_lines.push(Line::from(vec![
            Span::styled("DevEco: ", theme.style(StyleKind::Muted)),
            Span::raw(truncate_str(&home, 80)),
        ]));
    }

    if let Some(ms) = execution_time_ms {
        content_lines.push(Line::from(vec![
            Span::styled("exec: ", theme.style(StyleKind::Muted)),
            Span::raw(format!("{}ms", ms)),
        ]));
    }

    let status_line = match success {
        Some(true) => Some((true, "succeeded".to_string())),
        Some(false) => Some((false, "failed".to_string())),
        None => None,
    };

    if let Some((ok, status_text)) = status_line {
        let style = if ok {
            theme.style(StyleKind::Success)
        } else {
            theme.style(StyleKind::Error)
        };
        let mut text = format!("status: {}", status_text);
        if let Some(code) = exit_code {
            text.push_str(&format!(" (exit_code={})", code));
        }
        content_lines.push(Line::from(Span::styled(text, style)));
    }

    let inferred_missing_deveco_home = matches!(tool_state.result.as_deref(), Some(s) if s.contains("DEVECO_HOME"));

    let should_show_deveco_hint = matches!(
        error_kind.as_deref(),
        Some("missing_deveco_home") | Some("invalid_deveco_home")
    ) || inferred_missing_deveco_home;

    let max_line_width = block_content_max_width(available_width);

    if !is_running && should_show_deveco_hint {
        let headline = match error_kind.as_deref() {
            Some("missing_deveco_home") => "DEVECO_HOME is not set.",
            Some("invalid_deveco_home") => "DEVECO_HOME is set but looks invalid.",
            _ => "DevEco Studio toolchain not detected.",
        };
        content_lines.push(Line::from(vec![
            Span::styled("hint: ", theme.style(StyleKind::Muted)),
            Span::styled(headline, theme.style(StyleKind::Info)),
        ]));

        let display_error_message = error_message.clone().or_else(|| tool_state.result.clone());
        if let Some(msg) = display_error_message.as_deref() {
            if !msg.trim().is_empty() {
                let wrapped = wrap_display_lines(msg, max_line_width.saturating_sub(7).max(1));
                if let Some(first) = wrapped.first() {
                    content_lines.push(Line::from(vec![
                        Span::styled("error: ", theme.style(StyleKind::Muted)),
                        Span::raw(first.clone()),
                    ]));
                }
                for line in wrapped.iter().skip(1) {
                    content_lines.push(Line::from(vec![
                        Span::styled("       ", theme.style(StyleKind::Muted)),
                        Span::raw(line.clone()),
                    ]));
                }
            }
        }

        let help_text = help.as_deref().unwrap_or(DEVECO_HOME_HELP_FALLBACK);
        let mut help_lines_wrapped: Vec<String> = Vec::new();
        for line in help_text.lines() {
            let clean = strip_ansi_codes(line);
            if clean.trim().is_empty() {
                continue;
            }
            help_lines_wrapped.extend(wrap_display_lines(&clean, max_line_width));
        }
        let max = if expanded { usize::MAX } else { 6 };
        for line in help_lines_wrapped.iter().take(max) {
            content_lines.push(Line::from(Span::styled(line.clone(), theme.style(StyleKind::Muted))));
        }
        if help_lines_wrapped.len() > max {
            content_lines.push(Line::from(Span::styled(
                format!("▼ {} more lines (Tab/Click to expand)", help_lines_wrapped.len() - max),
                theme.style(StyleKind::Muted),
            )));
        }
    }

    if !is_running {
        if matches!(success, Some(false)) {
            let filtered = filter_hmos_errors(&stderr);
            let mut wrapped_filtered: Vec<String> = Vec::new();
            for line in &filtered {
                let clean = strip_ansi_codes(line);
                if clean.trim().is_empty() {
                    continue;
                }
                wrapped_filtered.extend(wrap_display_lines(&clean, max_line_width));
            }
            let max = if expanded { usize::MAX } else { 12 };
            for line in wrapped_filtered.iter().take(max) {
                content_lines.push(Line::from(Span::raw(line.clone())));
            }
            if wrapped_filtered.len() > max {
                content_lines.push(Line::from(Span::styled(
                    format!("▼ {} more lines (Tab/Click to expand)", wrapped_filtered.len() - max),
                    theme.style(StyleKind::Muted),
                )));
            }
        } else if matches!(success, Some(true)) {
            let output_lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
            let mut wrapped_output: Vec<String> = Vec::new();
            for line in output_lines {
                let clean = strip_ansi_codes(line);
                wrapped_output.extend(wrap_display_lines(&clean, max_line_width));
            }
            let max = if expanded { usize::MAX } else { 5 };
            for line in wrapped_output.iter().rev().take(max).rev() {
                content_lines.push(Line::from(Span::raw(line.clone())));
            }
            if !wrapped_output.is_empty() && wrapped_output.len() > max {
                content_lines.push(Line::from(Span::styled(
                    format!("▼ {} more lines (Tab/Click to expand)", wrapped_output.len() - max),
                    theme.style(StyleKind::Muted),
                )));
            }
        } else if let Some(ref result) = tool_state.result {
            let wrapped = wrap_display_lines(result, max_line_width);
            let max = if expanded { usize::MAX } else { 6 };
            for line in wrapped.iter().take(max) {
                content_lines.push(Line::from(Span::styled(line.clone(), theme.style(StyleKind::Muted))));
            }
            if wrapped.len() > max {
                content_lines.push(Line::from(Span::styled(
                    format!("▼ {} more lines (Tab/Click to expand)", wrapped.len() - max),
                    theme.style(StyleKind::Muted),
                )));
            }
        }
    } else if let Some(ref msg) = tool_state.progress_message {
        for line in wrap_display_lines(msg, max_line_width) {
            content_lines.push(Line::from(Span::styled(line, theme.style(StyleKind::Muted))));
        }
    } else {
        content_lines.push(Line::from(Span::styled("Compiling...", theme.style(StyleKind::Muted))));
    }

    let error = if matches!(tool_state.status, ToolDisplayStatus::Failed) {
        tool_state.result.as_deref()
    } else if matches!(success, Some(false)) {
        Some("Compilation failed")
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

// `ListItem` is referenced via the returned `super::ToolCardRenderOutput`
// type from `assemble_block`. Keep a tiny reference to avoid unused-import
// warnings if future edits drop the explicit `ListItem` mention.
#[allow(dead_code)]
const _LIST_ITEM_DUMMY: Option<ListItem<'static>> = None;
