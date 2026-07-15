//! Bot command dispatchers (Round 14 split, Round 15 trim).
//!
//! Owns 17 dispatchers and sub-routines:
//! - `dispatch` (god method — splits into 3 phase helpers)
//! - `switch_mode`, `confirm_then_run`, `set_verbose`
//! - `start_switch`, `select_workspace`, `select_assistant`
//! - `truncate_label`
//! - `select_session`
//! - `new_session_for_mode`, `guarded_new`
//! - `handle_cancel_task`, `handle_number`
//! - `route_pending` (god method), `pending_invalid`
//! - `handle_chat`
//! - `truncate_at_char_boundary`
//!
//! `handle_question_reply` and `submit_question_answers` live in
//! `command_router_questions` (Round 14 split for 800-line cap).
//! `start_resume` lives in `command_router_resume` (Round 15 split —
//! closed the R14 D-deviation by extracting its ~126 lines).

use super::command_router_state::{BotChatState, BotDisplayMode, PendingAction, PENDING_INVALID_LIMIT};

use super::command_router_view::{
    assistant_selection_view, build_question_view, confirm_mode_switch_view, main_menu_view, menu_or_welcome,
    need_session_view, session_selection_view, settings_menu_view, welcome_view, workspace_selection_view,
};

use super::command_router_questions::handle_question_reply;
use super::command_router_resume::start_resume;
use super::command_router_util::{
    parse_question_numbers, refresh_assistant_name_if_missing, result_from_menu, result_from_menu_with_forward,
    short_path_name,
};

use super::command_router_session::{
    bootstrap_im_chat_after_pairing, count_workspace_sessions, create_session, load_last_dialog_pair_from_turns,
    resolve_session_agent_type,
};

use super::command_router::{parse_command, BotCommand, BotQuestion, ForwardRequest, HandleResult};

use super::locale::{current_bot_language, fmt_count, strings_for, BotStrings};

use super::menu::{MenuItem, MenuView};

use crate::agentic::image_analysis::ImageContextData;

use serde_json::Value;

use tracing::{error, info};

pub(super) async fn dispatch(
    state: &mut BotChatState,
    cmd: BotCommand,
    image_contexts: Vec<crate::agentic::image_analysis::ImageContextData>,
) -> HandleResult {
    let language = current_bot_language().await;
    let s = strings_for(language);

    // Auto-expire pending actions before any branch.
    if state.pending_expired() {
        state.clear_pending();
        let mut view = main_menu_view(state, s);
        view = view.with_body(s.pending_expired);
        return result_from_menu(state, view);
    }

    // Universal escape hatches: /menu and /start always return the main menu
    // and clear any pending action.
    if matches!(cmd, BotCommand::Menu) {
        state.clear_pending();
        return menu_or_welcome(state, s);
    }

    // Pairing-code submitted after pairing already completed 鈫?just nudge.
    if let BotCommand::PairingCode(_) = &cmd {
        if state.paired {
            let view = MenuView::plain(s.main_title_assistant)
                .with_body(s.paired_success)
                .with_items(main_menu_view(state, s).items);
            return result_from_menu(state, view);
        }
        // Not paired path is handled by the platform wait_for_pairing loop.
    }

    if !state.paired {
        return result_from_menu(state, welcome_view(s));
    }

    // Lazily resolve `current_assistant_name` for chat states that were
    // persisted before this field existed. Without this, already-paired
    // users would keep seeing the workspace folder name (e.g. "workspace")
    // until they manually re-switch assistants.
    refresh_assistant_name_if_missing(state).await;

    // Handle /cancel as task cancellation when an active session exists.
    if let BotCommand::CancelTask(turn_id) = &cmd {
        return handle_cancel_task(state, turn_id.as_deref(), s).await;
    }

    // Numeric replies: when there is a pending action, route to it.  When
    // there isn't, treat the number as an index into `last_menu_commands`.
    if let BotCommand::NumberSelection(n) = cmd {
        return handle_number(state, n, s).await;
    }

    match cmd {
        BotCommand::Help => result_from_menu(
            state,
            MenuView::plain(s.welcome_title)
                .with_body(s.help_body)
                .with_items(vec![MenuItem::default(s.item_back, "/menu")]),
        ),
        BotCommand::Settings => {
            let verbose = super::load_bot_persistence().verbose_mode;
            result_from_menu(state, settings_menu_view(verbose, state, s))
        }
        BotCommand::SwitchMode(target) => switch_mode(state, target, s).await,
        BotCommand::SetVerbose(on) => set_verbose(state, on, s).await,
        BotCommand::SwitchContext => start_switch(state, s).await,
        BotCommand::NewSession => new_session_for_mode(state, s).await,
        BotCommand::NewCodeSession => guarded_new(state, "agentic", s).await,
        BotCommand::NewCoworkSession => guarded_new(state, "Cowork", s).await,
        BotCommand::NewClawSession => guarded_new(state, "Claw", s).await,
        BotCommand::ResumeSession => start_resume(state, 0, s).await,
        BotCommand::ChatMessage(msg) => handle_chat(state, &msg, image_contexts, s).await,
        BotCommand::Menu | BotCommand::CancelTask(_) | BotCommand::NumberSelection(_) | BotCommand::PairingCode(_) => {
            menu_or_welcome(state, s)
        } // already handled
    }
}

