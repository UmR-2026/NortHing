use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::{StyleKind, Theme};

use super::types::QuestionPrompt;

/// Render the question overlay on top of the message area.
pub fn render_question_overlay(
    frame: &mut Frame,
    prompt: &QuestionPrompt,
    theme: &Theme,
    area: Rect,
) {
    if prompt.on_confirm_page() {
        render_confirm_page(frame, prompt, theme, area);
    } else {
        render_question_page(frame, prompt, theme, area);
    }
}

/// Render a single question page with options
fn render_question_page(
    frame: &mut Frame,
    prompt: &QuestionPrompt,
    theme: &Theme,
    area: Rect,
) {
    let q = match prompt.current_question() {
        Some(q) => q,
        None => return,
    };

    // Calculate overlay height: header(1) + question(2) + options + other + hint(2) + padding
    let options_count = q.options.len() + 1; // +1 for "Other"
    let description_lines: usize = q
        .options
        .iter()
        .map(|o| if o.description.is_empty() { 0 } else { 1 })
        .sum();
    let tab_line = if prompt.tab_count() > 1 { 2 } else { 0 };
    let editing_line = if prompt.editing_custom { 1 } else { 0 };
    let content_height = 2 + tab_line + options_count + description_lines + editing_line + 1; // question + options + descriptions + padding
    let overlay_height = (content_height as u16 + 3).min(area.height.saturating_sub(2)); // +3 for borders/hint

    let overlay_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(overlay_height),
        width: area.width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // content
            Constraint::Length(2), // hint bar
        ])
        .split(overlay_area);

    // Content block with accent left border
    let content_block = Block::default()
        .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.background_panel));

    let inner = content_block.inner(chunks[0]);
    frame.render_widget(content_block, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    // Tab bar (multi-question only)
    if prompt.tab_count() > 1 {
        let mut tab_spans = Vec::new();
        for (i, qd) in prompt.questions.iter().enumerate() {
            if i > 0 {
                tab_spans.push(Span::raw("  "));
            }
            let is_active = i == prompt.current_tab;
            let is_answered = !prompt.answers[i].is_empty();
            if is_active {
                tab_spans.push(Span::styled(
                    format!(" {} ", qd.header),
                    Style::default()
                        .fg(theme.selection_foreground())
                        .bg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_spans.push(Span::styled(
                    format!(" {} ", qd.header),
                    Style::default().fg(if is_answered { theme.success } else { theme.muted }),
                ));
            }
        }
        // Confirm tab
        tab_spans.push(Span::raw("  "));
        let confirm_active = prompt.on_confirm_page();
        if confirm_active {
            tab_spans.push(Span::styled(
                " Confirm ",
                Style::default()
                    .fg(theme.selection_foreground())
                    .bg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(" Confirm ", theme.style(StyleKind::Muted)));
        }
        lines.push(Line::from(tab_spans));
        lines.push(Line::from(""));
    }

    // Question text
    let multi_hint = if q.multi_select {
        " (select all that apply)"
    } else {
        ""
    };
    lines.push(Line::from(Span::styled(
        format!("{}{}", q.question, multi_hint),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Options
    for (i, opt) in q.options.iter().enumerate() {
        let is_active = i == prompt.selected_option;
        let is_picked = prompt.answers[prompt.current_tab].contains(&opt.label);

        let number_style = if is_active {
            theme.style(StyleKind::Primary)
        } else {
            theme.style(StyleKind::Muted)
        };

        let label_style = if is_active {
            Style::default().fg(theme.primary)
        } else if is_picked {
            Style::default().fg(theme.success)
        } else {
            Style::default()
        };

        let marker = if q.multi_select {
            if is_picked {
                "[\u{2713}]"
            } else {
                "[ ]"
            }
        } else {
            if is_picked {
                "(\u{2022})"
            } else {
                "( )"
            }
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}. ", i + 1), number_style),
            Span::styled(format!("{} ", marker), label_style),
            Span::styled(opt.label.clone(), label_style),
            if is_picked && !q.multi_select {
                Span::styled(" \u{2713}", theme.style(StyleKind::Success))
            } else {
                Span::raw("")
            },
        ]));

        if !opt.description.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(opt.description.clone(), theme.style(StyleKind::Muted)),
            ]));
        }
    }

    // "Other" option
    let other_idx = q.options.len();
    let is_other_active = prompt.selected_option == other_idx;
    let custom_text = &prompt.custom_inputs[prompt.current_tab];
    let is_other_picked = prompt.answers[prompt.current_tab].contains(&"Other".to_string());

    let other_style = if is_other_active {
        Style::default().fg(theme.primary)
    } else if is_other_picked {
        Style::default().fg(theme.success)
    } else {
        Style::default()
    };

    let other_marker = if q.multi_select {
        if is_other_picked {
            "[\u{2713}]"
        } else {
            "[ ]"
        }
    } else {
        if is_other_picked {
            "(\u{2022})"
        } else {
            "( )"
        }
    };

    lines.push(Line::from(vec![
        Span::styled(
            format!("{}. ", other_idx + 1),
            if is_other_active {
                theme.style(StyleKind::Primary)
            } else {
                theme.style(StyleKind::Muted)
            },
        ),
        Span::styled(format!("{} ", other_marker), other_style),
        Span::styled("Type your own answer", other_style),
    ]));

    // Show custom input field when editing
    if prompt.editing_custom {
        let display = if custom_text.is_empty() {
            "(type your answer)".to_string()
        } else {
            format!("{}\u{2588}", custom_text) // cursor block
        };
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(
                display,
                if custom_text.is_empty() {
                    theme.style(StyleKind::Muted)
                } else {
                    Style::default()
                },
            ),
        ]));
    } else if !custom_text.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(custom_text.clone(), theme.style(StyleKind::Muted)),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);

    // Hint bar
    render_question_hint_bar(frame, chunks[1], theme, prompt);
}

