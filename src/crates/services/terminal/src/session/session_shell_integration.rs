//! Session shell integration setup + state accessors.
//!
//! Standalone sibling — `impl SessionManager` for:
//! - `inject_shell_integration` (mutates ShellConfig to load scripts)
//! - `wait_for_session_ready` (+ static helper)
//! - `integration_manager` accessor
//! - `has_shell_integration` / `get_command_state`
//!
//! Together with `session_lifecycle`, `session_events`,
//! and `session_commands` siblings, this replaces the previous
//! monolithic 1391-line `impl SessionManager` block.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::config::ShellConfig;
use crate::shell::{CommandState, ShellIntegration, ShellIntegrationManager, ShellType};
use crate::TerminalError;

use super::super::{SessionStatus, TerminalSession};
use super::session_manager::SessionManager;

impl SessionManager {
    /// Inject shell integration scripts and environment variables
    pub(super) fn inject_shell_integration(&self, shell_config: &mut ShellConfig, shell_type: &ShellType, nonce: &str) {
        // Set environment variables for shell integration
        // NOTE: Do NOT set TERMINAL_SHELL_INTEGRATION here! The script checks this
        // variable and returns early if it's set. The script sets it itself.
        shell_config
            .env
            .insert("TERM_PROGRAM".to_string(), "terminal".to_string());
        shell_config
            .env
            .insert("TERMINAL_INJECTION".to_string(), "1".to_string());
        shell_config.env.insert("TERMINAL_NONCE".to_string(), nonce.to_string());

        // Get the script path from scripts manager
        let script_path = match self.scripts_manager.get_script_path(shell_type) {
            Some(p) => p,
            None => return,
        };

        match shell_type {
            ShellType::Bash => {
                // Check if original args had --login
                let had_login = shell_config.args.iter().any(|arg| arg == "--login" || arg == "-l");
                if had_login {
                    // Set env var for login shell handling (script will source profiles)
                    shell_config
                        .env
                        .insert("TERMINAL_SHELL_LOGIN".to_string(), "1".to_string());
                }
                // Clear all args and use --init-file with -i (interactive mode)
                // --init-file only works for interactive shells, so -i is required!
                shell_config.args.clear();
                shell_config.args.push("--init-file".to_string());
                // Convert path: use forward slashes but keep Windows format (C:/...)
                let path_str = script_path.to_string_lossy().to_string();
                #[cfg(windows)]
                let path_str = path_str.replace('\\', "/");
                shell_config.args.push(path_str);
                // IMPORTANT: Add -i to ensure bash runs in interactive mode
                // Without -i, --init-file won't be executed!
                shell_config.args.push("-i".to_string());
            }
            ShellType::Zsh => {
                // script_path is the ZDOTDIR (directory containing .zshrc)
                // Store original ZDOTDIR
                if let Ok(home) = std::env::var("HOME") {
                    shell_config.env.insert("USER_ZDOTDIR".to_string(), home);
                }
                shell_config
                    .env
                    .insert("ZDOTDIR".to_string(), script_path.to_string_lossy().to_string());
            }
            ShellType::Fish => {
                // For fish, use source command to load the script file
                shell_config.args.push("--init-command".to_string());
                shell_config.args.push(format!("source '{}'", script_path.display()));
            }
            ShellType::PowerShell | ShellType::PowerShellCore => {
                // For PowerShell, use -ExecutionPolicy Bypass to avoid security errors
                // and -NoExit to keep the shell running after script execution
                shell_config.args.push("-ExecutionPolicy".to_string());
                shell_config.args.push("Bypass".to_string());
                shell_config.args.push("-NoLogo".to_string());
                shell_config.args.push("-NoExit".to_string());
                shell_config.args.push("-File".to_string());
                shell_config.args.push(script_path.to_string_lossy().to_string());
            }
            _ => {}
        }
    }

    /// Wait for a session to be ready for command execution
    ///
    /// This ensures both the session is active and shell integration is initialized.
    /// For new sessions, it waits for the shell integration to transition from Idle
    /// to Prompt/Input state, indicating the shell is ready to accept commands.
    // reason: wait_for_session_ready() instance method is reserved for the upcoming readiness-gated command queue; today's callers use the static version with explicit handles
    async fn wait_for_session_ready(&self, session_id: &str) -> crate::TerminalResult<()> {
        Self::wait_for_session_ready_static(&self.sessions, &self.session_integrations, session_id).await
    }

    /// Static version of wait_for_session_ready that takes explicit parameters
    pub async fn wait_for_session_ready_static(
        sessions: &Arc<RwLock<HashMap<String, TerminalSession>>>,
        session_integrations: &Arc<RwLock<HashMap<String, ShellIntegration>>>,
        session_id: &str,
    ) -> crate::TerminalResult<()> {
        let ready_timeout = Duration::from_secs(30);
        let ready_start = std::time::Instant::now();
        let mut initial_integration_state = None;
        while ready_start.elapsed() < ready_timeout {
            // Check session status
            let session_status = {
                let sessions_guard = sessions.read().await;
                sessions_guard.get(session_id).map(|s| s.status.clone())
            };

            // Check shell integration state
            let integration_state = {
                let integrations = session_integrations.read().await;
                integrations.get(session_id).map(|i| i.state().clone())
            };

            // Remember the initial integration state
            if initial_integration_state.is_none() {
                initial_integration_state = integration_state.clone();
            }

            match (session_status, integration_state) {
                // Session active or starting with integration info available.
                // Accept Starting here because ProcessReady can be delayed by the
                // pty_to_session mapping race; the shell is functional once
                // integration reaches Prompt/Input regardless of session status.
                (Some(SessionStatus::Active), Some(int_state)) | (Some(SessionStatus::Starting), Some(int_state)) => {
                    if initial_integration_state == Some(CommandState::Idle) {
                        match int_state {
                            CommandState::Prompt | CommandState::Input => {
                                return Ok(());
                            }
                            CommandState::Idle => {
                                if ready_start.elapsed() >= ready_timeout {
                                    return Ok(());
                                }
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                            _ => {
                                return Ok(());
                            }
                        }
                    } else {
                        return Ok(());
                    }
                }
                (Some(SessionStatus::Terminating), _) | (Some(SessionStatus::Exited { .. }), _) => {
                    return Err(TerminalError::Session(format!("Session {} is terminated", session_id)));
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        Err(TerminalError::Session(format!(
            "Session {} did not become ready in {:?}. \
            Shell integration may have failed. This can happen if your shell config \
            (~/.bashrc, ~/.bash_profile, etc.) contains 'exec', 'exit', or 'return' statements \
            that interrupt the shell integration script. Please check your shell configuration.",
            session_id, ready_timeout
        )))
    }

    /// Get the shell integration manager
    pub fn integration_manager(&self) -> Arc<ShellIntegrationManager> {
        self.integration_manager.clone()
    }

    /// Check if a session has shell integration enabled
    pub async fn has_shell_integration(&self, session_id: &str) -> bool {
        let integrations = self.session_integrations.read().await;
        integrations.contains_key(session_id)
    }

    /// Get the current command state for a session
    pub async fn get_command_state(&self, session_id: &str) -> Option<CommandState> {
        let integrations = self.session_integrations.read().await;
        integrations.get(session_id).map(|i| i.state().clone())
    }
}
