//! Command execution result types and streaming types.
//!
//! Standalone sibling — no cross-sibling dependencies.

use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// Why a command stream reached completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandCompletionReason {
    /// Command finished normally, including signal-driven exits not caused by timeout.
    Completed,
    /// Command hit the configured timeout and terminal attempted to interrupt it.
    TimedOut,
}

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct CommandExecuteResult {
    /// The command that was executed
    pub command: String,
    /// Unique command ID
    pub command_id: String,
    /// Command output
    pub output: String,
    /// Exit code (if available)
    pub exit_code: Option<i32>,
    /// Why command execution stopped.
    pub completion_reason: CommandCompletionReason,
}

/// Options for command execution
#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    /// Timeout for command execution (None = no timeout)
    pub timeout: Option<Duration>,
    /// Whether to prevent the command from being added to shell history
    pub prevent_history: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            timeout: None,
            prevent_history: true,
        }
    }
}

/// Events emitted during streaming command execution
#[derive(Debug, Clone)]
pub enum CommandStreamEvent {
    /// Command has started executing
    Started { command_id: String },
    /// Output data received
    Output { data: String },
    /// Command reached a terminal state.
    Completed {
        exit_code: Option<i32>,
        total_output: String,
        completion_reason: CommandCompletionReason,
        /// Post-command terminal state: the most recent terminal output that
        /// was NOT part of the command's own output. This includes the shell
        /// prompt (e.g., `$ `, `dquote> `) and any other text the shell
        /// displayed after the command finished. AI agents can use this to
        /// understand the full terminal context and avoid misjudgments.
        shell_state: Option<String>,
    },
    /// Command execution failed
    Error { message: String },
}

/// A stream of command execution events
pub type CommandStream = Pin<Box<dyn Stream<Item = CommandStreamEvent> + Send>>;
