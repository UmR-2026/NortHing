//! Bot session-resume dispatcher (Round 15 split).
//!
//! Owns:
//! - `start_resume` (god method, ~126 lines) — paginates the persisted
//!   session list and sets up `PendingAction::SelectSession` for either
//!   a page advance (`page + 1`) or a per-session pick.
//!
//! Split from `command_router_dispatch.rs` in Round 15 to keep the
//! dispatch file under the 800-line cap. R14 left dispatch at 842 lines
//! (4% over the 800-line cap), so `start_resume` was extracted to this
//! sibling. Only callers are `dispatch` (ResumeSession arm) and
//! `route_pending` (SelectSession handler, when the user picks `0` for
//! the next page).
//!
//! `truncate_label` stays defined in `command_router_dispatch` and is
//! shared via a `pub(super)` cross-sibling import here — the same pattern
//! `command_router_util` and `command_router_view` already use for it.

use super::command_router::HandleResult;
use super::command_router_dispatch::truncate_label;
use super::command_router_state::{BotChatState, BotDisplayMode, PendingAction};
use super::command_router_util::result_from_menu;
use super::command_router_view::need_session_view;
use super::locale::{fmt_count, BotStrings};
use super::menu::{MenuItem, MenuView};

pub(super) async fn start_resume(state: &mut BotChatState, page: usize, s: &'static BotStrings) -> HandleResult {
    use crate::agentic::persistence::PersistenceManager;
    use crate::infrastructure::PathManager;

    let ws_path = if state.display_mode == BotDisplayMode::Pro {
        match &state.current_workspace {
            Some(p) => std::path::PathBuf::from(p),
            None => {
                return result_from_menu(
                    state,
                    MenuView::plain(s.no_workspace).with_items(vec![
                        MenuItem::primary(s.item_switch_workspace, "/switch"),
                        MenuItem::default(s.item_back, "/menu"),
                    ]),
                );
            }
        }
    } else {
        match &state.current_assistant {
            Some(p) => std::path::PathBuf::from(p),
            None => {
                return result_from_menu(
                    state,
                    MenuView::plain(s.no_assistant).with_items(vec![
                        MenuItem::primary(s.item_switch_assistant, "/switch"),
                        MenuItem::default(s.item_back, "/menu"),
                    ]),
                );
            }
        }
    };

    let page_size = 10usize;
    let offset = page * page_size;

    let pm = match PathManager::new() {
        Ok(pm) => std::sync::Arc::new(pm),
        Err(e) => {
            return result_from_menu(state, MenuView::plain(format!("{}{e}", s.session_create_failed_prefix)));
        }
    };
    let store = match PersistenceManager::new(pm) {
        Ok(store) => store,
        Err(e) => {
            return result_from_menu(state, MenuView::plain(format!("{}{e}", s.session_create_failed_prefix)));
        }
    };
    let all_meta = match store.list_session_metadata(&ws_path).await {
        Ok(m) => m,
        Err(e) => {
            return result_from_menu(state, MenuView::plain(format!("{}{e}", s.session_create_failed_prefix)));
        }
    };

    if all_meta.is_empty() {
        return result_from_menu(state, need_session_view(state, s));
    }

    let total = all_meta.len();
    let has_more = offset + page_size < total;
    let sessions: Vec<_> = all_meta.into_iter().skip(offset).take(page_size).collect();

    let mut body = String::new();
    let mut items = Vec::new();
    let mut options = Vec::new();
    for (i, sess) in sessions.iter().enumerate() {
        let is_current = state.current_session_id.as_deref() == Some(&sess.session_id);
        let marker = if is_current { s.current_marker } else { "" };
        let ts = chrono::DateTime::from_timestamp(sess.last_active_at as i64 / 1000, 0)
            .map(|dt| dt.format("%m-%d %H:%M").to_string())
            .unwrap_or_default();
        let msg_hint = match sess.turn_count {
            0 => s.resume_msg_count_zero.to_string(),
            1 => s.resume_msg_count_one.to_string(),
            n => fmt_count(s.resume_msg_count_many_fmt, n),
        };
        body.push_str(&format!(
            "{}. [{}] {}{}\n   {} 路 {}\n",
            i + 1,
            sess.agent_type,
            sess.session_name,
            marker,
            ts,
            msg_hint,
        ));
        items.push(MenuItem::default(
            truncate_label(&format!("[{}] {}", sess.agent_type, sess.session_name), 26),
            (i + 1).to_string(),
        ));
        options.push((sess.session_id.clone(), sess.session_name.clone()));
    }
    if has_more {
        items.push(MenuItem::default(s.item_next_page, "0"));
    }
    items.push(MenuItem::default(s.item_back, "/menu"));

    state.set_pending(PendingAction::SelectSession {
        options,
        page,
        has_more,
    });

    let footer = if has_more {
        s.footer_reply_session_or_next
    } else {
        s.footer_reply_session
    };
    let view = MenuView::plain(format!("{} 路 #{}", s.resume_page_label, page + 1))
        .with_body(body.trim_end().to_string())
        .with_items(items)
        .with_footer(footer);
    result_from_menu(state, view)
}
