//! Non-keyboard event handling (Mouse, Paste, Resize) and exit reason application.
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{Event, MouseButton, MouseEventKind};

use crate::chat_state::ChatState;
use crate::ui::chat::{ChatView, MouseGestureOutcome};
use crate::ui::command_palette::PaletteAction;

use super::super::{ChatExitReason, ChatMode, NonKeyEventOutcome};

impl ChatMode {
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
    pub(in crate::modes::chat) fn handle_non_key_event(
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
