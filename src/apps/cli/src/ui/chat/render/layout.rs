impl ChatView {
    /// Render interface
    pub fn render(&mut self, frame: &mut Frame, chat_state: &ChatState) {
        let size = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(self.theme.background)),
            size,
        );

        // Dynamic input area height: 2 (borders) + visible content lines, capped at 8+2=10
        let max_input_content_lines: u16 = 8;
        let input_inner_width = size.width.saturating_sub(2); // subtract left+right borders
        let total_visual_lines = self.text_input.visual_line_count(input_inner_width) as u16;
        let content_lines = total_visual_lines.max(1).min(max_input_content_lines);
        let input_height = content_lines + 2; // +2 for top+bottom borders

        // Calculate shortcuts area height based on content
        let shortcuts_height = Self::calculate_shortcuts_height(size.width, chat_state, self.browse_mode);
        // Status area can grow for long status messages to avoid horizontal truncation.
        let raw_status_height =
            Self::calculate_status_height(size.width, chat_state, self.status.as_deref());
        // Keep a minimal conversation viewport while allowing status to expand when possible.
        let max_status_height = size
            .height
            .saturating_sub(3 + input_height + shortcuts_height + 3)
            .max(1);
        let status_height = raw_status_height.min(max_status_height);

        // Main layout: header + content + status bar + input + shortcuts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),            // header
                Constraint::Min(10),              // messages area
                Constraint::Length(status_height), // status bar (dynamic)
                Constraint::Length(input_height), // input area (dynamic)
                Constraint::Length(shortcuts_height), // shortcuts hint (dynamic)
            ])
            .split(size);

        // Render each part
        self.render_header(frame, chunks[0], chat_state);
        self.render_messages(frame, chunks[1], chat_state);
        self.render_status_bar(frame, chunks[2], chat_state);
        self.render_input(frame, chunks[3], chat_state);
        self.render_command_menu(frame, chunks[1]);
        self.render_model_selector(frame, chunks[1]);
        self.render_agent_selector(frame, chunks[1]);
        self.render_session_selector(frame, chunks[1]);
        self.render_skill_selector(frame, chunks[1]);
        self.render_subagent_selector(frame, chunks[1]);
        self.render_mcp_selector(frame, chunks[1]);
        self.render_mcp_add_dialog(frame, chunks[1]);
        self.render_provider_selector(frame, chunks[1]);
        self.render_model_config_form(frame, chunks[1]);
        self.render_theme_selector(frame, chunks[1]);
        self.render_shortcuts(frame, chunks[4], chat_state);

        // Render permission overlay on top of messages area if active (highest priority)
        if let Some(ref prompt) = chat_state.permission_prompt {
            render_permission_overlay(frame, prompt, &self.theme, chunks[1]);
        }
        // Render question overlay (second priority, only if no permission prompt)
        else if let Some(ref prompt) = chat_state.question_prompt {
            render_question_overlay(frame, prompt, &self.theme, chunks[1]);
        }

        // Command palette overlay (Ctrl+P)
        self.command_palette.render(frame, size, &self.theme);

        // Info popup overlay (topmost)
        if let Some(ref msg) = self.info_popup {
            super::widgets::render_info_popup(frame, size, msg, self.theme.primary);
        }
    }

    /// Calculate the required height for the shortcuts area
    fn calculate_shortcuts_height(available_width: u16, chat_state: &ChatState, browse_mode: bool) -> u16 {
        let mode_text = if browse_mode { " Browse " } else { " Chat " };
        let left_text = format!("{} | Model: {}", mode_text, chat_state.current_model_name);

        let right_text = "[Tab]Switch Agent [Alt+\u{21b5}]Newline [Ctrl+P]Commands [\u{2191}\u{2193}]History [Ctrl+E]Browse [Esc]Interrupt [Ctrl+C]Quit";

        let left_width = UnicodeWidthStr::width(left_text.as_str());
        let right_width = UnicodeWidthStr::width(right_text);

        // If both fit on one line (with at least 2 spaces gap), height is 1
        if left_width + right_width + 2 <= available_width as usize {
            1
        } else {
            // Otherwise, need 2 lines
            2
        }
    }

    fn calculate_status_height(
        available_width: u16,
        chat_state: &ChatState,
        status: Option<&str>,
    ) -> u16 {
        if chat_state.is_processing {
            return 1;
        }
        if available_width == 0 {
            return 1;
        }

        let status_text = if let Some(status_text) = status {
            format!(" {}", status_text)
        } else {
            format!(
                " Messages: {} | Tool calls: {} | Tokens: {}",
                chat_state.metadata.message_count,
                chat_state.metadata.tool_calls,
                chat_state.metadata.total_tokens,
            )
        };

        let width = available_width as usize;
        let mut lines = 0usize;
        for raw_line in status_text.lines() {
            let line_width = UnicodeWidthStr::width(raw_line);
            lines += ((line_width + width.saturating_sub(1)) / width).max(1);
        }
        if lines == 0 {
            lines = 1;
        }

        lines as u16
    }
}
