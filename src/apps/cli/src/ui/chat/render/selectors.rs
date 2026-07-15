impl ChatView {
fn render_command_menu(&mut self, frame: &mut Frame, area: Rect) {
        self.command_menu.render(frame, area, &self.theme);
    }

    fn render_model_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.model_selector.render(frame, area, &self.theme);
    }

    fn render_agent_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.agent_selector.render(frame, area, &self.theme);
    }

    fn render_session_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.session_selector.render(frame, area, &self.theme);
    }

    fn render_skill_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.skill_selector.render(frame, area, &self.theme);
    }

    fn render_subagent_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.subagent_selector.render(frame, area, &self.theme);
    }

    fn render_mcp_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.mcp_selector.render(frame, area, &self.theme);
    }

    fn render_mcp_add_dialog(&self, frame: &mut Frame, area: Rect) {
        self.popups.mcp_add_dialog.render(frame, area, &self.theme);
    }

    fn render_provider_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.provider_selector.render(frame, area, &self.theme);
    }

    fn render_model_config_form(&mut self, frame: &mut Frame, area: Rect) {
        super::model_config_form::render(&self.popups.model_config_form, frame, area, &self.theme);
    }

    fn render_theme_selector(&mut self, frame: &mut Frame, area: Rect) {
        self.popups.theme_selector.render(frame, area, &self.theme);
    }
}