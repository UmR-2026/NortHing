use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::{StyleKind, Theme};

use super::state::ModelConfigFormState;
use super::types::FormField;

/// Render the model config form popup
pub fn render(state: &ModelConfigFormState, frame: &mut Frame, area: Rect, theme: &Theme) {
    if !state.is_visible() {
        return;
    }

    let popup_width = area.width.saturating_sub(4).min(72);
    // Dynamic height: content rows + 2 (validation + hint) + 2 (border)
    let content_rows = state.display_rows().len();
    let ideal_height = (content_rows as u16 + 4).max(14);
    let popup_height = ideal_height.min(area.height.saturating_sub(2)).min(30);
    if popup_width < 30 || popup_height < 10 {
        return;
    }

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    let title = if state.editing_model_id().is_some() {
        format!(" Edit Model \u{2015} {} ", state.field_value(FormField::Name))
    } else {
        match state.provider_name() {
            Some(name) => format!(" Add Model \u{2015} {} ", name),
            None => " Add Model \u{2015} Custom ".to_string(),
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style(StyleKind::Primary))
        .style(Style::default().bg(theme.background))
        .title(title);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    if inner.height < 5 || inner.width < 20 {
        return;
    }

    // Reserve 2 rows at bottom: validation error + hint
    let content_height = inner.height.saturating_sub(2) as usize;

    let rows = state.display_rows();
    let total_rows = rows.len();

    let scroll_offset = if total_rows <= content_height {
        0
    } else {
        state.scroll_offset().min(total_rows - content_height)
    };

    let visible_end = (scroll_offset + content_height).min(total_rows);
    for (vi, row_idx) in (scroll_offset..visible_end).enumerate() {
        let y = inner.y + vi as u16;
        if y >= inner.y + inner.height.saturating_sub(2) {
            break;
        }

        let row_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };

        match &rows[row_idx] {
            super::types::DisplayRow::AdvancedHeader => {
                let sep = "\u{2500}".repeat((inner.width as usize).saturating_sub(20));
                let line = Line::from(vec![
                    Span::styled(
                        " ADVANCED ",
                        theme.style(StyleKind::Warning).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(sep, theme.style(StyleKind::Border)),
                ]);
                frame.render_widget(Paragraph::new(line), row_area);
            }
            super::types::DisplayRow::Label(field) => {
                let is_active = state.is_active_field(*field);
                let label_text = field_label(*field);
                let label_style = if is_active {
                    theme.style(StyleKind::Primary).add_modifier(Modifier::BOLD)
                } else {
                    theme.style(StyleKind::Info)
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(label_text, label_style))),
                    row_area,
                );
            }
            super::types::DisplayRow::Input(field) => {
                let is_active = state.is_active_field(*field);
                render_field_input(state, frame, row_area, *field, is_active, theme);
            }
        }
    }

    // Validation error (if any)
    let error_y = inner.y + inner.height.saturating_sub(2);
    if let Some(err) = state.validate_msg() {
        let err_area = Rect {
            x: inner.x,
            y: error_y,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" \u{26A0} {}", err),
                theme.style(StyleKind::Warning),
            ))),
            err_area,
        );
    }

    // Hint line
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint_area = Rect {
        x: inner.x,
        y: hint_y,
        width: inner.width,
        height: 1,
    };
    let adv_hint = if state.is_show_advanced() {
        "Ctrl+A: Hide advanced"
    } else {
        "Ctrl+A: Advanced"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" Tab/\u{2191}\u{2193}: Switch  Ctrl+S: Save  {}  Esc: Cancel", adv_hint),
            theme.style(StyleKind::Muted),
        ))),
        hint_area,
    );
}

/// Render a mutable version (updates visible_rows)
pub fn render_mut(state: &mut ModelConfigFormState, frame: &mut Frame, area: Rect, theme: &Theme) {
    if !state.is_visible() {
        return;
    }
    // Must match the same dynamic height calculation as render()
    let content_rows = state.display_rows().len();
    let ideal_height = (content_rows as u16 + 4).max(14);
    let popup_height = ideal_height.min(area.height.saturating_sub(2)).min(30);
    let inner_height = popup_height.saturating_sub(2);
    state.set_visible_rows(inner_height.saturating_sub(2) as usize);
    render(state, frame, area, theme);
}

