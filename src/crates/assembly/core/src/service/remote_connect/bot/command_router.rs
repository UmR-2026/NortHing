//! Shared command router for IM-bot connections (Round 14 facade).
//!
//! Public API surface (stable, re-exported by `bot/mod.rs`):
//!   - Types: BotChatState, BotCommand, BotAction, BotActionStyle, BotInteractiveRequest,
//!     BotInteractionHandler, BotMessageSender, BotQuestion, BotQuestionOption, BotDisplayMode,
//!     BotLanguage, HandleResult, ForwardRequest, ForwardedTurnResult, PendingAction.
//!   - Functions: parse_command, handle_command, welcome_message, complete_im_bot_pairing,
//!     execute_forwarded_turn, apply_interactive_request, current_bot_language.
//!
//! Sub-domain split (Round 14, Round 15 trim):
//!   - command_router_state: BotChatState + PendingAction + TTL consts
//!   - command_router_view: 11 view builders (welcome/menu/settings/select/question)
//!   - command_router_dispatch: 18 dispatchers + sub-routines (god method splits)
//!   - command_router_resume: `start_resume` god method (R15 extraction)
//!   - command_router_session: session lifecycle (bootstrap/create/load/resume)
//!   - command_router_util: 6 small helpers
//!   - command_router_tests: 4 test mods
//!
//! Note: sibling files are declared as `pub mod` in `bot/mod.rs` (R13b pattern:
//! siblings at the same directory level, declared in the parent mod.rs).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ==== re-exports from sub-siblings ====
pub use super::command_router_forwarded_turn::execute_forwarded_turn;
pub use super::command_router_state::{now_secs, BotChatState, BotDisplayMode, PendingAction};
pub use super::locale::{current_bot_language, BotLanguage};

// ==== re-exports used internally (cross-sibling calls) ====
use super::command_router_dispatch::{
    confirm_then_run, dispatch, guarded_new, handle_cancel_task, handle_chat, handle_number, new_session_for_mode,
    pending_invalid, route_pending, select_assistant, select_session, select_workspace, set_verbose, start_switch,
    switch_mode, truncate_at_char_boundary, truncate_label,
};
use super::command_router_questions::{handle_question_reply, submit_question_answers};
use super::command_router_resume::start_resume;
use super::command_router_session::{
    bootstrap_im_chat_after_pairing, count_workspace_sessions, create_session, load_last_dialog_pair_from_turns,
    resolve_session_agent_type,
};
use super::command_router_util::{
    normalize_im_command_text, refresh_assistant_name_if_missing, result_from_menu, result_from_menu_with_forward,
    short_path_name, strip_numeric_reply_suffix,
};
use super::command_router_view::{
    assistant_selection_view, build_question_view, confirm_mode_switch_view, main_menu_view, menu_or_welcome,
    need_session_view, question_option_line, ready_to_chat_body, session_selection_view, settings_menu_view,
    welcome_view, workspace_selection_view,
};

use super::locale::strings_for;
use super::menu::{MenuItem, MenuItemStyle, MenuView};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotQuestionOption {
    pub label: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotQuestion {
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub header: String,
    #[serde(default)]
    pub options: Vec<BotQuestionOption>,
    #[serde(rename = "multiSelect", default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotActionStyle {
    Primary,
    Default,
}

#[derive(Debug, Clone)]
pub struct BotAction {
    pub label: String,
    pub command: String,
    pub style: BotActionStyle,
}

impl BotAction {
    pub fn primary(label: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            style: BotActionStyle::Primary,
        }
    }
    pub fn secondary(label: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            style: BotActionStyle::Default,
        }
    }
}

impl From<MenuItem> for BotAction {
    fn from(item: MenuItem) -> Self {
        let style = match item.style {
            MenuItemStyle::Primary => BotActionStyle::Primary,
            // Danger and Default both map to non-primary on platforms that
            // don't have a native danger style.
            _ => BotActionStyle::Default,
        };
        BotAction {
            label: item.label,
            command: item.command,
            style,
        }
    }
}

pub struct HandleResult {
    pub reply: String,
    pub actions: Vec<BotAction>,
    pub forward_to_session: Option<ForwardRequest>,
    /// Same content as [`MenuView`] —adapters that want to render a richer
    /// view (Telegram inline keyboard, Feishu card, WeChat numbered text)
    /// can read this directly instead of `actions`.
    pub menu: MenuView,
}

#[derive(Debug, Clone)]
pub struct BotInteractiveRequest {
    pub reply: String,
    pub actions: Vec<BotAction>,
    pub menu: MenuView,
    pub pending_action: PendingAction,
}

pub type BotInteractionHandler =
    Arc<dyn Fn(BotInteractiveRequest) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub type BotMessageSender = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct ForwardRequest {
    pub session_id: String,
    pub content: String,
    pub agent_type: String,
    pub turn_id: String,
    pub image_contexts: Vec<crate::agentic::image_analysis::ImageContextData>,
}

