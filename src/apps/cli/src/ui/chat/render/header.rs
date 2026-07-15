impl ChatView {
    /// Render header
    fn render_header(&self, frame: &mut Frame, area: Rect, chat_state: &ChatState) {
        let title = format!(" northhing CLI v{} ", env!("CARGO_PKG_VERSION"));
        let agent_info = format!(" Agent: {} ", chat_state.agent_type);

        let workspace = chat_state
            .workspace
            .as_ref()
            .map(|w| format!("Workspace: {}", w))
            .unwrap_or_else(|| "No workspace".to_string());

        let header = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.style(StyleKind::Border))
            .style(Style::default().bg(self.theme.background));

        let title_style = Style::default()
            .fg(self.theme.primary)
            .add_modifier(Modifier::BOLD);

        let text = vec![Line::from(vec![
            Span::styled(&title, title_style),
            Span::raw("  "),
            Span::styled(&agent_info, self.theme.style(StyleKind::Primary)),
            Span::raw("  "),
            Span::styled(&workspace, self.theme.style(StyleKind::Muted)),
        ])];

        let paragraph = Paragraph::new(text)
            .block(header)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }
}
