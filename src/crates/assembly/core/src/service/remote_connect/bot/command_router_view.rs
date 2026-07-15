//! Bot menu view builders (Round 14 split).
//!
//! Owns all `MenuView` builders:
//! - `welcome_view`, `ready_to_chat_body`, `main_menu_view`, `settings_menu_view`
//! - `need_session_view`, `confirm_mode_switch_view`
//! - `workspace_selection_view`, `assistant_selection_view`, `session_selection_view`
//! - `build_question_view`, `question_option_line`
//! - `menu_or_welcome` (paired-state router)

use super::command_router_dispatch::truncate_label;

use super::command_router_state::{BotChatState, BotDisplayMode};

use super::command_router_util::{result_from_menu, short_path_name};

use super::command_router::{BotQuestion, BotQuestionOption, HandleResult};

use super::locale::BotStrings;

use super::menu::{MenuItem, MenuView};

pub(super) fn welcome_view(s: &'static BotStrings) -> MenuView {
    MenuView::plain(s.welcome_title)
        .with_body(s.welcome)
        .with_footer(s.welcome_body)
}

pub(super) fn ready_to_chat_body(state: &BotChatState, s: &'static BotStrings) -> Option<String> {
    // Always show the workspace / assistant name (a human-meaningful
    // identifier) regardless of whether a session is active. We deliberately
    // do NOT surface `current_session_id` — the random UUID tail (e.g.
    // "5cff6a1") is opaque to the user and adds nothing useful. If the
    // user wants to manage sessions they can use /resume which renders
    // proper session names.
    if state.display_mode == BotDisplayMode::Pro {
        match &state.current_workspace {
            Some(p) => Some(format!("{}: {}", s.current_workspace_label, short_path_name(p))),
            None => Some(s.no_workspace.to_string()),
        }
    } else {
        // Assistant mode: prefer the cached assistant display name (set by
        // pairing / switch / resume flows from `WorkspaceInfo.name`). The
        // workspace path's directory name is meaningless here — the actual
        // assistant folder is usually `workspace` or `workspace-<uuid>`,
        // both of which look like noise to the user.
        match &state.current_assistant {
            Some(p) => {
                let label = state
                    .current_assistant_name
                    .as_deref()
                    .filter(|n| !n.trim().is_empty())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| short_path_name(p));
                Some(format!("{}: {}", s.current_assistant_label, label))
            }
            None => Some(s.no_assistant.to_string()),
        }
    }
}

pub(super) fn main_menu_view(state: &BotChatState, s: &'static BotStrings) -> MenuView {
    let title = if state.display_mode == BotDisplayMode::Pro {
        s.main_title_expert
    } else {
        s.main_title_assistant
    };
    let body = ready_to_chat_body(state, s);
    let mut items: Vec<MenuItem> = Vec::new();
    if state.display_mode == BotDisplayMode::Pro {
        items.push(MenuItem::primary(s.item_new_code_session, "/new_code_session"));
        items.push(MenuItem::default(s.item_new_cowork_session, "/new_cowork_session"));
        items.push(MenuItem::default(s.item_resume_session, "/resume"));
        items.push(MenuItem::default(s.item_switch_workspace, "/switch"));
    } else {
        items.push(MenuItem::primary(s.item_new_session, "/new"));
        items.push(MenuItem::default(s.item_resume_session, "/resume"));
        items.push(MenuItem::default(s.item_switch_assistant, "/switch"));
    }
    items.push(MenuItem::default(s.item_settings, "/settings"));
    let mut view = MenuView::plain(title).with_items(items);
    if let Some(b) = body {
        view = view.with_body(b);
    }
    view
}

pub(super) fn settings_menu_view(verbose: bool, state: &BotChatState, s: &'static BotStrings) -> MenuView {
    let mut items: Vec<MenuItem> = Vec::new();
    if state.display_mode == BotDisplayMode::Pro {
        items.push(MenuItem::default(s.item_switch_to_assistant, "/assistant"));
    } else {
        items.push(MenuItem::default(s.item_switch_to_expert, "/expert"));
    }
    if verbose {
        items.push(MenuItem::default(s.item_verbose_off, "/concise"));
    } else {
        items.push(MenuItem::default(s.item_verbose_on, "/verbose"));
    }
    items.push(MenuItem::default(s.item_help, "/help"));
    items.push(MenuItem::default(s.item_back, "/menu"));
    let body = format!(
        "{} · {}: {}",
        if state.display_mode == BotDisplayMode::Pro {
            s.mode_expert
        } else {
            s.mode_assistant
        },
        s.verbose_label,
        if verbose {
            s.verbose_status_on
        } else {
            s.verbose_status_off
        },
    );
    MenuView::plain(s.settings_title).with_body(body).with_items(items)
}

pub(super) fn need_session_view(state: &BotChatState, s: &'static BotStrings) -> MenuView {
    let mut items = Vec::new();
    if state.display_mode == BotDisplayMode::Pro {
        items.push(MenuItem::primary(s.item_new_code_session, "/new_code_session"));
        items.push(MenuItem::default(s.item_new_cowork_session, "/new_cowork_session"));
    } else {
        items.push(MenuItem::primary(s.item_new_session, "/new"));
    }
    items.push(MenuItem::default(s.item_resume_session, "/resume"));
    items.push(MenuItem::default(s.item_back, "/menu"));
    MenuView::plain(s.need_session_title).with_items(items)
}

