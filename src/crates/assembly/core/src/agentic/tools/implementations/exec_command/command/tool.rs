//! Top-level [`ExecCommandTool`] plumbing: the unit struct, `Default`,
//! `new`, `local_shell_prompt_info`, and the [`Tool`] trait impl with the
//! `call_impl` dispatcher. The dispatcher delegates the local path to
//! [`ExecCommandTool::call_local_pipe`] in [`super::local`] and the remote
//! path to [`ExecCommandTool::call_remote_pipe`] in [`super::remote`].

use async_trait::async_trait;
use serde_json::{json, Value};

use super::super::local_shell::resolve_local_exec_shell;
use super::types::ExecCommandShellPromptInfo;
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext, ValidationResult};
use crate::agentic::tools::implementations::shell_safety;
use crate::util::errors::NortHingResult;

/// Unit struct implementation of the `ExecCommand` tool. All state lives on
/// the [`ToolUseContext`] supplied to each call; this struct only carries the
/// `name()` identity and the [`Default`] / `new` constructors.
pub struct ExecCommandTool;

impl Default for ExecCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecCommandTool {
    /// Build a fresh `ExecCommandTool`. The struct holds no state; this
    /// exists so callers can write `ExecCommandTool::new()` instead of the
    /// unit struct literal.
    pub fn new() -> Self {
        Self
    }

    /// Build the local-shell description consumed by the prompt builder.
    /// The returned [`ExecCommandShellPromptInfo`] is `pub(crate)` because
    /// only callers inside `northhing-core` need the field-level detail.
    pub(crate) async fn local_shell_prompt_info() -> ExecCommandShellPromptInfo {
        let shell = resolve_local_exec_shell().await;
        ExecCommandShellPromptInfo {
            display_name: shell.display_name,
            shell_type: shell.shell_type.to_string(),
            path: shell.path.to_string_lossy().to_string(),
            invocation: Self::shell_invocation_for_model(&shell.path, &shell.shell_type),
        }
    }

    /// Format a model-facing invocation hint for the resolved local shell.
    /// Moved from the original `impl ExecCommandTool` block so the
    /// sibling `local.rs` does not have to expose it across the split.
    fn shell_invocation_for_model(path: &std::path::Path, shell_type: &terminal_core::ShellType) -> String {
        let shell = path.to_string_lossy();
        match shell_type {
            terminal_core::ShellType::Bash
            | terminal_core::ShellType::Zsh
            | terminal_core::ShellType::Fish
            | terminal_core::ShellType::Sh
            | terminal_core::ShellType::Ksh
            | terminal_core::ShellType::Csh
            | terminal_core::ShellType::Custom(_) => format!("`{shell} -lc <cmd>`"),
            terminal_core::ShellType::PowerShell | terminal_core::ShellType::PowerShellCore => {
                format!("`{shell} -Command <cmd>`")
            }
            terminal_core::ShellType::Cmd => format!("`{shell} /c <cmd>`"),
        }
    }
}

#[async_trait]
impl Tool for ExecCommandTool {
    fn name(&self) -> &str {
        "ExecCommand"
    }

    async fn description(&self) -> NortHingResult<String> {
        Ok(r#"Runs a shell command in a separate process.

TTY and stdin:
- tty=true allocates a PTY and gives the command terminal semantics. Use tty=true only for commands that need interactive stdin.
- tty=false runs without a PTY. Locally this uses pipe-backed stdio; remotely it uses a non-PTY SSH exec channel.
- With tty=false, no interactive stdin is attached, and input-waiting programs may see EOF instead of a prompt.

Waiting and continuation:
- yield_time_ms waits for output until the process exits or the deadline is reached. It does not stop the process.
- If the process is still running after `yield_time_ms`, the result includes a numeric session_id.
- Use WriteStdin to poll for more output or send input to tty=true sessions, and ExecControl to interrupt or kill it.

Output:
- Output is only what was produced during this tool call's wait window.
- With tty=false, stdout and stderr ordering is not guaranteed; use tty=true or redirect stderr with 2>&1 when terminal ordering matters."#
            .to_string())
    }

    async fn description_with_context(&self, _context: Option<&ToolUseContext>) -> NortHingResult<String> {
        self.description().await
    }

    fn short_description(&self) -> String {
        "Run a command in a fresh process.".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "cmd": {
                    "type": "string",
                    "description": "Shell command to execute."
                },
                "workdir": {
                    "type": "string",
                    "description": "Optional absolute working directory path. Defaults to the workspace root."
                },
                "tty": {
                    "type": "boolean",
                    "description": "Set true only for commands that need interactive stdin. Defaults to false."
                },
                "yield_time_ms": {
                    "type": "number",
                    "description": "How long to wait for output before yielding. Defaults to 30000 ms."
                }
            },
            "required": ["cmd"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn manages_own_execution_timeout(&self) -> bool {
        true
    }

    async fn validate_input(&self, input: &Value, _context: Option<&ToolUseContext>) -> ValidationResult {
        let cmd = input.get("cmd").and_then(Value::as_str).unwrap_or_default();
        if cmd.trim().is_empty() {
            return ValidationResult {
                result: false,
                message: Some("cmd is required for ExecCommand".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }

        // R1: Shell safety denylist - block catastrophic commands before confirmation.
        // Uses guard_command_execution so denied commands are written to the audit log.
        const ENABLE_SHELL_DENYLIST: bool = true;
        if ENABLE_SHELL_DENYLIST {
            match shell_safety::guard_command_execution(cmd, "ExecCommand", true).await {
                Ok(shell_safety::GuardOutcome::DeniedByDenylist { pattern }) => {
                    return ValidationResult {
                        result: false,
                        message: Some(format!(
                            "Command matched shell denylist (R1 safety filter). Refusing to execute: {}\nMatched pattern: {}",
                            cmd, pattern
                        )),
                        error_code: Some(403),
                        meta: None,
                    };
                }
                Ok(_) => {}
                Err(e) => {
                    return ValidationResult {
                        result: false,
                        message: Some(format!("Shell safety guard error: {}", e)),
                        error_code: Some(500),
                        meta: None,
                    };
                }
            }
        }

        ValidationResult {
            result: true,
            message: None,
            error_code: None,
            meta: None,
        }
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        if context.is_remote() {
            return self.call_remote_pipe(input, context).await;
        }
        self.call_local_pipe(input, context).await
    }
}