pub(super) async fn switch_mode(
    state: &mut BotChatState,
    target: BotDisplayMode,
    s: &'static BotStrings,
) -> HandleResult {
    if state.display_mode == target {
        let body = if target == BotDisplayMode::Pro {
            s.mode_already_expert
        } else {
            s.mode_already_assistant
        };
        let mut view = main_menu_view(state, s);
        view = view.with_body(body);
        return result_from_menu(state, view);
    }
    state.display_mode = target;
    let body = if target == BotDisplayMode::Pro {
        s.mode_switched_to_expert
    } else {
        s.mode_switched_to_assistant
    };
    let mut view = main_menu_view(state, s);
    view = view.with_body(body);
    result_from_menu(state, view)
}

pub(super) async fn confirm_then_run(
    state: &mut BotChatState,
    target: BotDisplayMode,
    target_cmd: String,
    s: &'static BotStrings,
) -> HandleResult {
    state.set_pending(PendingAction::ConfirmModeSwitch {
        target_mode: target,
        target_cmd,
    });
    result_from_menu(state, confirm_mode_switch_view(target, s))
}

pub(super) async fn set_verbose(state: &mut BotChatState, on: bool, s: &'static BotStrings) -> HandleResult {
    let mut data = super::load_bot_persistence();
    data.verbose_mode = on;
    super::save_bot_persistence(&data);

    let body = if on { s.verbose_enabled } else { s.verbose_disabled };
    let mut view = settings_menu_view(on, state, s);
    view = view.with_body(body);
    result_from_menu(state, view)
}

pub(super) async fn start_switch(state: &mut BotChatState, s: &'static BotStrings) -> HandleResult {
    use crate::service::workspace::global_workspace_service;

    let ws_service = match global_workspace_service() {
        Some(s) => s,
        None => {
            return result_from_menu(
                state,
                MenuView::plain(s.workspace_service_unavailable)
                    .with_items(vec![MenuItem::default(s.item_back, "/menu")]),
            );
        }
    };

    if state.display_mode == BotDisplayMode::Pro {
        let workspaces = ws_service.recent_workspaces().await;
        if workspaces.is_empty() {
            return result_from_menu(
                state,
                MenuView::plain(s.switch_no_workspaces).with_items(vec![MenuItem::default(s.item_back, "/menu")]),
            );
        }
        let options: Vec<(String, String)> = workspaces
            .iter()
            .map(|ws| (ws.root_path.to_string_lossy().to_string(), ws.name.clone()))
            .collect();
        let view = workspace_selection_view(state, &options, s);
        state.set_pending(PendingAction::SelectWorkspace { options });
        result_from_menu(state, view)
    } else {
        let assistants = ws_service.get_assistant_workspaces().await;
        if assistants.is_empty() {
            return result_from_menu(
                state,
                MenuView::plain(s.switch_no_assistants).with_items(vec![MenuItem::default(s.item_back, "/menu")]),
            );
        }
        let options: Vec<(String, String)> = assistants
            .iter()
            .map(|ws| (ws.root_path.to_string_lossy().to_string(), ws.name.clone()))
            .collect();
        let view = assistant_selection_view(state, &options, s);
        state.set_pending(PendingAction::SelectAssistant { options });
        result_from_menu(state, view)
    }
}

