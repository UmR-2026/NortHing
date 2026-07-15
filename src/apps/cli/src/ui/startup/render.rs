use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use super::super::text_input::TextInputStyle;
use super::super::widgets;

use super::StartupPage;

impl StartupPage {
    pub(super) fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(self.theme.background)), size);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // main content
                Constraint::Length(1), // bottom bar
            ])
            .split(size);

        let main_area = chunks[0];
        let input_area = self.render_main(frame, main_area);
        self.render_bottom_bar(frame, chunks[1]);

        // Overlay: command menu (above input area)
        if self.command_menu.is_visible() {
            let menu_area = Rect {
                x: input_area.x,
                y: main_area.y,
                width: input_area.width,
                height: input_area.y.saturating_sub(main_area.y),
            };
            self.command_menu.render(frame, menu_area, &self.theme);
        }

        // Overlay: selector popups (centered on full screen)
        self.model_selector.render(frame, size, &self.theme);
        self.agent_selector.render(frame, size, &self.theme);
        self.session_selector.render(frame, size, &self.theme);
        self.skill_selector.render(frame, size, &self.theme);
        self.subagent_selector.render(frame, size, &self.theme);
        self.theme_selector.render(frame, size, &self.theme);
        self.provider_selector.render(frame, size, &self.theme);
        super::super::model_config_form::render_mut(&mut self.model_config_form, frame, size, &self.theme);

        // Overlay: command palette (Ctrl+P)
        self.command_palette.render(frame, size, &self.theme);

        // Overlay: info popup (highest priority)
        if let Some(ref msg) = self.info_popup {
            widgets::render_info_popup(frame, size, msg, self.theme.primary);
        }
    }

    /// Render main content, returns the input box area (for command menu positioning)
    fn render_main(&mut self, frame: &mut Frame, area: Rect) -> Rect {
        let max_width = 75u16.min(area.width.saturating_sub(4));

        // Dynamic input height: content lines (1..6) + 2 (padding top + agent label row) + 1 (gap)
        let input_content_width = max_width.saturating_sub(2 + 4); // left bar(2) + inner padding(4)
        let visual_lines = self.text_input.visual_line_count_with_prefix(input_content_width, 0) as u16;
        let content_lines = visual_lines.max(1).min(6);
        let input_box_height = content_lines + 3; // +1 top padding, +1 gap, +1 agent label

        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),           // top space
                Constraint::Length(12),               // logo
                Constraint::Length(1),                // gap
                Constraint::Length(input_box_height), // input box
                Constraint::Length(2),                // gap + tip/status
                Constraint::Min(1),                   // bottom space
            ])
            .split(area);

        // Logo
        self.render_logo(frame, v_chunks[1]);

        // Input box - centered horizontally
        let h_pad = area.width.saturating_sub(max_width) / 2;
        let input_area = Rect {
            x: area.x + h_pad,
            y: v_chunks[3].y,
            width: max_width,
            height: v_chunks[3].height,
        };
        self.render_input(frame, input_area);

        // Tip / status
        let tip_area = Rect {
            x: area.x + h_pad,
            y: v_chunks[4].y + 1,
            width: max_width,
            height: 1,
        };
        self.render_tip_or_status(frame, tip_area);

        input_area
    }

    fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        let highlight_color = self.theme.primary;
        let input_bg = self.input_background();

        // Split: 2 cols for left bar, rest for content
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2), // left bar
                Constraint::Min(1),    // content
            ])
            .split(area);

        // Left bar: full-height ┃
        let bar_lines: Vec<Line> = (0..area.height)
            .map(|_| Line::from(Span::styled(" ┃", Style::default().fg(highlight_color).bg(input_bg))))
            .collect();
        let bar = Paragraph::new(bar_lines).style(Style::default().bg(input_bg));
        frame.render_widget(bar, h_chunks[0]);

        // Content area with background
        let content_area = h_chunks[1];

        // Fill background
        let bg =
            Paragraph::new(vec![Line::from(""); content_area.height as usize]).style(Style::default().bg(input_bg));
        frame.render_widget(bg, content_area);

        // Inner content with padding
        let inner = Rect {
            x: content_area.x + 2,
            y: content_area.y + 1,
            width: content_area.width.saturating_sub(4),
            height: content_area.height.saturating_sub(1),
        };

        // Calculate how many lines are available for text input
        // Reserve 2 lines at the bottom: 1 gap + 1 agent label
        let text_height = inner.height.saturating_sub(2).max(1);
        let text_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: text_height,
        };

        // Render text input using shared TextInput component
        let style = TextInputStyle {
            first_line_prefix: "",
            continuation_prefix: "",
            placeholder: "Ask anything... or type / for commands".to_string(),
            text_style: Style::default().fg(self.theme.command_text).bg(input_bg),
            placeholder_style: Style::default().fg(self.theme.muted).bg(input_bg),
        };
        self.text_input.render(frame, text_area, &style, true);

        // Agent label + model name below input (with 1 line gap)
        if inner.height >= 3 {
            let mut spans = vec![Span::styled(&self.agent_type, Style::default().fg(highlight_color))];
            if !self.model_display_name.is_empty() {
                spans.push(Span::styled(" | ", Style::default().fg(self.theme.muted)));
                spans.push(Span::styled(
                    &self.model_display_name,
                    Style::default().fg(self.theme.muted),
                ));
            }
            let agent_line = Line::from(spans);
            let agent_area = Rect {
                x: inner.x,
                y: inner.y + text_height + 1,
                width: inner.width,
                height: 1,
            };
            frame.render_widget(Paragraph::new(agent_line), agent_area);
        }
    }

    fn input_background(&self) -> ratatui::style::Color {
        self.theme.input_background
    }

    fn render_tip_or_status(&self, frame: &mut Frame, area: Rect) {
        let line = if let Some(ref status) = self.status {
            Line::from(vec![
                Span::styled("● ", Style::default().fg(self.theme.success)),
                Span::styled(status.as_str(), Style::default().fg(self.theme.muted)),
            ])
        } else {
            Line::from(vec![
                Span::styled("● ", Style::default().fg(self.theme.warning)),
                Span::styled("Tip ", Style::default().fg(self.theme.warning)),
                Span::styled(self.tip, Style::default().fg(self.theme.muted)),
            ])
        };
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_bottom_bar(&self, frame: &mut Frame, area: Rect) {
        let version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let mcp_status = crate::get_mcp_status_text();

        // Determine MCP status color
        let mcp_color = if mcp_status.contains("Ready") {
            self.theme.success
        } else if mcp_status.contains("Failed") {
            self.theme.error
        } else {
            self.theme.warning
        };

        // Left: workspace path
        let left = Paragraph::new(Line::from(Span::styled(
            format!("  {}", self.workspace_display),
            Style::default().fg(self.theme.muted),
        )));
        frame.render_widget(left, area);

        // Right: MCP status | version
        let right = Paragraph::new(Line::from(vec![
            Span::styled(&mcp_status, Style::default().fg(mcp_color)),
            Span::styled(format!(" | {}  ", version), Style::default().fg(self.theme.muted)),
        ]))
        .alignment(Alignment::Right);
        frame.render_widget(right, area);
    }

    fn render_logo(&self, frame: &mut Frame, area: Rect) {
        let use_fancy_logo = area.width >= 80;
        let mut lines = vec![];
        lines.push(Line::from(""));

        if use_fancy_logo {
            let logo = vec![
                "  ██████╗ ██╗████████╗███████╗██╗   ██╗███╗   ██╗",
                "  ██╔══██╗██║╚══██╔══╝██╔════╝██║   ██║████╗  ██║",
                "  ██████╔╝██║   ██║   █████╗  ██║   ██║██╔██╗ ██║",
                "  ██╔══██╗██║   ██║   ██╔══╝  ██║   ██║██║╚██╗██║",
                "  ██████╔╝██║   ██║   ██║     ╚██████╔╝██║ ╚████║",
                "  ╚═════╝ ╚═╝   ╚═╝   ╚═╝      ╚═════╝ ╚═╝  ╚═══╝",
            ];

            let colors = [
                self.theme.primary,
                self.theme.info,
                self.theme.success,
                self.theme.warning,
                self.theme.error,
                self.theme.muted,
            ];

            for (i, line) in logo.iter().enumerate() {
                lines.push(Line::from(Span::styled(
                    *line,
                    Style::default()
                        .fg(colors[i % colors.len()])
                        .add_modifier(Modifier::BOLD),
                )));
            }
        } else {
            let logo = vec![
                "  ____  _ _   _____            ",
                " | __ )(_) |_|  ___|   _ _ __  ",
                " |  _ \\| | __| |_ | | | | '_ \\ ",
                " | |_) | | |_|  _|| |_| | | | |",
                " |____/|_|\\__|_|   \\__,_|_| |_|",
            ];

            let colors = [
                self.theme.primary,
                self.theme.info,
                self.theme.success,
                self.theme.warning,
                self.theme.error,
            ];

            for (i, line) in logo.iter().enumerate() {
                lines.push(Line::from(Span::styled(
                    *line,
                    Style::default()
                        .fg(colors[i % colors.len()])
                        .add_modifier(Modifier::BOLD),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "AI agent-driven command-line programming assistant",
            Style::default().fg(self.theme.muted).add_modifier(Modifier::ITALIC),
        )));

        let version = format!("v{}", env!("CARGO_PKG_VERSION"));
        lines.push(Line::from(Span::styled(version, Style::default().fg(self.theme.muted))));

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}