fn field_label(field: FormField) -> &'static str {
    match field {
        FormField::Name => "Config Name *",
        FormField::ModelName => "Model Name *",
        FormField::BaseUrl => "Base URL *",
        FormField::ApiKey => "API Key *",
        FormField::ProviderFormat => "Provider Format",
        FormField::ContextWindow => "Context Window",
        FormField::MaxTokens => "Max Output Tokens",
        FormField::EnableThinking => "Enable Thinking",
        FormField::PreservedThinking => "Preserved Thinking",
        FormField::SkipSslVerify => "Skip SSL Verify",
        FormField::CustomHeaders => "Custom Headers (JSON)",
        FormField::CustomHeadersMode => "Custom Headers Mode",
        FormField::CustomRequestBody => "Custom Request Body (JSON)",
    }
}

fn render_field_input(
    state: &ModelConfigFormState,
    frame: &mut Frame,
    area: Rect,
    field: FormField,
    is_active: bool,
    theme: &Theme,
) {
    match field {
        // ── Select field ──
        FormField::ProviderFormat => {
            let mut spans = vec![Span::styled("  ", Style::default())];
            for (i, &fmt) in PROVIDER_FORMATS.iter().enumerate() {
                let selected = i == state.provider_format_index();
                let style = if selected && is_active {
                    Style::default()
                        .bg(theme.primary)
                        .fg(theme.selection_foreground())
                        .add_modifier(Modifier::BOLD)
                } else if selected {
                    theme.style(StyleKind::Primary).add_modifier(Modifier::BOLD)
                } else {
                    theme.style(StyleKind::Muted)
                };
                let label = if selected {
                    format!(" [{}] ", fmt)
                } else {
                    format!("  {}  ", fmt)
                };
                spans.push(Span::styled(label, style));
            }
            if is_active {
                spans.push(Span::styled(
                    "  \u{2190}\u{2192} to change",
                    theme.style(StyleKind::Muted),
                ));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }

        // ── Select field: Custom Headers Mode ──
        FormField::CustomHeadersMode => {
            let mut spans = vec![Span::styled("  ", Style::default())];
            for (i, &mode) in CUSTOM_HEADERS_MODES.iter().enumerate() {
                let selected = i == state.custom_headers_mode_index();
                let style = if selected && is_active {
                    Style::default()
                        .bg(theme.primary)
                        .fg(theme.selection_foreground())
                        .add_modifier(Modifier::BOLD)
                } else if selected {
                    theme.style(StyleKind::Primary).add_modifier(Modifier::BOLD)
                } else {
                    theme.style(StyleKind::Muted)
                };
                let label = if selected {
                    format!(" [{}] ", mode)
                } else {
                    format!("  {}  ", mode)
                };
                spans.push(Span::styled(label, style));
            }
            if is_active {
                spans.push(Span::styled(
                    "  \u{2190}\u{2192} to change",
                    theme.style(StyleKind::Muted),
                ));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }

        // ── Toggle (boolean) fields ──
        FormField::EnableThinking | FormField::PreservedThinking | FormField::SkipSslVerify => {
            let value = match field {
                FormField::EnableThinking => state.enable_thinking(),
                FormField::PreservedThinking => state.support_preserved_thinking(),
                FormField::SkipSslVerify => state.skip_ssl_verify(),
                _ => false,
            };

            let (indicator, ind_style) = if value {
                (
                    "[\u{2713}] ON ",
                    if is_active {
                        Style::default()
                            .bg(theme.primary)
                            .fg(theme.selection_foreground())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    },
                )
            } else {
                (
                    "[ ] OFF",
                    if is_active {
                        Style::default()
                            .bg(theme.primary)
                            .fg(theme.selection_foreground())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        theme.style(StyleKind::Muted)
                    },
                )
            };

            let mut spans = vec![
                Span::styled("    ", Style::default()),
                Span::styled(indicator, ind_style),
            ];

            if is_active {
                spans.push(Span::styled(
                    "  Space/Enter to toggle",
                    theme.style(StyleKind::Muted),
                ));
            }

            // Warning for skip_ssl_verify
            if field == FormField::SkipSslVerify && value {
                spans.push(Span::styled(
                    "  \u{26A0} Insecure",
                    theme.style(StyleKind::Warning),
                ));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }

        // ── Text input fields ──
        _ => {
            let value = state.field_value(field);

            let is_password = matches!(field, FormField::ApiKey);
            let display_value: String = if is_password && !value.is_empty() {
                let len = value.chars().count();
                if len <= 4 {
                    "\u{2022}".repeat(len)
                } else {
                    format!(
                        "{}{}",
                        "\u{2022}".repeat(len - 4),
                        &value[value.len().saturating_sub(4)..]
                    )
                }
            } else {
                value.to_string()
            };

            if is_active {
                let cursor_pos = state.cursor();
                let (before_raw, after_raw) = if is_password {
                    let display_len = display_value.chars().count();
                    let display_cursor = cursor_pos.min(display_len);
                    let before = display_value.chars().take(display_cursor).collect::<String>();
                    let after = display_value.chars().skip(display_cursor).collect::<String>();
                    (before, after)
                } else {
                    let cursor_byte = char_to_byte(value, cursor_pos);
                    let before = value[..cursor_byte].to_string();
                    let after = value[cursor_byte..].to_string();
                    (before, after)
                };

                let cursor_char = if after_raw.is_empty() {
                    " ".to_string()
                } else {
                    after_raw.chars().next().unwrap().to_string()
                };

                let after_cursor = if after_raw.len() > cursor_char.len() {
                    after_raw[cursor_char.len()..].to_string()
                } else {
                    String::new()
                };

                let line = Line::from(vec![
                    Span::styled(
                        "  > ",
                        theme.style(StyleKind::Primary).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(before_raw, Style::default().fg(Color::White)),
                    Span::styled(cursor_char, Style::default().fg(Color::Black).bg(Color::White)),
                    Span::styled(after_cursor, Style::default().fg(Color::White)),
                ]);
                frame.render_widget(Paragraph::new(line), area);
            } else {
                let is_empty = display_value.is_empty();
                let display = if is_empty {
                    field_placeholder(field).to_string()
                } else {
                    display_value
                };

                let style = if is_empty {
                    theme.style(StyleKind::Muted)
                } else {
                    Style::default().fg(Color::White)
                };

                // JSON validation indicator for JSON fields
                let json_hint = match field {
                    FormField::CustomHeaders | FormField::CustomRequestBody if !is_empty => {
                        if serde_json::from_str::<serde_json::Value>(value.trim()).is_ok() {
                            Some(("\u{2713}", Color::Green))
                        } else {
                            Some(("\u{2717}", Color::Red))
                        }
                    }
                    _ => None,
                };

                let mut spans =
                    vec![Span::styled("    ", Style::default()), Span::styled(display, style)];
                if let Some((mark, color)) = json_hint {
                    spans.push(Span::styled(
                        format!("  {}", mark),
                        Style::default().fg(color),
                    ));
                }

                let line = Line::from(spans);
                frame.render_widget(Paragraph::new(line), area);
            }
        }
    }
}

fn field_placeholder(field: FormField) -> &'static str {
    match field {
        FormField::Name => "e.g. My Model Config",
        FormField::ModelName => "e.g. gpt-4, claude-sonnet-4-5-20250929",
        FormField::BaseUrl => "https://api.example.com/v1/chat/completions",
        FormField::ApiKey => "Enter your API key",
        FormField::ProviderFormat => "",
        FormField::ContextWindow => "128000",
        FormField::MaxTokens => "8192",
        FormField::EnableThinking => "",
        FormField::PreservedThinking => "",
        FormField::SkipSslVerify => "",
        FormField::CustomHeaders => r#"e.g. {"X-Custom": "value"}"#,
        FormField::CustomHeadersMode => "",
        FormField::CustomRequestBody => r#"e.g. {"temperature": 1, "top_p": 0.95}"#,
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

const PROVIDER_FORMATS: [&str; 2] = ["openai", "anthropic"];
const CUSTOM_HEADERS_MODES: [&str; 2] = ["merge", "replace"];