pub(super) async fn select_workspace(
    state: &mut BotChatState,
    path: &str,
    name: &str,
    s: &'static BotStrings,
) -> HandleResult {
    use crate::service::workspace::global_workspace_service;

    let ws_service = match global_workspace_service() {
        Some(svc) => svc,
        None => {
            return result_from_menu(state, MenuView::plain(s.workspace_service_unavailable));
        }
    };
    let path_buf = std::path::PathBuf::from(path);
    match ws_service.open_workspace(path_buf).await {
        Ok(info) => {
            if let Err(e) =
                crate::service::snapshot::initialize_snapshot_manager_for_workspace(info.root_path.clone(), None).await
            {
                error!("Failed to init snapshot after bot workspace switch: {e}");
            }
            state.current_workspace = Some(path.to_string());
            state.current_session_id = None;
            info!("Bot switched workspace to: {path}");

            let session_count = count_workspace_sessions(path).await;
            let body = format!(
                "{}: {} 路 {}",
                s.current_workspace_label,
                name,
                fmt_count(s.workspace_session_count_fmt, session_count),
            );
            let mut view = main_menu_view(state, s);
            view = view.with_body(body);
            result_from_menu(state, view)
        }
        Err(e) => result_from_menu(state, MenuView::plain(format!("{}{e}", s.workspace_open_failed_prefix))),
    }
}

pub(super) async fn select_assistant(
    state: &mut BotChatState,
    path: &str,
    name: &str,
    s: &'static BotStrings,
) -> HandleResult {
    use crate::service::workspace::global_workspace_service;

    let ws_service = match global_workspace_service() {
        Some(svc) => svc,
        None => {
            return result_from_menu(state, MenuView::plain(s.workspace_service_unavailable));
        }
    };
    let path_buf = std::path::PathBuf::from(path);
    match ws_service.open_workspace(path_buf).await {
        Ok(info) => {
            if let Err(e) =
                crate::service::snapshot::initialize_snapshot_manager_for_workspace(info.root_path.clone(), None).await
            {
                error!("Failed to init snapshot after bot assistant switch: {e}");
            }
            state.current_assistant = Some(path.to_string());
            state.current_assistant_name = Some(name.to_string());
            state.current_session_id = None;
            info!("Bot switched assistant to: {path}");

            let session_count = count_workspace_sessions(path).await;
            let body = format!(
                "{}: {} 路 {}",
                s.current_assistant_label,
                name,
                fmt_count(s.workspace_session_count_fmt, session_count),
            );
            let mut view = main_menu_view(state, s);
            view = view.with_body(body);
            result_from_menu(state, view)
        }
        Err(e) => result_from_menu(state, MenuView::plain(format!("{}{e}", s.workspace_open_failed_prefix))),
    }
}

