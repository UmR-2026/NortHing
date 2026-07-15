impl ChatView {
fn render_shortcuts(&self, frame: &mut Frame, area: Rect, chat_state: &ChatState) {
        let muted = self.theme.style(StyleKind::Muted);

        // Build left side content
        let mode_text = if self.browse_mode {
            " Browse "
        } else {
            " Chat "
        };

        // Build left text for width calculation
        let left_text = format!("{} | Model: {}", mode_text, chat_state.current_model_name);

        // Build left line with proper styling
        let left_spans = vec![
            Span::styled(mode_text, self.theme.style(StyleKind::Primary)),
            Span::styled(" | ", muted),
            Span::styled(format!("Model: {}", chat_state.current_model_name), muted),
        ];

        // Build right side shortcuts with proper styling
        let shortcuts = vec![
            ("Tab", "Switch Agent"),
            ("Alt+\u{21b5}", "Newline"),
            ("Ctrl+P", "Commands"),
            ("\u{2191}\u{2193}", "History"),
            ("Ctrl+E", "Browse"),
            ("Esc", "Interrupt"),
            ("Ctrl+C", "Quit"),
        ];

        let mut right_spans = Vec::new();
        let mut right_text = String::new();
        for (i, (key, desc)) in shortcuts.iter().enumerate() {
            if i > 0 {
                right_spans.push(Span::styled(" ", muted));
                right_text.push(' ');
            }
            let key_text = format!("[{}]", key);
            right_spans.push(Span::styled(key_text.clone(), muted));
            right_spans.push(Span::styled(*desc, muted));
            right_text.push_str(&key_text);
            right_text.push_str(desc);
        }

        // Render lines based on available width
        let available_width = area.width as usize;
        let left_line = Line::from(left_spans);
        let right_line = Line::from(right_spans);

        // Calculate widths using unicode_width
        let left_width = UnicodeWidthStr::width(left_text.as_str());
        let right_width = UnicodeWidthStr::width(right_text.as_str());

        let mut lines = Vec::new();

        if left_width + right_width + 2 <= available_width {
            // Both fit on one line: left-align left, right-align right
            let gap = available_width.saturating_sub(left_width + right_width);
            let mut combined_spans = Vec::new();
            combined_spans.extend(left_line.spans);
            combined_spans.push(Span::raw(" ".repeat(gap)));
            combined_spans.extend(right_line.spans);
            lines.push(Line::from(combined_spans));
        } else {
            // Need multiple lines: render left and right separately
            lines.push(left_line);
            lines.push(right_line);
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}