pub(super) fn confirm_mode_switch_view(target_mode: BotDisplayMode, s: &'static BotStrings) -> MenuView {
    let target_label = if target_mode == BotDisplayMode::Pro {
        s.mode_expert
    } else {
        s.mode_assistant
    };
    let body = format!("{} → {}", s.mode_confirm_switch_prefix, target_label);
    MenuView::plain(s.settings_title).with_body(body).with_items(vec![
        MenuItem::primary(s.item_confirm_switch, "1"),
        MenuItem::default(s.item_back, "/menu"),
    ])
}

pub(super) fn menu_or_welcome(state: &mut BotChatState, s: &'static BotStrings) -> HandleResult {
    if state.paired {
        result_from_menu(state, main_menu_view(state, s))
    } else {
        result_from_menu(state, welcome_view(s))
    }
}

pub(super) fn workspace_selection_view(
    state: &BotChatState,
    options: &[(String, String)],
    s: &'static BotStrings,
) -> MenuView {
    let mut items = Vec::new();
    let mut body = String::new();
    for (i, (path, name)) in options.iter().enumerate() {
        let is_current = state.current_workspace.as_deref() == Some(path.as_str());
        let marker = if is_current { s.current_marker } else { "" };
        body.push_str(&format!("{}. {}{}\n", i + 1, name, marker));
        items.push(MenuItem::default(truncate_label(name, 24), (i + 1).to_string()));
    }
    items.push(MenuItem::default(s.item_back, "/menu"));
    MenuView::plain(s.switch_pick_workspace)
        .with_body(body.trim_end().to_string())
        .with_items(items)
        .with_footer(s.footer_reply_workspace)
        .without_plain_text_items()
}

pub(super) fn assistant_selection_view(
    state: &BotChatState,
    options: &[(String, String)],
    s: &'static BotStrings,
) -> MenuView {
    let mut items = Vec::new();
    let mut body = String::new();
    for (i, (path, name)) in options.iter().enumerate() {
        let is_current = state.current_assistant.as_deref() == Some(path.as_str());
        let marker = if is_current { s.current_marker } else { "" };
        body.push_str(&format!("{}. {}{}\n", i + 1, name, marker));
        items.push(MenuItem::default(truncate_label(name, 24), (i + 1).to_string()));
    }
    items.push(MenuItem::default(s.item_back, "/menu"));
    MenuView::plain(s.switch_pick_assistant)
        .with_body(body.trim_end().to_string())
        .with_items(items)
        .with_footer(s.footer_reply_assistant)
        .without_plain_text_items()
}

pub(super) fn session_selection_view(
    state: &BotChatState,
    options: &[(String, String)],
    page: usize,
    has_more: bool,
    s: &'static BotStrings,
) -> MenuView {
    let mut items = Vec::new();
    let mut body = String::new();
    for (i, (id, name)) in options.iter().enumerate() {
        let is_current = state.current_session_id.as_deref() == Some(id.as_str());
        let marker = if is_current { s.current_marker } else { "" };
        body.push_str(&format!("{}. {}{}\n", i + 1, name, marker));
        items.push(MenuItem::default(truncate_label(name, 26), (i + 1).to_string()));
    }
    if has_more {
        items.push(MenuItem::default(s.item_next_page, "0"));
    }
    items.push(MenuItem::default(s.item_back, "/menu"));
    let footer = if has_more {
        s.footer_reply_session_or_next
    } else {
        s.footer_reply_session
    };
    MenuView::plain(format!("{} · #{}", s.resume_page_label, page + 1))
        .with_body(body.trim_end().to_string())
        .with_items(items)
        .with_footer(footer)
        .without_plain_text_items()
}

pub(super) fn question_option_line(index: usize, option: &BotQuestionOption) -> String {
    if option.description.is_empty() {
        format!("{}. {}", index + 1, option.label)
    } else {
        format!("{}. {} - {}", index + 1, option.label, option.description)
    }
}

pub(super) fn build_question_view(
    s: &'static BotStrings,
    questions: &[BotQuestion],
    current_index: usize,
    awaiting_custom_text: bool,
) -> MenuView {
    let question = &questions[current_index];
    let title = format!("{} {}/{}", s.question_title, current_index + 1, questions.len());

    let mut body = String::new();
    if !question.header.is_empty() {
        body.push_str(&question.header);
        body.push('\n');
    }
    body.push_str(&question.question);
    body.push_str("\n\n");
    for (idx, option) in question.options.iter().enumerate() {
        body.push_str(&question_option_line(idx, option));
        body.push('\n');
    }
    body.push_str(&format!("{}. {}\n", question.options.len() + 1, s.item_other,));

    let footer = if awaiting_custom_text {
        s.footer_question_custom
    } else if question.multi_select {
        s.footer_question_multi
    } else {
        s.footer_question_single
    };

    let mut items: Vec<MenuItem> = Vec::new();
    if !awaiting_custom_text && !question.multi_select {
        for (idx, option) in question.options.iter().enumerate() {
            items.push(MenuItem::default(
                truncate_label(&option.label, 24),
                (idx + 1).to_string(),
            ));
        }
        items.push(MenuItem::default(
            s.item_other,
            (question.options.len() + 1).to_string(),
        ));
    }
    items.push(MenuItem::default(s.item_back, "/menu"));

    MenuView::plain(title)
        .with_body(body.trim_end().to_string())
        .with_items(items)
        .with_footer(footer)
}