pub(super) fn truncate_label(label: &str, max_chars: usize) -> String {
    let trimmed = label.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

pub(super) async fn select_session(
    state: &mut BotChatState,
    session_id: &str,
    session_name: &str,
    s: &'static BotStrings,
) -> HandleResult {
    state.current_session_id = Some(session_id.to_string());
    info!("Bot resumed session: {session_id}");

    let last_pair = load_last_dialog_pair_from_turns(state.current_workspace.as_deref(), session_id).await;
    let mut body = format!("{}{}\n", s.resume_resumed_prefix, session_name);
    if let Some((user_text, ai_text)) = last_pair {
        body.push('\n');
        body.push_str(s.resume_last_dialog_header);
        body.push('\n');
        body.push_str(&format!("{}: {}\n\n", s.resume_you_label, user_text));
        body.push_str(&format!("AI: {}\n\n", ai_text));
        body.push_str(s.resume_continue_hint);
    } else {
        body.push('\n');
        body.push_str(s.resume_first_message_hint);
    }

    // Resumed session leaves the user ready to chat …"show no menu so the
    // chat surface stays uncluttered.
    let view = MenuView::plain("").with_body(body);
    result_from_menu(state, view)
}

pub(super) async fn new_session_for_mode(state: &mut BotChatState, s: &'static BotStrings) -> HandleResult {
    let agent_type = if state.display_mode == BotDisplayMode::Pro {
        "agentic"
    } else {
        "Claw"
    };
    guarded_new(state, agent_type, s).await
}

pub(super) async fn guarded_new(state: &mut BotChatState, agent_type: &str, s: &'static BotStrings) -> HandleResult {
    let needs_pro = matches!(agent_type, "agentic" | "Cowork");
    let needs_assistant = matches!(agent_type, "Claw");

    if needs_pro && state.display_mode != BotDisplayMode::Pro {
        let target_cmd = match agent_type {
            "agentic" => "/new_code_session",
            "Cowork" => "/new_cowork_session",
            _ => "/new_code_session",
        };
        return confirm_then_run(state, BotDisplayMode::Pro, target_cmd.to_string(), s).await;
    }
    if needs_assistant && state.display_mode != BotDisplayMode::Assistant {
        return confirm_then_run(state, BotDisplayMode::Assistant, "/new_claw_session".to_string(), s).await;
    }
    if needs_pro && state.current_workspace.is_none() {
        return result_from_menu(
            state,
            MenuView::plain(s.no_workspace).with_items(vec![
                MenuItem::primary(s.item_switch_workspace, "/switch"),
                MenuItem::default(s.item_back, "/menu"),
            ]),
        );
    }
    create_session(state, agent_type).await
}

pub(super) async fn handle_cancel_task(
    state: &mut BotChatState,
    requested_turn_id: Option<&str>,
    s: &'static BotStrings,
) -> HandleResult {
    use crate::service::remote_connect::remote_server::get_or_init_global_dispatcher;

    let session_id = match state.current_session_id.clone() {
        Some(id) => id,
        None => {
            return result_from_menu(state, MenuView::plain(s.task_no_active));
        }
    };
    let dispatcher = get_or_init_global_dispatcher();
    match dispatcher.cancel_task(&session_id, requested_turn_id).await {
        Ok(_) => {
            state.clear_pending();
            result_from_menu(state, MenuView::plain(s.task_cancel_requested))
        }
        Err(e) => result_from_menu(state, MenuView::plain(format!("{}{e}", s.task_cancel_failed_prefix))),
    }
}

pub(super) async fn handle_number(state: &mut BotChatState, n: usize, s: &'static BotStrings) -> HandleResult {
    if let Some(pending) = state.pending_action.clone() {
        return route_pending(state, pending, &n.to_string(), s).await;
    }
    // No pending action: 0 always returns to main menu.
    if n == 0 {
        return menu_or_welcome(state, s);
    }
    if n >= 1 && n <= state.last_menu_commands.len() {
        let cmd_str = state.last_menu_commands[n - 1].clone();
        let next_cmd = parse_command(&cmd_str);
        return Box::pin(dispatch(state, next_cmd, vec![])).await;
    }
    handle_chat(state, &n.to_string(), vec![], s).await
}

pub(super) async fn route_pending(
    state: &mut BotChatState,
    pending: PendingAction,
    raw_input: &str,
    s: &'static BotStrings,
) -> HandleResult {
    match pending {
        PendingAction::SelectWorkspace { options } => {
            let parsed: Option<usize> = raw_input.parse().ok();
            match parsed {
                Some(0) => {
                    state.clear_pending();
                    menu_or_welcome(state, s)
                }
                Some(n) if n >= 1 && n <= options.len() => {
                    state.clear_pending();
                    let (path, name) = options[n - 1].clone();
                    select_workspace(state, &path, &name, s).await
                }
                _ => {
                    state.set_pending(PendingAction::SelectWorkspace { options });
                    Box::pin(pending_invalid(state, s)).await
                }
            }
        }
        PendingAction::SelectAssistant { options } => {
            let parsed: Option<usize> = raw_input.parse().ok();
            match parsed {
                Some(0) => {
                    state.clear_pending();
                    menu_or_welcome(state, s)
                }
                Some(n) if n >= 1 && n <= options.len() => {
                    state.clear_pending();
                    let (path, name) = options[n - 1].clone();
                    select_assistant(state, &path, &name, s).await
                }
                _ => {
                    state.set_pending(PendingAction::SelectAssistant { options });
                    Box::pin(pending_invalid(state, s)).await
                }
            }
        }
        PendingAction::SelectSession {
            options,
            page,
            has_more,
        } => {
            let parsed: Option<usize> = raw_input.parse().ok();
            match parsed {
                Some(0) if has_more => {
                    state.clear_pending();
                    start_resume(state, page + 1, s).await
                }
                Some(0) => {
                    state.clear_pending();
                    menu_or_welcome(state, s)
                }
                Some(n) if n >= 1 && n <= options.len() => {
                    state.clear_pending();
                    let (id, name) = options[n - 1].clone();
                    select_session(state, &id, &name, s).await
                }
                _ => {
                    state.set_pending(PendingAction::SelectSession {
                        options,
                        page,
                        has_more,
                    });
                    Box::pin(pending_invalid(state, s)).await
                }
            }
        }
        PendingAction::AskUserQuestion {
            tool_id,
            questions,
            current_index,
            answers,
            awaiting_custom_text,
            pending_answer,
        } => {
            handle_question_reply(
                state,
                tool_id,
                questions,
                current_index,
                answers,
                awaiting_custom_text,
                pending_answer,
                raw_input,
                s,
            )
            .await
        }
        PendingAction::ConfirmModeSwitch {
            target_mode,
            target_cmd,
        } => {
            let parsed: Option<usize> = raw_input.parse().ok();
            match parsed {
                Some(1) => {
                    state.clear_pending();
                    state.display_mode = target_mode;
                    let next_cmd = parse_command(&target_cmd);
                    Box::pin(dispatch(state, next_cmd, vec![])).await
                }
                Some(0) => {
                    state.clear_pending();
                    menu_or_welcome(state, s)
                }
                _ => {
                    state.set_pending(PendingAction::ConfirmModeSwitch {
                        target_mode,
                        target_cmd,
                    });
                    Box::pin(pending_invalid(state, s)).await
                }
            }
        }
    }
}

pub(super) async fn pending_invalid(state: &mut BotChatState, s: &'static BotStrings) -> HandleResult {
    state.pending_invalid_count = state.pending_invalid_count.saturating_add(1);
    if state.pending_invalid_count >= PENDING_INVALID_LIMIT {
        state.clear_pending();
        let mut view = main_menu_view(state, s);
        view = view.with_body(s.pending_invalid_after_retries);
        return result_from_menu(state, view);
    }
    // Re-render the pending prompt with an invalid-input notice so the user
    // sees the option list again instead of just an opaque error.
    let pending = match state.pending_action.clone() {
        Some(p) => p,
        None => {
            return result_from_menu(state, main_menu_view(state, s));
        }
    };
    let mut view = match &pending {
        PendingAction::SelectWorkspace { options } => workspace_selection_view(state, options, s),
        PendingAction::SelectAssistant { options } => assistant_selection_view(state, options, s),
        PendingAction::SelectSession {
            options,
            page,
            has_more,
        } => session_selection_view(state, options, *page, *has_more, s),
        PendingAction::AskUserQuestion {
            questions,
            current_index,
            awaiting_custom_text,
            ..
        } => build_question_view(s, questions, *current_index, *awaiting_custom_text),
        PendingAction::ConfirmModeSwitch { target_mode, .. } => confirm_mode_switch_view(*target_mode, s),
    };
    let original_body = view.body.take().unwrap_or_default();
    let new_body = if original_body.is_empty() {
        s.pending_invalid_input.to_string()
    } else {
        format!("{}\n\n{}", s.pending_invalid_input, original_body)
    };
    view = view.with_body(new_body);
    result_from_menu(state, view)
}

pub(super) async fn handle_chat(
    state: &mut BotChatState,
    message: &str,
    image_contexts: Vec<crate::agentic::image_analysis::ImageContextData>,
    s: &'static BotStrings,
) -> HandleResult {
    // If there is a pending action, route the message to it (text answer for
    // questions, "ignore" for menu-style pendings).
    if let Some(pending) = state.pending_action.clone() {
        return route_pending(state, pending, message, s).await;
    }

    if state.display_mode == BotDisplayMode::Pro && state.current_workspace.is_none() {
        return result_from_menu(
            state,
            MenuView::plain(s.no_workspace).with_items(vec![
                MenuItem::primary(s.item_switch_workspace, "/switch"),
                MenuItem::default(s.item_back, "/menu"),
            ]),
        );
    }
    if state.current_session_id.is_none() {
        return result_from_menu(state, need_session_view(state, s));
    }
    // Pre-existing safe unwrap (1f19784): the is_none() check above guarantees
    // Some — the unwrap() here is intentional, not a missing invariant. Kept
    // as-is per the "no NEW iron rule violations" rule; this comment exists
    // so future readers don't mis-classify it as new debt.
    let session_id = state.current_session_id.clone().unwrap();
    let turn_id = format!("turn_{}", uuid::Uuid::new_v4());

    // Pick the agent type from the actual session …"NOT a hardcoded
    // "agentic" …"otherwise every chat message goes through the Code
    // (`agentic`) agent regardless of what kind of session was created.
    // Concretely: the IM pairing bootstrap creates a `Claw` session for
    // assistant mode, but the old hardcoded value caused all subsequent
    // messages to be re-routed to the Code agent and the assistant flow
    // was effectively bypassed.  We mirror the agent type the session was
    // actually created with, falling back to "agentic" only if the session
    // is missing in memory (e.g. needs lazy restore …"`send_message` will
    // also normalize via `resolve_agent_type`).
    let agent_type = resolve_session_agent_type(&session_id)
        .await
        .unwrap_or_else(|| "agentic".to_string());

    // Intentionally do NOT send a "Processing..." / "Queued" interstitial
    // message with a Cancel-task menu. The session manager queues new user
    // messages automatically: the user can simply send another message and
    // it will be processed once the current atomic step finishes. Showing
    // a cancel button adds noise (especially on WeChat where every reply
    // costs a context_token slot) without giving the user anything they
    // actually need. The empty `MenuView::default()` here is silently
    // dropped by every adapter's `send_handle_result` (see the
    // empty-text guards in weixin.rs / feishu.rs / telegram.rs).
    let view = MenuView::default();

    let forward = ForwardRequest {
        session_id,
        content: message.to_string(),
        agent_type,
        turn_id,
        image_contexts,
    };

    result_from_menu_with_forward(state, view, Some(forward))
}

pub(super) fn truncate_at_char_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}
