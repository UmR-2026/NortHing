//! Popup navigation and popup event interception for chat mode.
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::agent::Agent;
use crate::chat_state::ChatState;
use crate::ui::chat::{ChatView, PopupType};
use crate::ui::command_palette::PaletteAction;
use crate::ui::mcp_add_dialog::McpAddAction;
use crate::ui::model_config_form::ModelFormAction;
use crate::ui::permission::PermissionAction;
use crate::ui::question::QuestionAction;
use crate::ui::session_selector::SessionAction;

use super::super::{ChatExitReason, ChatMode};
use super::bridge::bridge;

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

    /// Intercept key event when a permission prompt is active.
    pub(super) fn handle_permission_prompt_key(
        &mut self,
        key: KeyEvent,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Option<Result<Option<ChatExitReason>>> {
        let prompt = chat_state.permission_prompt.as_mut()?;
        let action = prompt.handle_key_event(key);
        match action {
            PermissionAction::AllowOnce => {
                let tool_id = prompt.tool_id.clone();
                let agent = self.agent.clone();
                chat_state.permission_prompt = None;
                tracing::info!("User allowed tool once: {}", tool_id);
                bridge(rt_handle, async move {
                    if let Err(e) = agent.confirm_tool(&tool_id, None).await {
                        tracing::error!("Failed to confirm tool: {}", e);
                    }
                });
                chat_view.set_status(Some("Tool confirmed".to_string()));
            }
            PermissionAction::AllowAlways => {
                let tool_id = prompt.tool_id.clone();
                let agent = self.agent.clone();
                chat_state.permission_prompt = None;
                tracing::info!("User allowed tool always: {}", tool_id);
                bridge(rt_handle, async move {
                    if let Err(e) = agent.confirm_tool(&tool_id, None).await {
                        tracing::error!("Failed to confirm tool: {}", e);
                    }
                    // Skip all future tool confirmations
                    if let Ok(svc) = northhing_core::service::config::get_global_config_service().await {
                        if let Err(e) = svc.set_config("ai.skip_tool_confirmation", true).await {
                            tracing::warn!("Failed to set skip_tool_confirmation: {}", e);
                        }
                    }
                });
                chat_view.set_status(Some("Tool confirmed (always)".to_string()));
            }
            PermissionAction::Reject(reason) => {
                let tool_id = prompt.tool_id.clone();
                let agent = self.agent.clone();
                chat_state.permission_prompt = None;
                tracing::info!("User rejected tool: {}, reason: {}", tool_id, reason);
                let reason_clone = reason.clone();
                bridge(rt_handle, async move {
                    if let Err(e) = agent.reject_tool(&tool_id, reason_clone).await {
                        tracing::error!("Failed to reject tool: {}", e);
                    }
                });
                chat_view.set_status(Some(format!("Tool rejected: {}", reason)));
            }
            PermissionAction::None => {
                // Permission prompt consumed the key, no further action
            }
        }
        Some(Ok(None))
    }

    /// Intercept key event when a question prompt is active.
    pub(super) fn handle_question_prompt_key(
        &mut self,
        key: KeyEvent,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Option<Result<Option<ChatExitReason>>> {
        let prompt = chat_state.question_prompt.as_mut()?;
        let action = prompt.handle_key_event(key);
        match action {
            QuestionAction::Submit(answers) => {
                let tool_id = prompt.tool_id.clone();
                let agent = self.agent.clone();
                chat_state.question_prompt = None;
                tracing::info!("User submitted answers for tool: {}", tool_id);
                bridge(rt_handle, async move {
                    if let Err(e) = agent.submit_user_answers(&tool_id, answers).await {
                        tracing::error!("Failed to submit answers: {}", e);
                    }
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
        Some(Ok(None))
    }

    /// Intercept key event when global popups, info popup, command palette, or specific popups are active.
    pub(super) fn handle_popup_key(
        &mut self,
        key: KeyEvent,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<Option<Option<ChatExitReason>>> {
        // Global popup navigation: Ctrl+W closes all popups, Esc navigates back
        if self.any_popup_visible(chat_view) {
            match (key.code, key.modifiers) {
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                    self.close_all_popups(chat_view);
                    return Ok(Some(None));
                }
                (KeyCode::Esc, _) => {
                    self.navigate_back(chat_view);
                    return Ok(Some(None));
                }
                _ => {}
            }
        }

        // Info popup intercepts all keys when visible
        if chat_view.info_popup_visible() {
            chat_view.dismiss_info_popup();
            return Ok(Some(None));
        }

        // Command palette intercepts all keys when visible
        if chat_view.command_palette_visible() {
            let action = chat_view.command_palette_handle_key(key);
            match action {
                PaletteAction::Execute(id) => {
                    let result = self.handle_palette_action(&id, chat_view, chat_state, rt_handle)?;
                    return Ok(Some(result));
                }
                PaletteAction::Dismiss | PaletteAction::None => {}
            }
            return Ok(Some(None));
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
            return Ok(Some(None));
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
            return Ok(Some(None));
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
            return Ok(Some(None));
        }

        if chat_view.session_selector_visible() {
            let action = chat_view.session_selector_handle_key(key);
            match action {
                SessionAction::Switch(item) => {
                    return Ok(Some(Some(ChatExitReason::SwitchSession(item.session_id))));
                }
                SessionAction::Delete(item) => {
                    self.handle_session_delete(&item, chat_view, chat_state, rt_handle);
                }
                SessionAction::Close | SessionAction::None => {}
            }
            return Ok(Some(None));
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
            return Ok(Some(None));
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
            return Ok(Some(None));
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
            return Ok(Some(None));
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
            return Ok(Some(None));
        }

        if chat_view.provider_selector_visible() {
            if let Some(selection) = chat_view.provider_selector_handle_key(key) {
                self.handle_provider_selection(selection, chat_view);
            }
            return Ok(Some(None));
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
            return Ok(Some(None));
        }

        Ok(None)
    }
}
