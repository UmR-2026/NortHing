//! Chat mode keyboard, non-keyboard event handling, and popup navigation helpers.
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

use crate::chat_state::ChatState;
use crate::ui::chat::{ChatView, MouseGestureOutcome, PopupType};

use crate::agent::Agent;
use crate::ui::command_palette::PaletteAction;
use crate::ui::mcp_add_dialog::McpAddAction;
use crate::ui::model_config_form::ModelFormAction;
use crate::ui::permission::PermissionAction;
use crate::ui::question::QuestionAction;
use crate::ui::session_selector::SessionAction;

use super::{ChatExitReason, ChatMode, NonKeyEventOutcome};

impl ChatMode {
    /// Check if any popup is currently visible
    pub(crate) fn any_popup_visible(&self, chat_view: &ChatView) -> bool {
        chat_view.command_palette_visible()
            || chat_view.model_selector_visible()
            || chat_view.agent_selector_visible()
            || chat_view.session_selector_visible()
            || chat_view.skill_selector_visible()
            || chat_view.subagent_selector_visible()
            || chat_view.mcp_selector_visible()
            || chat_view.mcp_add_dialog_visible()
            || chat_view.provider_selector_visible()
            || chat_view.model_config_form_visible()
            || chat_view.theme_selector_visible()
            || chat_view.info_popup_visible()
    }

    /// Close all popups and clear the navigation stack
    pub(crate) fn close_all_popups(&self, chat_view: &mut ChatView) {
        // Cancel theme preview if active
        if chat_view.theme_selector_visible() {
            chat_view.cancel_theme_preview();
        }
        chat_view.hide_command_palette();
        chat_view.hide_model_selector();
        chat_view.hide_agent_selector();
        chat_view.hide_session_selector();
        chat_view.hide_skill_selector();
        chat_view.hide_subagent_selector();
        chat_view.hide_mcp_selector();
        chat_view.hide_mcp_add_dialog();
        chat_view.hide_provider_selector();
        chat_view.hide_model_config_form();
        chat_view.hide_theme_selector();
        chat_view.dismiss_info_popup();
        chat_view.popups.popup_stack.clear();
    }

    /// Navigate back to the previous popup in the stack, or close all if at the root
    pub(crate) fn navigate_back(&self, chat_view: &mut ChatView) {
        // Pop the current popup from the stack and hide it
        if let Some(current) = chat_view.popups.popup_stack.pop() {
            // Hide the current popup
            match current {
                PopupType::CommandPalette => chat_view.hide_command_palette(),
                PopupType::ModelSelector => chat_view.hide_model_selector(),
                PopupType::AgentSelector => chat_view.hide_agent_selector(),
                PopupType::SessionSelector => chat_view.hide_session_selector(),
                PopupType::SkillSelector => chat_view.hide_skill_selector(),
                PopupType::SubagentSelector => chat_view.hide_subagent_selector(),
                PopupType::McpSelector => chat_view.hide_mcp_selector(),
                PopupType::McpAddDialog => chat_view.hide_mcp_add_dialog(),
                PopupType::ProviderSelector => chat_view.hide_provider_selector(),
                PopupType::ModelConfigForm => chat_view.hide_model_config_form(),
                PopupType::ThemeSelector => {
                    chat_view.hide_theme_selector();
                    chat_view.cancel_theme_preview();
                }
                PopupType::InfoPopup => chat_view.dismiss_info_popup(),
            }

            // If there's a previous popup in the stack, re-show it
            if let Some(previous) = chat_view.popups.popup_stack.peek() {
                match previous {
                    PopupType::CommandPalette => chat_view.reshow_command_palette(),
                    PopupType::ModelSelector => chat_view.reshow_model_selector(),
                    PopupType::AgentSelector => chat_view.reshow_agent_selector(),
                    PopupType::SessionSelector => chat_view.reshow_session_selector(),
                    PopupType::SkillSelector => chat_view.reshow_skill_selector(),
                    PopupType::SubagentSelector => chat_view.reshow_subagent_selector(),
                    PopupType::McpSelector => chat_view.reshow_mcp_selector(),
                    PopupType::McpAddDialog => chat_view.reshow_mcp_add_dialog(),
                    PopupType::ProviderSelector => chat_view.reshow_provider_selector(),
                    PopupType::ModelConfigForm => chat_view.reshow_model_config_form(),
                    PopupType::ThemeSelector => chat_view.reshow_theme_selector(),
                    PopupType::InfoPopup => {}
                }
            }
        }
    }

