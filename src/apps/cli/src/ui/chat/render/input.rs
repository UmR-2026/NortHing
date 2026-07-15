impl ChatView {
fn render_input(&mut self, frame: &mut Frame, area: Rect, chat_state: &ChatState) {
        use super::text_input::TextInputStyle;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.style(StyleKind::Primary))
            .title(" Input ");

        let inner = block.inner(area);

        // Render the block border
        frame.render_widget(block, area);

        let style = TextInputStyle {
            first_line_prefix: "> ",
            continuation_prefix: "  ",
            placeholder: "Enter message...".to_string(),
            text_style: ratatui::style::Style::default(),
            placeholder_style: self.theme.style(StyleKind::Muted),
        };

        self.text_input.render(frame, inner, &style, !chat_state.is_processing);
    }
}