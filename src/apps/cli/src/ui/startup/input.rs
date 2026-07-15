use super::super::command_palette::PaletteAction;
use super::super::model_config_form::ModelFormAction;
use super::super::session_selector::SessionAction;

use crate::commands::STARTUP_COMMAND_SPECS;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::time::Duration;

use super::PopupType;
use super::StartupPage;
use super::KEYBOARD_SHORTCUTS_HELP;

impl StartupPage {
    pub fn run<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<super::StartupResult>
    where
        <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
    {
        terminal.clear()?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Ok(first_event) = event::read() {
                    let mut events = vec![first_event];
                    // Short wait to let rapid paste events arrive in the same batch.
                    // Duration::ZERO would split pastes across loop iterations.
                    while event::poll(Duration::from_millis(5))? {
                        if let Ok(ev) = event::read() {
                            events.push(ev);
                        } else {
                            break;
                        }
                    }

                    // Paste detection: multiple key events with Enter + printable chars
                    let key_count = events
                        .iter()
                        .filter(|e| matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat))
                        .count();
                    let has_enter = events.iter().any(|e| {
                        matches!(e, Event::Key(k) if (k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat) && k.code == KeyCode::Enter)
                    });
                    let has_printable = events.iter().any(|e| {
                        matches!(e, Event::Key(k) if (k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat) && matches!(k.code, KeyCode::Char(_)))
                    });
                    let is_paste_batch = key_count > 1 && has_enter && has_printable;

                    if is_paste_batch {
                        let mut paste_buf = String::new();
                        let mut non_key_events = Vec::new();
                        for ev in events {
                            match ev {
                                Event::Key(k) if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat => {
                                    match k.code {
                                        KeyCode::Char(c) => paste_buf.push(c),
                                        KeyCode::Enter => paste_buf.push('\n'),
                                        _ => {}
                                    }
                                }
                                other => non_key_events.push(other),
                            }
                        }
                        if !paste_buf.is_empty() {
                            self.text_input.insert_paste(&paste_buf);
                            self.refresh_command_menu();
                        }
                        for ev in non_key_events {
                            self.handle_non_key_event(ev, terminal)?;
                        }
                    } else {
                        for ev in events {
                            match ev {
                                Event::Key(key)
                                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
                                {
                                    if let Some(result) = self.handle_key(key) {
                                        return Ok(result);
                                    }
                                }
                                other => {
                                    self.handle_non_key_event(other, terminal)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_non_key_event<B: Backend>(&mut self, ev: Event, terminal: &mut Terminal<B>) -> Result<()> {
        match ev {
            Event::Mouse(mouse) => {
                if self.command_palette.captures_mouse(&mouse) {
                    let action = self.command_palette.handle_mouse_event(&mouse);
                    if let PaletteAction::Execute(id) = action {
                        let _ = self.handle_palette_action(&id);
                    }
                } else if self.theme_selector.captures_mouse(&mouse) {
                    self.theme_selector.handle_mouse_event(&mouse);
                    if let Some(selected) = self.theme_selector.selected_item().cloned() {
                        self.preview_theme_selection(&selected);
                    }
                } else if self.provider_selector.captures_mouse(&mouse) {
                    if let Some(selection) = self.provider_selector.handle_mouse_event(&mouse) {
                        self.handle_provider_selection(selection);
                    }
                }
            }
            Event::Paste(text) => {
                self.text_input.insert_paste(&text);
                self.refresh_command_menu();
            }
            Event::Resize(_, _) => {
                // Avoid full-screen clear on every resize event to reduce flicker.
                let _ = terminal;
            }
            _ => {}
        }
        Ok(())
    }

    // ======================== Input handling ========================

    fn handle_key(&mut self, key: KeyEvent) -> Option<super::StartupResult> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        // Clear transient status on any key press
        self.status = None;

        // ── Info popup intercepts all keys ──
        if self.info_popup.is_some() {
            self.info_popup = None;
            return None;
        }

        // ── Global popup navigation: Ctrl+W closes all popups ──
        if self.any_popup_visible() {
            match (key.code, key.modifiers) {
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                    self.close_all_popups();
                    return None;
                }
                _ => {}
            }
        }

        // ── Selector popups intercept all keys when active ──

        if self.theme_selector.is_visible() {
            match key.code {
                KeyCode::Up => {
                    self.theme_selector.move_up();
                    if let Some(selected) = self.theme_selector.selected_item().cloned() {
                        self.preview_theme_selection(&selected);
                    }
                }
                KeyCode::Down => {
                    self.theme_selector.move_down();
                    if let Some(selected) = self.theme_selector.selected_item().cloned() {
                        self.preview_theme_selection(&selected);
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected) = self.theme_selector.confirm_selection() {
                        self.theme_selector.hide();
                        self.apply_theme_selection(&selected);
                    }
                }
                KeyCode::Esc => self.navigate_back(),
                _ => {}
            }
            return None;
        }

        if self.model_selector.is_visible() {
            match key.code {
                KeyCode::Up => self.model_selector.move_up(),
                KeyCode::Down => self.model_selector.move_down(),
                KeyCode::Enter => {
                    if let Some(selected) = self.model_selector.confirm_selection() {
                        self.model_selector.hide();
                        self.apply_model_selection(&selected);
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(selected) = self.model_selector.confirm_selection() {
                        self.model_selector.hide();
                        self.edit_model(&selected);
                    }
                }
                KeyCode::Esc => self.navigate_back(),
                _ => {}
            }
            return None;
        }

        if self.agent_selector.is_visible() {
            match key.code {
                KeyCode::Up => self.agent_selector.move_up(),
                KeyCode::Down => self.agent_selector.move_down(),
                KeyCode::Enter => {
                    if let Some(selected) = self.agent_selector.confirm_selection() {
                        self.agent_selector.hide();
                        self.apply_agent_selection(&selected);
                    }
                }
                KeyCode::Esc => self.navigate_back(),
                _ => {}
            }
            return None;
        }

        if self.session_selector.is_visible() {
            let action = self.session_selector.handle_key_event(key);
            match action {
                SessionAction::Switch(item) => {
                    return Some(super::StartupResult::ContinueSession(item.session_id));
                }
                SessionAction::Delete(item) => {
                    self.handle_session_delete(&item);
                }
                SessionAction::Close => {
                    self.navigate_back();
                }
                SessionAction::None => {}
            }
            return None;
        }

        if self.skill_selector.is_visible() {
            match key.code {
                KeyCode::Up => self.skill_selector.move_up(),
                KeyCode::Down => self.skill_selector.move_down(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(action) = self.skill_selector.confirm_selection() {
                        self.handle_skill_selector_action(action);
                    }
                }
                KeyCode::Esc => self.navigate_back(),
                _ => {}
            }
            return None;
        }

        if self.subagent_selector.is_visible() {
            match key.code {
                KeyCode::Up => self.subagent_selector.move_up(),
                KeyCode::Down => self.subagent_selector.move_down(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(action) = self.subagent_selector.confirm_selection() {
                        self.handle_subagent_selector_action(action);
                    }
                }
                KeyCode::Esc => self.navigate_back(),
                _ => {}
            }
            return None;
        }

        if self.provider_selector.is_visible() {
            if let Some(selection) = self.provider_selector.handle_key_event(key) {
                self.handle_provider_selection(selection);
            }
            return None;
        }

        if self.model_config_form.is_visible() {
            let action = self.model_config_form.handle_key_event(key);
            match action {
                ModelFormAction::Save(result) => {
                    if result.editing_model_id.is_some() {
                        self.update_existing_model(result);
                    } else {
                        self.save_new_model(result);
                    }
                }
                ModelFormAction::Cancel => {
                    self.navigate_back();
                    self.status = Some("Model form cancelled".to_string());
                }
                ModelFormAction::None => {}
            }
            return None;
        }

        // ── Command palette intercepts all keys when visible ──

        if self.command_palette.is_visible() {
            let action = self.command_palette.handle_key_event(key);
            match action {
                PaletteAction::Execute(id) => {
                    return self.handle_palette_action(&id);
                }
                PaletteAction::Dismiss => {
                    self.navigate_back();
                }
                PaletteAction::None => {}
            }
            return None;
        }

        // ── Command menu navigation ──

        if self.command_menu.is_visible() {
            match key.code {
                KeyCode::Up => {
                    self.command_menu.move_up();
                    return None;
                }
                KeyCode::Down => {
                    self.command_menu.move_down();
                    return None;
                }
                KeyCode::Enter => {
                    if let Some(cmd) = self.command_menu.apply_selection() {
                        return self.handle_command(&cmd);
                    }
                    return None;
                }
                KeyCode::Esc => {
                    self.text_input.clear();
                    self.command_menu.update_with_commands("", 0, STARTUP_COMMAND_SPECS);
                    return None;
                }
                _ => {
                    // Fall through to normal input handling, which updates the menu
                }
            }
        }

        // ── Normal key handling ──

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                return Some(super::StartupResult::Exit);
            }
            (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.push_current_popup_to_stack();
                self.command_palette.show();
                return None;
            }
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        self.text_input.insert_paste(&text);
                        self.refresh_command_menu();
                    }
                }
            }
            (KeyCode::Enter, m) if m.contains(KeyModifiers::ALT) => {
                self.text_input.handle_newline();
                self.refresh_command_menu();
            }
            (KeyCode::Enter, _) => {
                if let Some(cmd) = self.command_menu.apply_selection() {
                    return self.handle_command(&cmd);
                }

                if self.text_input.is_empty() {
                    return Some(super::StartupResult::NewSession { prompt: None });
                }
                let trimmed = self.text_input.text().trim().to_string();
                if trimmed == "/exit" || trimmed == "exit" || trimmed == "quit" {
                    return Some(super::StartupResult::Exit);
                }
                if trimmed.starts_with('/') {
                    return self.handle_command(&trimmed);
                }
                return Some(super::StartupResult::NewSession { prompt: Some(trimmed) });
            }
            (KeyCode::Esc, _) => {
                if !self.text_input.is_empty() {
                    self.text_input.clear();
                    self.refresh_command_menu();
                }
            }
            (KeyCode::Tab, _) => {
                self.cycle_agent(1);
            }
            (KeyCode::BackTab, _) => {
                self.cycle_agent(-1);
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                if !self.text_input.move_cursor_up() {
                    self.text_input.set_cursor_home();
                }
                self.refresh_command_menu();
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if !self.text_input.move_cursor_down() {
                    self.text_input.set_cursor_end();
                }
                self.refresh_command_menu();
            }
            (KeyCode::Char(c), _) => {
                self.text_input.handle_char(c);
                self.refresh_command_menu();
            }
            (KeyCode::Backspace, _) => {
                self.text_input.handle_backspace();
                self.refresh_command_menu();
            }
            (KeyCode::Delete, _) => {
                self.text_input.handle_delete();
                self.refresh_command_menu();
            }
            (KeyCode::Left, _) => {
                self.text_input.move_cursor_left();
            }
            (KeyCode::Right, _) => {
                self.text_input.move_cursor_right();
            }
            (KeyCode::Home, _) => {
                self.text_input.set_cursor_home();
            }
            (KeyCode::End, _) => {
                self.text_input.set_cursor_end();
            }
            _ => {}
        }
        None
    }

    // ======================== Palette action execution ========================

    fn handle_palette_action(&mut self, action_id: &str) -> Option<super::StartupResult> {
        match action_id {
            // Session group
            "new_session" => {
                return Some(super::StartupResult::NewSession { prompt: None });
            }
            "sessions" => {
                self.show_session_selector();
            }
            "usage" => {
                self.status = Some("No active session for /usage.".to_string());
            }
            // Prompt group
            "skills" => {
                self.show_skill_selector();
            }
            "subagents" => {
                self.show_subagent_selector();
            }
            // Models group
            "select_model" => {
                self.show_model_selector();
            }
            "add_model" => {
                self.push_current_popup_to_stack();
                self.provider_selector.show();
            }
            // Appearance group
            "theme" => {
                self.show_theme_selector();
            }
            // Agent group
            "switch_agent" => {
                self.show_agent_selector();
            }
            // MCP group
            "mcp_servers" => {
                return Some(super::StartupResult::NewSession {
                    prompt: Some("/mcps".to_string()),
                });
            }
            // System group
            "help" => {
                self.info_popup = Some(KEYBOARD_SHORTCUTS_HELP.to_string());
            }
            "exit" => {
                return Some(super::StartupResult::Exit);
            }
            _ => {
                self.status = Some(format!("Unknown palette action: {}", action_id));
            }
        }
        None
    }

    // ======================== Command execution ========================

    fn handle_command(&mut self, command: &str) -> Option<super::StartupResult> {
        let cmd = command.split_whitespace().next().unwrap_or("");

        self.text_input.clear();
        self.refresh_command_menu();

        match cmd {
            "/help" => {
                self.info_popup = Some(KEYBOARD_SHORTCUTS_HELP.to_string());
            }
            "/exit" => {
                return Some(super::StartupResult::Exit);
            }
            "/sessions" => {
                self.show_session_selector();
            }
            "/models" => {
                self.show_model_selector();
            }
            "/theme" => {
                self.show_theme_selector();
            }
            "/connect" => {
                self.push_current_popup_to_stack();
                self.provider_selector.show();
            }
            "/agents" => {
                self.show_agent_selector();
            }
            "/skills" => {
                self.show_skill_selector();
            }
            "/subagents" => {
                self.show_subagent_selector();
            }
            "/mcps" => {
                // Enter chat mode and auto-trigger /mcps command
                return Some(super::StartupResult::NewSession {
                    prompt: Some("/mcps".to_string()),
                });
            }
            "/acp" => {
                return Some(super::StartupResult::NewSession {
                    prompt: Some("/acp".to_string()),
                });
            }
            "/usage" => {
                self.status = Some("No active session for /usage.".to_string());
            }
            "/init" => match crate::prompts::get_cli_prompt("init") {
                Some(prompt) => {
                    return Some(super::StartupResult::NewSession {
                        prompt: Some(prompt.to_string()),
                    });
                }
                None => {
                    self.status = Some("Init prompt not found".to_string());
                }
            },
            _ => {
                self.status = Some(format!("Unknown command: {}. Type /help for available commands.", cmd));
            }
        }

        None
    }

    // ======================== Helpers ========================

    /// Push the currently visible popup onto the navigation stack and hide it
    pub(super) fn push_current_popup_to_stack(&mut self) {
        if self.command_palette.is_visible() {
            self.popup_stack.push(PopupType::CommandPalette);
            self.command_palette.hide();
        } else if self.model_selector.is_visible() {
            self.popup_stack.push(PopupType::ModelSelector);
            self.model_selector.hide();
        } else if self.agent_selector.is_visible() {
            self.popup_stack.push(PopupType::AgentSelector);
            self.agent_selector.hide();
        } else if self.session_selector.is_visible() {
            self.popup_stack.push(PopupType::SessionSelector);
            self.session_selector.hide();
        } else if self.skill_selector.is_visible() {
            self.popup_stack.push(PopupType::SkillSelector);
            self.skill_selector.hide();
        } else if self.subagent_selector.is_visible() {
            self.popup_stack.push(PopupType::SubagentSelector);
            self.subagent_selector.hide();
        } else if self.theme_selector.is_visible() {
            self.popup_stack.push(PopupType::ThemeSelector);
            self.theme_selector.hide();
        } else if self.provider_selector.is_visible() {
            self.popup_stack.push(PopupType::ProviderSelector);
            self.provider_selector.hide();
        } else if self.model_config_form.is_visible() {
            self.popup_stack.push(PopupType::ModelConfigForm);
            self.model_config_form.hide();
        }
    }

    /// Navigate back to the previous popup in the stack, or close current if at the root
    pub(super) fn navigate_back(&mut self) {
        // First hide the currently visible popup
        if self.command_palette.is_visible() {
            self.command_palette.hide();
        } else if self.model_selector.is_visible() {
            self.model_selector.hide();
        } else if self.agent_selector.is_visible() {
            self.agent_selector.hide();
        } else if self.session_selector.is_visible() {
            self.session_selector.hide();
        } else if self.skill_selector.is_visible() {
            self.skill_selector.hide();
        } else if self.subagent_selector.is_visible() {
            self.subagent_selector.hide();
        } else if self.theme_selector.is_visible() {
            self.theme_selector.hide();
            self.cancel_theme_preview();
        } else if self.provider_selector.is_visible() {
            self.provider_selector.hide();
        } else if self.model_config_form.is_visible() {
            self.model_config_form.hide();
        }

        // If there's a previous popup in the stack, re-show it
        if let Some(previous) = self.popup_stack.pop() {
            match previous {
                PopupType::CommandPalette => self.command_palette.reshow(),
                PopupType::ModelSelector => self.model_selector.reshow(),
                PopupType::AgentSelector => self.agent_selector.reshow(),
                PopupType::SessionSelector => self.session_selector.reshow(),
                PopupType::SkillSelector => self.skill_selector.reshow(),
                PopupType::SubagentSelector => self.subagent_selector.reshow(),
                PopupType::ThemeSelector => self.theme_selector.reshow(),
                PopupType::ProviderSelector => self.provider_selector.reshow(),
                PopupType::ModelConfigForm => self.model_config_form.reshow(),
            }
        }
    }

    /// Close all popups and clear the navigation stack
    pub(super) fn close_all_popups(&mut self) {
        self.command_palette.hide();
        self.model_selector.hide();
        self.agent_selector.hide();
        self.session_selector.hide();
        self.skill_selector.hide();
        self.subagent_selector.hide();
        self.theme_selector.hide();
        self.cancel_theme_preview();
        self.provider_selector.hide();
        self.model_config_form.hide();
        self.popup_stack.clear();
    }

    pub(super) fn refresh_command_menu(&mut self) {
        self.command_menu
            .update_with_commands(&self.text_input.input, self.text_input.cursor, STARTUP_COMMAND_SPECS);
    }

    pub(super) fn set_input(&mut self, text: &str) {
        self.text_input.set_text(text);
        self.refresh_command_menu();
    }
}
