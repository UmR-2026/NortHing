//! Bot chat state (Round 14 split).
//!
//! Owns:
//! - `PENDING_TTL_SECS` / `PENDING_INVALID_LIMIT` consts
//! - `BotDisplayMode` enum
//! - `BotChatState` struct + impl
//! - `PendingAction` enum
//! - `now_secs` helper
//!
//! See command_router.rs (facade) for public API surface.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::command_router::BotQuestion;

pub(super) const PENDING_TTL_SECS: i64 = 5 * 60;
/// How many invalid replies are tolerated before pending state is auto-cleared.
pub(super) const PENDING_INVALID_LIMIT: u8 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BotDisplayMode {
    /// Expert mode: can create Code / Cowork sessions on real workspaces.
    #[serde(rename = "pro")]
    Pro,
    /// Default assistant mode: Claw sessions on the assistant workspace.
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotChatState {
    pub chat_id: String,
    pub paired: bool,
    pub current_workspace: Option<String>,
    pub current_assistant: Option<String>,
    /// Human-readable name of the active assistant (e.g. "默认助理" / "Bob").
    /// Populated alongside `current_assistant` from `WorkspaceInfo.name` so
    /// the assistant-mode menu body can show a meaningful label instead of
    /// the workspace directory name (which is often a generic
    /// "workspace" / "workspace-<uuid>" folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_assistant_name: Option<String>,
    pub current_session_id: Option<String>,
    #[serde(default)]
    pub display_mode: BotDisplayMode,

    /// Active interactive prompt awaiting a user reply.
    /// Not persisted — cleared on bot restart.
    #[serde(skip)]
    pub pending_action: Option<PendingAction>,
    /// Unix timestamp (seconds) when the current `pending_action` becomes
    /// invalid.  Refreshed whenever a new pending action is set.
    #[serde(skip)]
    pub pending_expires_at: i64,
    /// How many invalid replies the user has sent against the current
    /// pending action.  Resets on every successful transition.
    #[serde(skip)]
    pub pending_invalid_count: u8,

    /// Commands corresponding to the items in the most recent menu, used so
    /// numeric replies (`1` ~ `last_menu_commands.len()`) work without
    /// platform-native buttons.  Not persisted.
    #[serde(skip, default)]
    pub last_menu_commands: Vec<String>,
}

impl BotChatState {
    pub fn new(chat_id: String) -> Self {
        Self {
            chat_id,
            paired: false,
            current_workspace: None,
            current_assistant: None,
            current_assistant_name: None,
            current_session_id: None,
            display_mode: BotDisplayMode::Assistant,
            pending_action: None,
            pending_expires_at: 0,
            pending_invalid_count: 0,
            last_menu_commands: Vec::new(),
        }
    }

    /// Returns the workspace root path that should be used to resolve relative
    /// file references emitted by the agent (e.g. markdown links in replies).
    ///
    /// In Pro mode this is the explicitly switched workspace
    /// (`current_workspace`); in Assistant mode the agent runs against the
    /// per-user assistant workspace held in `current_assistant`. IM platform
    /// adapters MUST consult both — looking only at `current_workspace` causes
    /// auto-push to silently drop relative-path attachments produced by
    /// assistant sessions (the most common case for end users).
    pub fn active_workspace_path(&self) -> Option<String> {
        self.current_workspace
            .clone()
            .or_else(|| self.current_assistant.clone())
    }

    pub(super) fn set_pending(&mut self, action: PendingAction) {
        self.pending_action = Some(action);
        self.pending_expires_at = now_secs() + PENDING_TTL_SECS;
        self.pending_invalid_count = 0;
    }

    pub(super) fn clear_pending(&mut self) {
        self.pending_action = None;
        self.pending_expires_at = 0;
        self.pending_invalid_count = 0;
    }

    pub(super) fn pending_expired(&self) -> bool {
        self.pending_action.is_some() && now_secs() > self.pending_expires_at
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    SelectWorkspace {
        options: Vec<(String, String)>,
    },
    SelectAssistant {
        options: Vec<(String, String)>,
    },
    SelectSession {
        options: Vec<(String, String)>,
        page: usize,
        has_more: bool,
    },
    AskUserQuestion {
        tool_id: String,
        questions: Vec<BotQuestion>,
        current_index: usize,
        answers: Vec<Value>,
        awaiting_custom_text: bool,
        pending_answer: Option<Value>,
    },
    /// Confirm switching to the other display mode and then run `target_cmd`.
    ConfirmModeSwitch {
        target_mode: BotDisplayMode,
        target_cmd: String,
    },
}
