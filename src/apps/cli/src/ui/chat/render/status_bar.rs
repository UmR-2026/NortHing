impl ChatView {
/// Render status bar (between Conversation and Input)
    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect, chat_state: &ChatState) {
        if chat_state.is_processing {
            // Show thinking spinner when processing
            self.spinner.tick();
            let loading_text = format!(" {} Thinking...", self.spinner.current());
            let stats_text = format!("Tokens: {} ", chat_state.metadata.total_tokens);

            let padding_len = (area.width as usize)
                .saturating_sub(loading_text.len() + stats_text.len());

            let loading_span = Span::styled(loading_text, self.theme.style(StyleKind::Primary));
            let stats_span = Span::styled(stats_text, self.theme.style(StyleKind::Muted));

            let line = Line::from(vec![
                loading_span,
                Span::raw(" ".repeat(padding_len)),
                stats_span,
            ]);

            let paragraph = Paragraph::new(line);
            frame.render_widget(paragraph, area);
        } else {
            let status_text = if let Some(status) = &self.status {
                format!(" {}", status)
            } else {
                format!(
                    " Messages: {} | Tool calls: {} | Tokens: {}",
                    chat_state.metadata.message_count,
                    chat_state.metadata.tool_calls,
                    chat_state.metadata.total_tokens,
                )
            };

            let paragraph = Paragraph::new(status_text)
                .style(self.theme.style(StyleKind::Muted))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, area);
        }
    }
}