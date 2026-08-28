//! Normal keyboard action arms and key bindings for chat mode.
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::agent::Agent;
use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;

use super::super::{ChatExitReason, ChatMode};
use super::bridge::bridge;

impl ChatMode {
    /// Handle normal key bindings when no prompt or popup intercepts the key.
    pub(super) fn handle_key_action(
        &mut self,
        key: KeyEvent,
        chat_view: &mut ChatView,
        chat_state: &mut ChatState,
        rt_handle: &tokio::runtime::Handle,
    ) -> Result<Option<ChatExitReason>> {
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
                    bridge(rt_handle, async move {
                        if let Err(e) = agent.cancel_current_turn().await {
                            tracing::error!("Failed to cancel turn: {}", e);
                        }
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
                    let display_name = super::super::agent_display_name(&self.agent_type);
                    chat_view.set_status(Some(format!("{} is thinking...", display_name)));

                    let agent = self.agent.clone();
                    let input_clone = input.clone();
                    let agent_type = self.agent_type.clone();
                    match bridge(rt_handle, agent.send_message(input_clone, &agent_type)) {
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
                    bridge(rt_handle, async move {
                        if let Err(e) = agent.cancel_current_turn().await {
                            tracing::error!("Failed to cancel turn: {}", e);
                        }
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
}
