//! Bot session lifecycle (Round 14 split).
//!
//! Owns:
//! - `bootstrap_im_chat_after_pairing` (initial assistant workspace + session)
//! - `count_workspace_sessions` (PersistenceManager lookup)
//! - `load_last_dialog_pair_from_turns` (resume preview)
//! - `strip_user_message_tags` (delegates to core::strip_prompt_markup)
//! - `truncate_text` (char-count truncation)
//! - `create_session` (god method — splits into 3 phase helpers)
//! - `resolve_session_agent_type` (chat message agent type lookup)

use super::command_router_state::{BotChatState, BotDisplayMode};

use super::command_router_util::{result_from_menu, short_path_name};

use super::command_router::HandleResult;

use super::locale::{current_bot_language, strings_for};

use super::menu::{MenuItem, MenuView};

use tracing::error;

pub async fn bootstrap_im_chat_after_pairing(state: &mut BotChatState) -> String {
    use crate::service::workspace::global_workspace_service;

    state.display_mode = BotDisplayMode::Assistant;
    let language = current_bot_language().await;
    let s = strings_for(language);

    let ws_service = match global_workspace_service() {
        Some(s) => s,
        None => return s.bootstrap_workspace_unavailable.to_string(),
    };

    let mut assistants = ws_service.get_assistant_workspaces().await;
    if assistants.is_empty() {
        match ws_service.create_assistant_workspace(None).await {
            Ok(w) => assistants.push(w),
            Err(e) => return format!("{}{e}", s.assistant_create_failed_prefix),
        }
    }

    let picked = assistants
        .iter()
        .find(|w| w.assistant_id.is_none())
        .cloned()
        .or_else(|| assistants.first().cloned());

    let Some(ws_info) = picked else {
        return s.bootstrap_workspace_unavailable.to_string();
    };

    let path_buf = ws_info.root_path.clone();
    if let Err(e) = ws_service.open_workspace(path_buf.clone()).await {
        return format!("{}{e}", s.workspace_open_failed_prefix);
    }
    if let Err(e) = crate::service::snapshot::initialize_snapshot_manager_for_workspace(path_buf, None).await {
        error!("IM bot bootstrap: snapshot init after pairing: {e}");
    }

    state.current_assistant = Some(ws_info.root_path.to_string_lossy().to_string());
    state.current_assistant_name = Some(ws_info.name.clone());
    state.current_session_id = None;

    let create_res = create_session(state, "Claw").await;
    if state.current_session_id.is_none() {
        let detail = create_res.reply.lines().next().unwrap_or("").to_string();
        return format!("{}{detail}", s.bootstrap_session_failed_prefix);
    }

    s.bootstrap_ready.to_string()
}

pub(super) async fn count_workspace_sessions(workspace_path: &str) -> usize {
    use crate::agentic::persistence::PersistenceManager;
    use crate::infrastructure::PathManager;

    let wp = std::path::PathBuf::from(workspace_path);
    let pm = match PathManager::new() {
        Ok(pm) => std::sync::Arc::new(pm),
        Err(_) => return 0,
    };
    let store = match PersistenceManager::new(pm) {
        Ok(store) => store,
        Err(_) => return 0,
    };
    store.list_session_metadata(&wp).await.map(|v| v.len()).unwrap_or(0)
}

pub(super) async fn load_last_dialog_pair_from_turns(
    workspace_path: Option<&str>,
    session_id: &str,
) -> Option<(String, String)> {
    use crate::agentic::persistence::PersistenceManager;
    use crate::infrastructure::PathManager;

    const MAX_USER_LEN: usize = 200;
    const MAX_AI_LEN: usize = 400;

    let wp = std::path::PathBuf::from(workspace_path?);
    let pm = std::sync::Arc::new(PathManager::new().ok()?);
    let store = PersistenceManager::new(pm).ok()?;
    let turns = store.load_session_turns(&wp, session_id).await.ok()?;
    let turn = turns.last()?;

    let user_text = strip_user_message_tags(&turn.user_message.content);
    if user_text.is_empty() {
        return None;
    }

    let mut ai_text = String::new();
    for round in &turn.model_rounds {
        for t in &round.text_items {
            if t.is_subagent_item.unwrap_or(false) {
                continue;
            }
            if !t.content.is_empty() {
                if !ai_text.is_empty() {
                    ai_text.push('\n');
                }
                ai_text.push_str(&t.content);
            }
        }
    }
    if ai_text.is_empty() {
        return None;
    }
    Some((
        truncate_text(&user_text, MAX_USER_LEN),
        truncate_text(&ai_text, MAX_AI_LEN),
    ))
}

