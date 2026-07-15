//! Shell integration types and script helpers.
//!
//! Subdirectory sibling — `super` resolves to `shell::integration`. Cross-sibling:
//! `super::types::*` exports used by `shell_integration.rs` and
//! `shell_integration_manager.rs`.

use crate::shell::ShellType;

/// OSC 633 sequence types for shell integration
#[derive(Debug, Clone, PartialEq)]
pub enum OscSequence {
    /// 633;A - Prompt started
    PromptStart,
    /// 633;B - Command input started (prompt ended)
    CommandInputStart,
    /// 633;C - Command execution started
    CommandExecutionStart,
    /// 633;D[;exitCode] - Command finished with optional exit code
    CommandFinished { exit_code: Option<i32> },
    /// 633;E;commandLine[;nonce] - Command line content
    CommandLine { command: String, nonce: Option<String> },
    /// 633;F - Continuation prompt start
    ContinuationStart,
    /// 633;G - Continuation prompt end
    ContinuationEnd,
    /// 633;H - Right prompt start
    RightPromptStart,
    /// 633;I - Right prompt end
    RightPromptEnd,
    /// 633;P;property=value - Property
    Property { key: String, value: String },
}

/// Command execution state
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CommandState {
    /// Waiting for prompt
    #[default]
    Idle,
    /// Prompt is being displayed
    Prompt,
    /// User is inputting command
    Input,
    /// Command is executing
    Executing,
    /// Command has finished (but may still have pending output)
    Finished { exit_code: Option<i32> },
}

impl CommandState {
    /// Check if we should still collect output (executing or just finished)
    ///
    /// Note: This only checks the state itself. `ShellIntegration::should_collect_output()`
    /// also considers the `post_command_collecting` flag for ConPTY late output.
    pub fn should_collect_output(&self) -> bool {
        matches!(self, CommandState::Executing | CommandState::Finished { .. })
    }
}

/// Event emitted by shell integration
#[derive(Debug, Clone)]
pub enum ShellIntegrationEvent {
    /// Command started executing
    CommandStarted { command: String, command_id: String },
    /// Command finished with exit code
    CommandFinished { command_id: String, exit_code: Option<i32> },
    /// Current working directory changed
    CwdChanged { cwd: String },
    /// Shell property changed
    PropertyChanged { key: String, value: String },
    /// Output data received during command execution
    OutputData { command_id: String, data: String },
}

/// Get the path to shell integration script for a given shell type
pub fn get_integration_script_path(shell_type: &ShellType) -> Option<&'static str> {
    match shell_type {
        ShellType::Bash => Some("shellIntegration-bash.sh"),
        ShellType::Zsh => Some("shellIntegration-rc.zsh"),
        ShellType::Fish => Some("shellIntegration.fish"),
        ShellType::PowerShell | ShellType::PowerShellCore => Some("shellIntegration.ps1"),
        _ => None,
    }
}

/// Get the shell integration script content embedded in the binary
pub fn get_integration_script_content(shell_type: &ShellType) -> Option<&'static str> {
    match shell_type {
        ShellType::Bash => Some(include_str!("../scripts/shellIntegration-bash.sh")),
        ShellType::Zsh => Some(include_str!("../scripts/shellIntegration-rc.zsh")),
        ShellType::Fish => Some(include_str!("../scripts/shellIntegration.fish")),
        ShellType::PowerShell | ShellType::PowerShellCore => Some(include_str!("../scripts/shellIntegration.ps1")),
        _ => None,
    }
}

/// Generate shell command to inject shell integration
pub fn get_injection_command(shell_type: &ShellType, script_path: &str) -> Option<String> {
    match shell_type {
        ShellType::Bash => Some(format!(r#"source "{}""#, script_path.replace('\\', "/"))),
        ShellType::Zsh => Some(format!(r#"source "{}""#, script_path.replace('\\', "/"))),
        ShellType::Fish => Some(format!(r#"source "{}""#, script_path.replace('\\', "/"))),
        ShellType::PowerShell | ShellType::PowerShellCore => Some(format!(r#". "{}""#, script_path.replace('/', "\\"))),
        _ => None,
    }
}