    /// Handle keyboard events — extracted from the original ChatMode::run loop.
    pub(crate) fn handle_key_event(
        &mut self,
        key: KeyEvent,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<Option<ChatExitReason>> {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return Ok(None);
        }

        // ── Permission prompt intercepts all keys when active ──
        if let Some(ref mut prompt) = chat_state.permission_prompt {
            let action = prompt.handle_key_event(key);
            match action {
                PermissionAction::AllowOnce => {
                    let tool_id = prompt.tool_id.clone();
                    let agent = self.agent.clone();
                    chat_state.permission_prompt = None;
                    tracing::info!("User allowed tool once: {}", tool_id);
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.confirm_tool(&tool_id, None).await {
                                tracing::error!("Failed to confirm tool: {}", e);
                            }
                        })
                    });
                    chat_view.set_status(Some("Tool confirmed".to_string()));
                }
                PermissionAction::AllowAlways => {
                    let tool_id = prompt.tool_id.clone();
                    let agent = self.agent.clone();
                    chat_state.permission_prompt = None;
                    tracing::info!("User allowed tool always: {}", tool_id);
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.confirm_tool(&tool_id, None).await {
                                tracing::error!("Failed to confirm tool: {}", e);
                            }
                            // Skip all future tool confirmations
                            if let Ok(svc) = northhing_core::service::config::get_global_config_service().await {
                                if let Err(e) = svc.set_config("ai.skip_tool_confirmation", true).await {
                                    tracing::warn!("Failed to set skip_tool_confirmation: {}", e);
                                }
                            }
                        })
                    });
                    chat_view.set_status(Some("Tool confirmed (always)".to_string()));
                }
                PermissionAction::Reject(reason) => {
                    let tool_id = prompt.tool_id.clone();
                    let agent = self.agent.clone();
                    chat_state.permission_prompt = None;
                    tracing::info!("User rejected tool: {}, reason: {}", tool_id, reason);
                    let reason_clone = reason.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.reject_tool(&tool_id, reason_clone).await {
                                tracing::error!("Failed to reject tool: {}", e);
                            }
                        })
                    });
                    chat_view.set_status(Some(format!("Tool rejected: {}", reason)));
                }
                PermissionAction::None => {
                    // Permission prompt consumed the key, no further action
                }
            }
            return Ok(None);
        }

        // ── Question prompt intercepts all keys when active ──
        if let Some(ref mut prompt) = chat_state.question_prompt {
            let action = prompt.handle_key_event(key);
            match action {
                QuestionAction::Submit(answers) => {
                    let tool_id = prompt.tool_id.clone();
                    let agent = self.agent.clone();
                    chat_state.question_prompt = None;
                    tracing::info!("User submitted answers for tool: {}", tool_id);
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.submit_user_answers(&tool_id, answers).await {
                                tracing::error!("Failed to submit answers: {}", e);
                            }
                        })
                    });
                    chat_view.set_status(Some("Answers submitted".to_string()));
                }
                QuestionAction::Reject => {
                    let tool_id = prompt.tool_id.clone();
                    chat_state.question_prompt = None;
                    tracing::info!("User dismissed question prompt: {}", tool_id);
                    chat_view.set_status(Some("Question dismissed".to_string()));
                }
                QuestionAction::None => {
                    // Question prompt consumed the key, no further action
                }
            }
            return Ok(None);
        }

        // ── Normal key handling ──

        // Global popup navigation: Ctrl+W closes all popups, Esc navigates back
        if self.any_popup_visible(chat_view) {
            match (key.code, key.modifiers) {
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                    self.close_all_popups(chat_view);
                    return Ok(None);
                }
                (KeyCode::Esc, _) => {
                    self.navigate_back(chat_view);
                    return Ok(None);
                }
                _ => {}
            }
        }

        // Info popup intercepts all keys when visible
        if chat_view.info_popup_visible() {
            chat_view.dismiss_info_popup();
            return Ok(None);
        }

        // Command palette intercepts all keys when visible
        if chat_view.command_palette_visible() {
            let action = chat_view.command_palette_handle_key(key);
            match action {
                PaletteAction::Execute(id) => {
                    return self.handle_palette_action(&id, chat_view, chat_state, rt_handle);
                }
                PaletteAction::Dismiss | PaletteAction::None => {}
            }
            return Ok(None);
        }

        // Handle popup events first (when visible)
        if chat_view.model_selector_visible() {
            match key.code {
                KeyCode::Up => chat_view.model_selector_up(),
                KeyCode::Down => chat_view.model_selector_down(),
                KeyCode::Enter => {
                    if let Some(selected) = chat_view.model_selector_confirm() {
                        chat_view.hide_model_selector();
                        self.apply_model_selection(&selected, chat_view, chat_state, rt_handle);
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(selected) = chat_view.model_selector_confirm() {
                        chat_view.hide_model_selector();
                        self.edit_model(&selected, chat_view, rt_handle);
                    }
                }
                // Note: Esc is handled globally for navigation back
                _ => {}
            }
            return Ok(None);
        }

        if chat_view.theme_selector_visible() {
            match key.code {
                KeyCode::Up => {
                    chat_view.theme_selector_up();
                    if let Some(selected) = chat_view.theme_selector_selected() {
                        self.preview_theme_selection(&selected, chat_view);
                    }
                }
                KeyCode::Down => {
                    chat_view.theme_selector_down();
                    if let Some(selected) = chat_view.theme_selector_selected() {
                        self.preview_theme_selection(&selected, chat_view);
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected) = chat_view.theme_selector_confirm() {
                        chat_view.hide_theme_selector();
                        self.apply_theme_selection(&selected, chat_view);
                        chat_view.commit_theme_preview();
                    }
                }
                // Note: Esc is handled globally for navigation back
                _ => {}
            }
            return Ok(None);
        }

        if chat_view.agent_selector_visible() {
            match key.code {
                KeyCode::Up => chat_view.agent_selector_up(),
                KeyCode::Down => chat_view.agent_selector_down(),
                KeyCode::Enter => {
                    if let Some(selected) = chat_view.agent_selector_confirm() {
                        chat_view.hide_agent_selector();
                        self.apply_agent_selection(&selected, chat_state);
                    }
                }
                // Note: Esc is handled globally for navigation back
                _ => {}
            }
            return Ok(None);
        }

        if chat_view.session_selector_visible() {
            let action = chat_view.session_selector_handle_key(key);
            match action {
                SessionAction::Switch(item) => {
                    return Ok(Some(ChatExitReason::SwitchSession(item.session_id)));
                }
                SessionAction::Delete(item) => {
                    self.handle_session_delete(&item, chat_view, chat_state, rt_handle);
                }
                SessionAction::Close | SessionAction::None => {}
            }
            return Ok(None);
        }

        if chat_view.skill_selector_visible() {
            match key.code {
                KeyCode::Up => chat_view.skill_selector_up(),
                KeyCode::Down => chat_view.skill_selector_down(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(action) = chat_view.skill_selector_confirm() {
                        self.handle_skill_selector_action(action, chat_view, chat_state, rt_handle);
                    }
                }
                // Note: Esc is handled globally for navigation back
                _ => {}
            }
            return Ok(None);
        }

        if chat_view.subagent_selector_visible() {
            match key.code {
                KeyCode::Up => chat_view.subagent_selector_up(),
                KeyCode::Down => chat_view.subagent_selector_down(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(action) = chat_view.subagent_selector_confirm() {
                        self.handle_subagent_selector_action(action, chat_view, chat_state, rt_handle);
                    }
                }
                // Note: Esc is handled globally for navigation back
                _ => {}
            }
            return Ok(None);
        }

        if chat_view.mcp_selector_visible() {
            match key.code {
                KeyCode::Up => chat_view.mcp_selector_up(),
                KeyCode::Down => chat_view.mcp_selector_down(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(selected) = chat_view.mcp_selector_confirm() {
                        self.toggle_mcp_server(&selected.id, chat_view);
                    }
                }
                KeyCode::Char('a') => {
                    // Open add dialog (hide selector first)
                    chat_view.hide_mcp_selector();
                    chat_view.show_mcp_add_dialog();
                }
                KeyCode::Char('d') => {
                    if let Some(selected) = chat_view.mcp_selector_confirm() {
                        // First press: enter confirm-delete mode
                        // Second press: actually delete (handled by confirm_delete state)
                        if chat_view.mcp_selector_is_confirm_delete(&selected.id) {
                            self.delete_mcp_server(&selected.id, chat_view);
                        } else {
                            chat_view.mcp_selector_start_confirm_delete(selected.id.clone());
                        }
                    }
                }
                KeyCode::Char('e') => {
                    chat_view.hide_mcp_selector();
                    self.open_mcp_config(chat_state);
                }
                // Note: Esc is handled globally for navigation back
                _ => {
                    // Any other key cancels the confirm-delete state
                    chat_view.mcp_selector_cancel_confirm_delete();
                }
            }
            return Ok(None);
        }

        if chat_view.mcp_add_dialog_visible() {
            let action = chat_view.mcp_add_dialog_handle_key(key);
            match action {
                McpAddAction::Confirm { name, config_json } => {
                    self.add_mcp_server(&name, &config_json, chat_view);
                }
                McpAddAction::Cancel => {
                    // Re-open the MCP selector
                    self.show_mcp_selector(chat_view, chat_state, rt_handle);
                }
                McpAddAction::None => {}
            }
            return Ok(None);
        }

        if chat_view.provider_selector_visible() {
            if let Some(selection) = chat_view.provider_selector_handle_key(key) {
                self.handle_provider_selection(selection, chat_view);
            }
            return Ok(None);
        }

        if chat_view.model_config_form_visible() {
            let action = chat_view.model_config_form_handle_key(key);
            match action {
                ModelFormAction::Save(result) => {
                    if result.editing_model_id.is_some() {
                        self.update_existing_model(result, chat_view, chat_state, rt_handle);
                    } else {
                        self.save_new_model(result, chat_view, chat_state, rt_handle);
                    }
                }
                ModelFormAction::Cancel => {
                    chat_view.set_status(Some("Model form cancelled".to_string()));
                }
                ModelFormAction::None => {}
            }
            return Ok(None);
        }

        match (key.code, key.modifiers) {
            // Ctrl+V: read clipboard directly (reliable paste on Windows where
            // bracketed paste is broken — crossterm issue #962)
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => match Clipboard::new().and_then(|mut cb| cb.get_text()) {
                Ok(text) if !text.is_empty() => {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    for c in normalized.chars() {
                        chat_view.handle_char(c);
                    }
                }
                _ => {}
            },

            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                // If processing, cancel the current turn instead of quitting
                if chat_state.is_processing {
                    tracing::info!("User requested cancellation");
                    let agent = self.agent.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.cancel_current_turn().await {
                                tracing::error!("Failed to cancel turn: {}", e);
                            }
                        })
                    });
                    chat_view.set_status(Some("Cancelling...".to_string()));
                    return Ok(None);
                }
                tracing::info!("User requested quit");
                return Ok(Some(ChatExitReason::Quit));
            }

            (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                chat_view.show_command_palette();
                return Ok(None);
            }

            // Alt+Enter: insert newline in input
            (KeyCode::Enter, m) if m.contains(KeyModifiers::ALT) => {
                chat_view.handle_newline();
            }

            (KeyCode::Enter, _) => {
                if let Some(cmd) = chat_view.apply_command_menu_selection() {
                    let cmd_result = self.handle_command(&cmd, chat_view, chat_state, rt_handle)?;
                    return Ok(cmd_result);
                }

                if chat_state.is_processing {
                    let trimmed = chat_view.input_text().trim();
                    if trimmed.starts_with('/') {
                        if let Some(input) = chat_view.send_input() {
                            let cmd_result = self.handle_command(&input, chat_view, chat_state, rt_handle)?;
                            return Ok(cmd_result);
                        }
                    } else if !trimmed.is_empty() {
                        chat_view.set_status(Some(
                            "Currently processing. Type a /command, or press Ctrl+C to cancel.".to_string(),
                        ));
                    }
                    return Ok(None);
                }

                if let Some(input) = chat_view.send_input() {
                    tracing::info!("User input: {}", input);

                    if input.starts_with('/') {
                        let cmd_result = self.handle_command(&input, chat_view, chat_state, rt_handle)?;
                        return Ok(cmd_result);
                    }

                    // Send message to agent
                    let display_name = super::agent_display_name(&self.agent_type);
                    chat_view.set_status(Some(format!("{} is thinking...", display_name)));

                    let agent = self.agent.clone();
                    let input_clone = input.clone();
                    let agent_type = self.agent_type.clone();
                    match tokio::task::block_in_place(|| {
                        rt_handle.block_on(agent.send_message(input_clone, &agent_type))
                    }) {
                        Ok(turn_id) => {
                            tracing::info!("Started turn: {}", turn_id);
                        }
                        Err(e) => {
                            tracing::error!("Failed to send message: {}", e);
                            chat_view.set_status(Some(format!("Error: {}", e)));
                        }
                    }
                }
            }

            (KeyCode::Backspace, _) => {
                chat_view.handle_backspace();
            }

            (KeyCode::Left, _) => {
                chat_view.move_cursor_left();
            }
            (KeyCode::Right, _) => {
                chat_view.move_cursor_right();
            }

            // Ctrl+O: toggle expand/collapse on focused block tool
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                chat_view.toggle_focused_tool_expand(chat_state);
            }

            // Ctrl+J: focus previous block tool (up)
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                chat_view.cycle_block_tool_focus_prev(chat_state);
            }

            // Ctrl+K: focus next block tool (down)
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                chat_view.cycle_block_tool_focus_next(chat_state);
            }

            // ↑↓: input history only. Conversation scrolling stays on PageUp/PageDown or mouse.
            (KeyCode::Up, KeyModifiers::NONE) => {
                if chat_view.command_menu_visible() {
                    chat_view.command_menu_up();
                } else {
                    chat_view.history_prev();
                }
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if chat_view.command_menu_visible() {
                    chat_view.command_menu_down();
                } else {
                    chat_view.history_next();
                }
            }

            (KeyCode::Home, KeyModifiers::CONTROL) => {
                let total = chat_view.count_message_lines(chat_state);
                chat_view.scroll_to_top(total);
                chat_view.set_status(Some("Jumped to conversation top".to_string()));
            }

            (KeyCode::End, KeyModifiers::CONTROL) => {
                chat_view.scroll_to_bottom();
                chat_view.set_status(Some("Jumped to conversation bottom".to_string()));
            }

            (KeyCode::Home, _) => {
                chat_view.set_cursor_home();
            }

            (KeyCode::End, _) => {
                chat_view.set_cursor_end();
            }

            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                chat_view.clear_input();
            }

            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                chat_view.toggle_browse_mode();
                let status_msg = if chat_view.browse_mode {
                    "Entered browse mode, use PageUp/PageDown or mouse wheel to scroll conversation"
                } else {
                    "Exited browse mode"
                };
                chat_view.set_status(Some(status_msg.to_string()));
            }

            (KeyCode::PageUp, _) => {
                let total = chat_view.count_message_lines(chat_state);
                chat_view.scroll_up(10, total);
            }

            (KeyCode::PageDown, _) => {
                chat_view.scroll_down(10);
            }

            (KeyCode::Esc, _) => {
                if chat_state.is_processing {
                    tracing::info!("User requested cancellation (Esc)");
                    let agent = self.agent.clone();
                    tokio::task::block_in_place(|| {
                        rt_handle.block_on(async move {
                            if let Err(e) = agent.cancel_current_turn().await {
                                tracing::error!("Failed to cancel turn: {}", e);
                            }
                        })
                    });
                    chat_view.set_status(Some("Cancelling...".to_string()));
                    return Ok(None);
                }
                if chat_view.browse_mode {
                    chat_view.scroll_to_bottom();
                    chat_view.set_status(Some("Exited browse mode".to_string()));
                }
            }

            (KeyCode::Tab, _) => {
                if !chat_state.is_processing {
                    self.cycle_agent(chat_view, chat_state, rt_handle);
                }
            }

            (KeyCode::BackTab, _) => {
                if !chat_state.is_processing {
                    self.cycle_agent_reverse(chat_view, chat_state, rt_handle);
                }
            }

            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if !c.is_control() && c != '\u{0}' {
                    chat_view.handle_char(c);
                }
            }

            _ => {}
        }

        Ok(None)
    }

    /// Apply an exit reason from handle_key_event (shared by normal and batch paths).
    pub(crate) fn apply_exit_reason(
        reason: ChatExitReason,
        this: &mut Self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        session_id: &mut String,
        rt_handle: &tokio::runtime::Handle,
        should_quit: &mut bool,
        exit_reason: &mut ChatExitReason,
    ) {
        match reason {
            ChatExitReason::SwitchSession(new_session_id) => {
                match this.switch_to_session(&new_session_id, session_id, chat_state, chat_view, rt_handle) {
                    Ok(()) => tracing::info!("Switched to session: {}", new_session_id),
                    Err(e) => {
                        chat_state.add_system_message(format!("Failed to switch session: {}", e));
                        tracing::error!("Failed to switch session: {}", e);
                    }
                }
            }
            ChatExitReason::NewSession => match this.create_new_session(session_id, chat_state, chat_view, rt_handle) {
                Ok(()) => tracing::info!("Created new session: {}", session_id),
                Err(e) => {
                    chat_state.add_system_message(format!("Failed to create new session: {}", e));
                    tracing::error!("Failed to create new session: {}", e);
                }
            },
            other => {
                *should_quit = true;
                *exit_reason = other;
            }
        }
    }

    /// Handle non-key events (Mouse, Paste, Resize, etc.).
    pub(super) fn handle_non_key_event(
        event: Event,
        this: &mut Self,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        session_id: &mut String,
        rt_handle: &tokio::runtime::Handle,
        should_quit: &mut bool,
        exit_reason: &mut ChatExitReason,
    ) -> Result<NonKeyEventOutcome> {
        let mut outcome = NonKeyEventOutcome::default();
        match event {
            Event::Mouse(mouse) => {
                if chat_view.command_palette_captures_mouse(&mouse) {
                    let action = chat_view.command_palette_handle_mouse(&mouse);
                    match action {
                        PaletteAction::Execute(id) => {
                            if let Some(reason) = this.handle_palette_action(&id, chat_view, chat_state, rt_handle)? {
                                Self::apply_exit_reason(
                                    reason,
                                    this,
                                    chat_view,
                                    chat_state,
                                    session_id,
                                    rt_handle,
                                    should_quit,
                                    exit_reason,
                                );
                            }
                        }
                        PaletteAction::Dismiss | PaletteAction::None => {}
                    }
                } else if chat_view.provider_selector_captures_mouse(&mouse) {
                    if let Some(selection) = chat_view.provider_selector_handle_mouse(&mouse) {
                        this.handle_provider_selection(selection, chat_view);
                    }
                } else if chat_view.handle_mouse_event(&mouse) {
                    if let Some(action) = chat_view.take_pending_skill_action() {
                        this.handle_skill_selector_action(action, chat_view, chat_state, rt_handle);
                    }
                    if let Some(action) = chat_view.take_pending_subagent_action() {
                        this.handle_subagent_selector_action(action, chat_view, chat_state, rt_handle);
                    }
                } else {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            let total = chat_view.count_message_lines(chat_state);
                            chat_view.scroll_up(3, total);
                        }
                        MouseEventKind::ScrollDown => {
                            chat_view.scroll_down(3);
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let _ = chat_view.begin_mouse_selection(mouse.column, mouse.row);
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let _ = chat_view.update_mouse_selection(mouse.column, mouse.row);
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            match chat_view.complete_mouse_selection_or_click(mouse.column, mouse.row) {
                                MouseGestureOutcome::CopyText(text) => {
                                    match Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
                                        Ok(()) => chat_view.set_status(Some("Copied to clipboard".to_string())),
                                        Err(_) => chat_view.set_status(Some("Failed to copy selection".to_string())),
                                    }
                                }
                                MouseGestureOutcome::Click(col, row) => {
                                    chat_view.handle_mouse_click(col, row);
                                }
                                MouseGestureOutcome::None => {}
                            }
                        }
                        MouseEventKind::Moved => {
                            if !chat_view.update_mouse_selection(mouse.column, mouse.row) {
                                chat_view.handle_mouse_move(mouse.column, mouse.row);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(cmd) = chat_view.take_pending_command() {
                    if let Some(reason) = this.handle_command(&cmd, chat_view, chat_state, rt_handle)? {
                        Self::apply_exit_reason(
                            reason,
                            this,
                            chat_view,
                            chat_state,
                            session_id,
                            rt_handle,
                            should_quit,
                            exit_reason,
                        );
                    }
                }
                if let Some(theme) = chat_view.take_pending_theme_preview() {
                    this.preview_theme_selection(&theme, chat_view);
                }
                if let Some(server_id) = chat_view.take_pending_mcp_toggle() {
                    this.toggle_mcp_server(&server_id, chat_view);
                }
                outcome.request_redraw = true;
            }
            Event::Paste(text) => {
                if chat_view.mcp_add_dialog_visible() {
                    chat_view.mcp_add_dialog_handle_paste(&text);
                } else {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    for c in normalized.chars() {
                        chat_view.handle_char(c);
                    }
                }
                outcome.request_redraw = true;
            }
            Event::Resize(_, _) => {
                outcome.resize_seen = true;
            }
            _ => {}
        }
        Ok(outcome)
    }
}