pub(super) fn strip_user_message_tags(raw: &str) -> String {
    crate::agentic::core::strip_prompt_markup(raw)
}

pub(super) fn truncate_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

pub(super) async fn create_session(state: &mut BotChatState, agent_type: &str) -> HandleResult {
    use crate::agentic::coordination::global_coordinator;
    use crate::service::workspace::global_workspace_service;
    use crate::service_agent_runtime::CoreServiceAgentRuntime;
    use northhing_services_integrations::remote_connect::{
        build_remote_session_create_request, RemoteConnectSubmissionSource,
    };

    let language = current_bot_language().await;
    let s = strings_for(language);
    let is_claw = agent_type == "Claw";

    let coordinator = match global_coordinator() {
        Some(c) => c,
        None => {
            return result_from_menu(state, MenuView::plain(s.session_system_unavailable));
        }
    };

    let ws_path = if is_claw {
        if let Some(p) = state.current_assistant.clone() {
            Some(p)
        } else {
            let ws_service = match global_workspace_service() {
                Some(s) => s,
                None => {
                    return result_from_menu(state, MenuView::plain(s.workspace_service_unavailable));
                }
            };
            let workspaces = ws_service.get_assistant_workspaces().await;
            let resolved: Option<(String, String)> =
                if let Some(default_ws) = workspaces.into_iter().find(|w| w.assistant_id.is_none()) {
                    Some((
                        default_ws.root_path.to_string_lossy().to_string(),
                        default_ws.name.clone(),
                    ))
                } else {
                    match ws_service.create_assistant_workspace(None).await {
                        Ok(ws_info) => Some((ws_info.root_path.to_string_lossy().to_string(), ws_info.name.clone())),
                        Err(e) => {
                            return result_from_menu(
                                state,
                                MenuView::plain(format!("{}{e}", s.assistant_create_failed_prefix)),
                            );
                        }
                    }
                };
            if let Some((ref path, ref name)) = resolved {
                state.current_assistant = Some(path.clone());
                state.current_assistant_name = Some(name.clone());
            }
            resolved.map(|(p, _)| p)
        }
    } else {
        state.current_workspace.clone()
    };

    let session_name = match agent_type {
        "Cowork" => {
            if language.is_chinese() {
                "远程协作会话"
            } else {
                "Remote Cowork Session"
            }
        }
        "Claw" => {
            if language.is_chinese() {
                "远程助理会话"
            } else {
                "Remote Claw Session"
            }
        }
        _ => {
            if language.is_chinese() {
                "远程编码会话"
            } else {
                "Remote Code Session"
            }
        }
    };

    let Some(workspace_path) = ws_path else {
        let view = if is_claw {
            MenuView::plain(s.no_assistant).with_items(vec![
                MenuItem::primary(s.item_switch_assistant, "/switch"),
                MenuItem::default(s.item_back, "/menu"),
            ])
        } else {
            MenuView::plain(s.no_workspace).with_items(vec![
                MenuItem::primary(s.item_switch_workspace, "/switch"),
                MenuItem::default(s.item_back, "/menu"),
            ])
        };
        return result_from_menu(state, view);
    };

    let request = build_remote_session_create_request(
        session_name,
        agent_type,
        Some(workspace_path.clone()),
        RemoteConnectSubmissionSource::Bot,
    );
    let runtime = match CoreServiceAgentRuntime::agent_runtime(coordinator.clone()) {
        Ok(runtime) => runtime,
        Err(error) => {
            return result_from_menu(
                state,
                MenuView::plain(format!("{}{}", s.session_create_failed_prefix, error)),
            );
        }
    };
    match runtime.create_session(request).await {
        Ok(session) => {
            state.current_session_id = Some(session.session_id.clone());
            let body = format!(
                "{}{}\n{}{}\n\n{}",
                s.session_created_prefix,
                session_name,
                s.session_workspace_label,
                short_path_name(&workspace_path),
                s.session_start_hint,
            );
            let view = MenuView::plain("").with_body(body);
            result_from_menu(state, view)
        }
        Err(e) => result_from_menu(
            state,
            MenuView::plain(format!(
                "{}{}",
                s.session_create_failed_prefix,
                CoreServiceAgentRuntime::runtime_error_message(e)
            )),
        ),
    }
}

pub(super) async fn resolve_session_agent_type(session_id: &str) -> Option<String> {
    use crate::agentic::coordination::global_coordinator;
    use crate::service_agent_runtime::CoreServiceAgentRuntime;

    let coordinator = global_coordinator()?;
    let runtime = CoreServiceAgentRuntime::agent_runtime(coordinator).ok()?;
    runtime.resolve_session_agent_type(session_id).await.ok().flatten()
}