pub struct ForwardedTurnResult {
    pub display_text: String,
    pub full_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    /// Show welcome (unpaired) or main menu (paired).  Triggered by
    /// `/start`, `/menu`, `/m`, `菜单`, or `0` at the top level.
    Menu,
    /// Show settings sub-menu.
    Settings,
    /// Show help text.
    Help,
    /// Switch display mode.
    SwitchMode(BotDisplayMode),
    /// Toggle verbose execution-detail mode (persisted globally).
    SetVerbose(bool),
    /// Generic "switch" entry —picks workspace or assistant by mode.
    SwitchContext,
    /// Generic "new session" entry —picks the right session type by mode.
    NewSession,
    /// Specific session creators (kept as hidden aliases).
    NewCodeSession,
    NewCoworkSession,
    NewClawSession,
    /// Resume an existing session (workspace or assistant by mode).
    ResumeSession,
    /// Cancel currently running task.
    CancelTask(Option<String>),
    /// Pairing code submitted before pairing.
    PairingCode(String),
    /// Numeric reply to a menu / pending action.
    NumberSelection(usize),
    /// Free-form chat message forwarded to the AI session.
    ChatMessage(String),
}

pub fn parse_command(text: &str) -> BotCommand {
    let normalized = normalize_im_command_text(text);
    let trimmed = normalized.trim();
    if let Some(rest) = trimmed.strip_prefix("/cancel_task") {
        let arg = rest.trim();
        return if arg.is_empty() {
            BotCommand::CancelTask(None)
        } else {
            BotCommand::CancelTask(Some(arg.to_string()))
        };
    }
    if let Some(rest) = trimmed.strip_prefix("/cancel") {
        let arg = rest.trim();
        return if arg.is_empty() {
            BotCommand::CancelTask(None)
        } else {
            BotCommand::CancelTask(Some(arg.to_string()))
        };
    }
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        // Top-level navigation / settings.
        "/start" | "/menu" | "/m" | "菜单" => return BotCommand::Menu,
        "/settings" | "/s" | "设置" => return BotCommand::Settings,
        "/help" | "/?" | "/h" | "帮助" | "？" => return BotCommand::Help,

        // Mode switches (visible).
        "/expert" | "/pro" | "专业模式" => {
            return BotCommand::SwitchMode(BotDisplayMode::Pro);
        }
        "/assistant" | "助理模式" => {
            return BotCommand::SwitchMode(BotDisplayMode::Assistant);
        }

        // Verbose toggles.
        "/verbose" | "详细" => return BotCommand::SetVerbose(true),
        "/concise" | "简洁" => return BotCommand::SetVerbose(false),

        // Generic switch (picks workspace or assistant by mode).
        "/switch" | "切换" => return BotCommand::SwitchContext,
        // Hidden aliases.
        "/switch_workspace" | "切换工作区" => return BotCommand::SwitchContext,
        "/switch_assistant" | "切换助理" => return BotCommand::SwitchContext,

        // Generic "new" picks the right session type by mode.
        "/new" | "/n" | "新建" | "新建会话" | "新会话" => return BotCommand::NewSession,
        // Hidden aliases / power users.
        "/new_code_session" | "新建编码会话" => return BotCommand::NewCodeSession,
        "/new_cowork_session" | "新建协作会话" => {
            return BotCommand::NewCoworkSession;
        }
        "/new_claw_session" | "新建助理会话" => return BotCommand::NewClawSession,

        // Resume.
        "/resume" | "/r" | "/resume_session" | "恢复" | "恢复会话" => {
            return BotCommand::ResumeSession;
        }
        _ => {}
    }

    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return BotCommand::PairingCode(trimmed.to_string());
    }

    let num_token = strip_numeric_reply_suffix(trimmed);
    if let Ok(n) = num_token.parse::<usize>() {
        if n <= 99 {
            // `0` is intentionally returned as `NumberSelection(0)` so context
            // such as "next page" inside SelectSession can override the
            // default "0 = back to menu" interpretation.  See `handle_number`.
            return BotCommand::NumberSelection(n);
        }
    }
    BotCommand::ChatMessage(trimmed.to_string())
}

pub fn welcome_message(language: BotLanguage) -> &'static str {
    strings_for(language).welcome
}

pub fn apply_interactive_request(state: &mut BotChatState, req: &BotInteractiveRequest) {
    state.set_pending(req.pending_action.clone());
    state.last_menu_commands = req.menu.items.iter().map(|i| i.command.clone()).collect();
}

pub async fn handle_command(
    state: &mut BotChatState,
    cmd: BotCommand,
    images: Vec<super::super::remote_server::ImageAttachment>,
) -> HandleResult {
    let image_contexts: Vec<crate::agentic::image_analysis::ImageContextData> =
        super::super::remote_server::images_to_contexts(if images.is_empty() { None } else { Some(&images) });
    dispatch(state, cmd, image_contexts).await
}

pub async fn complete_im_bot_pairing(state: &mut BotChatState) -> HandleResult {
    state.paired = true;
    let language = current_bot_language().await;
    let s = strings_for(language);
    let note = bootstrap_im_chat_after_pairing(state).await;

    let mut view = main_menu_view(state, s);
    let combined_body = match view.body.take() {
        Some(b) => format!("{}\n\n{}\n\n{}", s.paired_success, note, b),
        None => format!("{}\n\n{}", s.paired_success, note),
    };
    view = view.with_body(combined_body);
    result_from_menu(state, view)
}