/// Render the confirm/review page (multi-question)
fn render_confirm_page(
    frame: &mut Frame,
    prompt: &QuestionPrompt,
    theme: &Theme,
    area: Rect,
) {
    let content_height = 3 + prompt.questions.len(); // title + blank + questions + padding
    let tab_line = 2;
    let overlay_height =
        ((content_height + tab_line) as u16 + 4).min(area.height.saturating_sub(2));

    let overlay_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(overlay_height),
        width: area.width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(overlay_area);

    let content_block = Block::default()
        .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.background_panel));

    let inner = content_block.inner(chunks[0]);
    frame.render_widget(content_block, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    // Tab bar
    let mut tab_spans = Vec::new();
    for (i, qd) in prompt.questions.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw("  "));
        }
        let is_answered = !prompt.answers[i].is_empty();
        tab_spans.push(Span::styled(
            format!(" {} ", qd.header),
            Style::default().fg(if is_answered { theme.success } else { theme.muted }),
        ));
    }
    tab_spans.push(Span::raw("  "));
    tab_spans.push(Span::styled(
        " Confirm ",
        Style::default()
            .fg(theme.selection_foreground())
            .bg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::from(tab_spans));
    lines.push(Line::from(""));

    // Review title
    lines.push(Line::from(Span::styled(
        "Review your answers",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Answer summary per question
    for (i, q) in prompt.questions.iter().enumerate() {
        let answer_list = &prompt.answers[i];
        let custom = &prompt.custom_inputs[i];

        let display_answers: Vec<String> = answer_list
            .iter()
            .map(|a| {
                if a == "Other" && !custom.is_empty() {
                    custom.clone()
                } else {
                    a.clone()
                }
            })
            .collect();

        let answered = !display_answers.is_empty();
        let value_text = if answered {
            display_answers.join(", ")
        } else {
            "(not answered)".to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", q.header), theme.style(StyleKind::Muted)),
            Span::styled(
                value_text,
                if answered {
                    Style::default()
                } else {
                    theme.style(StyleKind::Error)
                },
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);

    // Hint bar
    let hint_block = Block::default().style(Style::default().bg(theme.background_element));
    frame.render_widget(hint_block, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("Enter", Style::default()),
        Span::styled(" submit  ", theme.style(StyleKind::Muted)),
        Span::styled("Tab", Style::default()),
        Span::styled(" back  ", theme.style(StyleKind::Muted)),
        Span::styled("Esc", Style::default()),
        Span::styled(" dismiss", theme.style(StyleKind::Muted)),
    ]))
    .style(Style::default().bg(theme.background_element));
    frame.render_widget(hint, chunks[1]);
}

/// Render the hint bar for question pages
fn render_question_hint_bar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    prompt: &QuestionPrompt,
) {
    let hint_block = Block::default().style(Style::default().bg(theme.background_element));
    frame.render_widget(hint_block, area);

    let mut spans = vec![Span::raw(" ")];

    if prompt.editing_custom {
        spans.push(Span::styled("Enter", Style::default()));
        spans.push(Span::styled(" confirm  ", theme.style(StyleKind::Muted)));
        spans.push(Span::styled("Esc", Style::default()));
        spans.push(Span::styled(" cancel", theme.style(StyleKind::Muted)));
    } else {
        if prompt.tab_count() > 1 {
            spans.push(Span::styled("Tab", Style::default()));
            spans.push(Span::styled(" switch  ", theme.style(StyleKind::Muted)));
        }
        spans.push(Span::styled("\u{2191}\u{2193}", Style::default()));
        spans.push(Span::styled(" select  ", theme.style(StyleKind::Muted)));
        spans.push(Span::styled("Enter", Style::default()));

        let q = prompt.current_question();
        let action_text = if q.map(|q| q.multi_select).unwrap_or(false) {
            " toggle  "
        } else if prompt.is_single_auto_submit() {
            " submit  "
        } else {
            " confirm  "
        };
        spans.push(Span::styled(action_text, theme.style(StyleKind::Muted)));

        spans.push(Span::styled("1-9", Style::default()));
        spans.push(Span::styled(" quick  ", theme.style(StyleKind::Muted)));
        spans.push(Span::styled("Esc", Style::default()));
        spans.push(Span::styled(" dismiss", theme.style(StyleKind::Muted)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.background_element));
    frame.render_widget(paragraph, area);
}
