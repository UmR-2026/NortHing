//! Chat mode keyboard, non-keyboard event handling, and popup navigation helpers.
use anyhow::Result;
use crossterm::event::{KeyEvent, KeyEventKind};

use crate::chat_state::ChatState;
use crate::ui::chat::ChatView;

use super::{ChatExitReason, ChatMode};

pub(crate) mod bridge;
pub(crate) mod key_actions;
pub(crate) mod key_popups;
pub(crate) mod non_key;

impl ChatMode {
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
        if let Some(outcome) = self.handle_permission_prompt_key(key, chat_view, chat_state, rt_handle) {
            return outcome;
        }

        // ── Question prompt intercepts all keys when active ──
        if let Some(outcome) = self.handle_question_prompt_key(key, chat_view, chat_state, rt_handle) {
            return outcome;
        }

        // ── Popups (Global nav, Info, Command palette, Specific popups) ──
        if let Some(outcome) = self.handle_popup_key(key, chat_view, chat_state, rt_handle)? {
            return Ok(outcome);
        }

        // ── Normal key handling ──
        self.handle_key_action(key, chat_view, chat_state, rt_handle)
    }
}
