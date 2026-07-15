//! Chat mode implementation — facade
//!
//! Interactive chat mode with TUI interface.
//! Events are consumed directly from core's EventQueue.
//!
//! This is the facade module. The actual logic is split across sibling files:
//! - `run.rs` — main event loop
//! - `input.rs` — keyboard / non-keyboard / exit handling
//! - `commands.rs` — command palette + slash commands + usage report
//! - `theme.rs` — theme selector
//! - `agent.rs` — agent selector + cycling
//! - `model.rs` — model selector + apply
//! - `session.rs` — session switch / create / selector / delete
//! - `skill.rs` — skill management
//! - `subagent.rs` — subagent management
//! - `mcp.rs` — MCP server management
//! - `model_config.rs` — provider selection / save / edit / update
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::{agentic_system::AgenticSystem, core_adapter::CoreAgentAdapter};
use crate::config::CliConfig;
use northhing_core::service::token_usage::TokenUsageService;

/// Keyboard shortcuts help text (used by commands.rs /help and /palette help)
const KEYBOARD_SHORTCUTS_HELP: &str = "\
Keyboard Shortcuts\n\
─────────────────────────────────\n\
Tab / Shift+Tab   Switch Agent\n\
Ctrl+P            Command Palette\n\
Ctrl+J / Ctrl+K   Prev / Next Tool\n\
Ctrl+O            Expand / Collapse Tool\n\
Ctrl+E            Toggle Browse Mode\n\
↑ / ↓             Input History\n\
PageUp / PageDown Scroll Messages\n\
Ctrl+Home / End   Jump to Top / Bottom\n\
Ctrl+U            Clear Input\n\
Esc               Back / Interrupt\n\
Ctrl+W            Close All Windows\n\
Ctrl+C            Quit";

/// Chat mode exit reason
#[derive(Debug, Clone, PartialEq)]
pub enum ChatExitReason {
    /// User exits program
    Quit,
    /// Switch to a different session
    SwitchSession(String),
    /// Create a new session
    NewSession,
}

/// Pending MCP operation (deferred to allow a render frame for loading state)
enum PendingMcpOp {
    Toggle(String),
    Add { name: String, config_json: String },
    Delete(String),
}

enum PendingMcpTask {
    Toggle {
        server_id: String,
        handle: tokio::task::JoinHandle<northhing_core::util::errors::NortHingResult<()>>,
    },
    Add {
        name: String,
        handle: tokio::task::JoinHandle<northhing_core::util::errors::NortHingResult<()>>,
    },
    Delete {
        server_id: String,
        handle: tokio::task::JoinHandle<northhing_core::util::errors::NortHingResult<()>>,
    },
}

#[derive(Default)]
struct NonKeyEventOutcome {
    request_redraw: bool,
    resize_seen: bool,
}

pub struct ChatMode {
    pub(crate) config: CliConfig,
    /// Current agent type (e.g. "agentic", "plan", "debug")
    pub(crate) agent_type: String,
    pub(crate) workspace: Option<String>,
    pub(crate) agent: Arc<CoreAgentAdapter>,
    pub(crate) token_usage_service: Arc<TokenUsageService>,
    /// If set, restore this existing session instead of creating a new one
    pub(crate) restore_session_id: Option<String>,
    /// If set, send this prompt automatically when the session starts
    pub(crate) initial_prompt: Option<String>,
    /// Pending MCP operation — set in key handler, executed after one render frame
    pending_mcp_op: Option<PendingMcpOp>,
    /// Running MCP tasks (non-blocking, polled in main loop)
    pending_mcp_tasks: Vec<PendingMcpTask>,
}

/// Map agent_type to a display name for status messages
fn agent_display_name(agent_type: &str) -> &'static str {
    match agent_type {
        "agentic" => "Fang",
        _ => "AI Assistant",
    }
}

impl ChatMode {
    pub fn new(
        config: CliConfig,
        agent_type: String,
        workspace: Option<String>,
        agentic_system: &AgenticSystem,
    ) -> Self {
        let agent = Arc::new(CoreAgentAdapter::new(
            agentic_system.coordinator.clone(),
            agentic_system.event_queue.clone(),
            workspace.clone().map(PathBuf::from),
        ));

        Self {
            config,
            agent_type,
            workspace,
            agent,
            token_usage_service: agentic_system.token_usage_service.clone(),
            restore_session_id: None,
            initial_prompt: None,
            pending_mcp_op: None,
            pending_mcp_tasks: Vec::new(),
        }
    }

    /// Set a session ID to restore (for "Continue Last Session")
    pub fn with_restore_session(mut self, session_id: String) -> Self {
        self.restore_session_id = Some(session_id);
        self
    }

    /// Set an initial prompt to send automatically when the session starts
    pub fn with_initial_prompt(mut self, prompt: String) -> Self {
        self.initial_prompt = Some(prompt);
        self
    }

    /// Entry point — thin wrapper that delegates to `run::run_loop`.
    pub fn run(
        &mut self,
        existing_terminal: Option<ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>>,
    ) -> Result<ChatExitReason> {
        run::run_loop(self, existing_terminal)
    }
}

// ── Sibling modules (sub-domain split) ──
pub mod agent;
pub mod commands;
pub mod input;
pub mod mcp;
pub mod model;
pub mod model_config;
pub mod run;
pub mod session;
pub mod skill;
pub mod subagent;
pub mod theme;
