//! Bot command router small helpers (Round 14 split).
//!
//! Owns:
//! - `normalize_im_command_text` (full-width digit normalization)
//! - `strip_numeric_reply_suffix` (numeric reply suffix stripper)
//! - `result_from_menu` / `result_from_menu_with_forward` (MenuView -> HandleResult)
//! - `refresh_assistant_name_if_missing` (one-shot workspace service lookup)
//! - `short_path_name` (path basename)

use super::command_router_dispatch::truncate_label;

use super::command_router_state::BotChatState;

use super::command_router::BotAction;

use super::menu::{MenuItem, MenuView};

use super::HandleResult;

use super::ForwardRequest;

pub(super) fn normalize_im_command_text(text: &str) -> String {
    text.trim()
        .chars()
        .map(|c| match c {
            '\u{FF10}'..='\u{FF19}' => char::from_u32(c as u32 - 0xFF10 + u32::from(b'0')).unwrap_or(c),
            c => c,
        })
        .collect()
}

pub(super) fn strip_numeric_reply_suffix(s: &str) -> &str {
    s.trim_end_matches(|c: char| {
        matches!(
            c,
            '.' | '。' | '、' | ',' | '，' | ':' | '：' | ';' | '；' | ')' | '）' | ']' | '】'
        )
    })
    .trim()
}

pub(super) fn result_from_menu(state: &mut BotChatState, view: MenuView) -> HandleResult {
    let actions: Vec<BotAction> = view.items.iter().cloned().map(BotAction::from).collect();
    state.last_menu_commands = view.numeric_commands();
    HandleResult {
        reply: view.render_text_block(),
        actions,
        forward_to_session: None,
        menu: view,
    }
}

pub(super) fn result_from_menu_with_forward(
    state: &mut BotChatState,
    view: MenuView,
    forward: Option<ForwardRequest>,
) -> HandleResult {
    let mut r = result_from_menu(state, view);
    r.forward_to_session = forward;
    r
}

pub(super) async fn refresh_assistant_name_if_missing(state: &mut BotChatState) {
    use crate::service::workspace::global_workspace_service;
    if state.current_assistant_name.is_some() {
        return;
    }
    let Some(path) = state.current_assistant.clone() else {
        return;
    };
    let Some(svc) = global_workspace_service() else {
        return;
    };
    let workspaces = svc.get_assistant_workspaces().await;
    if let Some(ws) = workspaces.into_iter().find(|w| w.root_path.to_string_lossy() == path) {
        state.current_assistant_name = Some(ws.name);
    }
}

pub(super) fn short_path_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Parse a comma-separated list of numeric question indices (e.g. "1,3,2" → [1,3,2]).
/// Returns None if the input is empty or contains any non-numeric token.
pub(super) fn parse_question_numbers(input: &str) -> Option<Vec<usize>> {
    let mut result = Vec::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value = trimmed.parse::<usize>().ok()?;
        result.push(value);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
