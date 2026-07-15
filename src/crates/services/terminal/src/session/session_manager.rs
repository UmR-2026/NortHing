//! Session manager — struct definition, free helper functions, and tests.
//!
//! After R28b retry, the `impl SessionManager { ... }` block is split across
//! 4 sibling files within `session/` (each owns one sub-domain):
//!
//! - [`session_lifecycle`]       — new, binding, create, get/list, IO, close,
//!                                 shutdown, Drop (impl ~370 lines)
//! - [`session_events`]          — start_event_forwarding, event_emitter,
//!                                 subscribe_session_output (impl ~210 lines)
//! - [`session_shell_integration`] — inject_shell_integration, wait_for_ready,
//!                                 integration_manager, has/get_command_state
//!                                 (impl ~250 lines)
//! - [`session_commands`]        — execute_command, execute_command_stream,
//!                                 send_command, wait_for_active (impl ~430 lines)
//!
//! This facade keeps the [`SessionManager`] struct definition, the per-session
//! shared state fields (all `pub(super)` for sibling visibility), and the small
//! free functions used by the stream execution path.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::config::TerminalConfig;
use crate::events::TerminalEventEmitter;
use crate::pty::PtyService;
use crate::shell::{ScriptsManager, ShellIntegration, ShellIntegrationManager};

use super::super::TerminalSessionBinding;

pub(super) const COMMAND_TIMEOUT_INTERRUPT_GRACE_MS: Duration = Duration::from_millis(500);

pub(super) fn compute_stream_output_delta(last_sent_output: &mut String, output: &str) -> Option<String> {
    if output.len() < last_sent_output.len() || !output.starts_with(last_sent_output.as_str()) {
        last_sent_output.clear();
    }

    let new_data = output
        .strip_prefix(last_sent_output.as_str())
        .filter(|data| !data.is_empty())
        .map(|data| data.to_string());

    last_sent_output.clear();
    last_sent_output.push_str(output);

    new_data
}

pub(super) async fn get_integration_output_snapshot(
    session_integrations: &Arc<RwLock<HashMap<String, ShellIntegration>>>,
    session_id: &str,
) -> String {
    let integrations = session_integrations.read().await;
    integrations
        .get(session_id)
        .map(|i| i.output().to_string())
        .unwrap_or_default()
}

/// Get the post-command terminal state from shell integration.
/// Returns the most recent terminal output that was NOT part of the command's
/// own output — typically the shell prompt (e.g., `$ `, `dquote> `) or any
/// other text the shell displayed after the command finished.
pub(super) async fn get_post_command_terminal_state(
    session_integrations: &Arc<RwLock<HashMap<String, ShellIntegration>>>,
    session_id: &str,
) -> Option<String> {
    let integrations = session_integrations.read().await;
    integrations.get(session_id).and_then(|i| {
        let recent = i.recent_plain_output().trim().to_string();
        if recent.is_empty() {
            None
        } else {
            Some(recent)
        }
    })
}

/// Session manager for terminal sessions
pub struct SessionManager {
    /// Configuration
    pub(super) config: TerminalConfig,

    /// Active sessions
    pub(super) sessions: Arc<RwLock<HashMap<String, TerminalSession>>>,

    /// PTY service
    pub(super) pty_service: Arc<PtyService>,

    /// Event emitter
    pub(super) event_emitter: Arc<TerminalEventEmitter>,

    /// Mapping from PTY ID to session ID
    pub(super) pty_to_session: Arc<RwLock<HashMap<u32, String>>>,

    /// Shell integration manager
    pub(super) integration_manager: Arc<ShellIntegrationManager>,

    /// Per-session shell integration instances
    pub(super) session_integrations: Arc<RwLock<HashMap<String, ShellIntegration>>>,

    /// Session binding manager for external entity bindings
    pub(super) binding: Arc<TerminalSessionBinding>,

    /// Shell integration scripts manager
    pub(super) scripts_manager: ScriptsManager,

    /// Per-session output taps for real-time output streaming
    pub(super) output_taps: Arc<DashMap<String, Vec<tokio::sync::mpsc::Sender<String>>>>,
}

// `TerminalSession` re-exported via `super::super::TerminalSession` (session/mod.rs
// declares `mod session_manager;` which becomes `crate::session::session_manager::SessionManager`).
use super::super::TerminalSession;

#[cfg(test)]
mod tests {
    use super::super::types::CommandCompletionReason;
    use super::compute_stream_output_delta;

    #[test]
    fn stream_output_delta_returns_utf8_suffix_without_cutting_chars() {
        let mut last_sent_output = "你好！我是 northhing，".to_string();
        let output = "你好！我是 northhing，可以帮助你完成软件工程任务。".to_string();

        let delta = compute_stream_output_delta(&mut last_sent_output, &output);

        assert_eq!(delta.as_deref(), Some("可以帮助你完成软件工程任务。"));
        assert_eq!(last_sent_output, output);
    }

    #[test]
    fn stream_output_delta_resets_when_previous_snapshot_is_not_prefix() {
        let mut last_sent_output = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
        let output = "你好！我是 northhing，可以帮助你完成软件工程任务。有什么我可以帮你的吗？";

        let delta = compute_stream_output_delta(&mut last_sent_output, output);

        assert_eq!(delta.as_deref(), Some(output));
        assert_eq!(last_sent_output, output);
    }

    #[test]
    fn stream_output_delta_returns_none_when_output_is_unchanged() {
        let mut last_sent_output = "hello 你好".to_string();

        let delta = compute_stream_output_delta(&mut last_sent_output, "hello 你好");

        assert_eq!(delta, None);
        assert_eq!(last_sent_output, "hello 你好");
    }

    #[test]
    fn completion_reason_serializes_with_camel_case_contract() {
        assert_eq!(
            serde_json::to_string(&CommandCompletionReason::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&CommandCompletionReason::TimedOut).unwrap(),
            "\"timedOut\""
        );
    }
}
