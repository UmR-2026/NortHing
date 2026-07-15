//! Shared types and constants for `ExecCommandTool`.
//!
//! Constants, the private `RemoteShell` value type, and the `pub(crate)`
//! `ExecCommandShellPromptInfo` consumed by the prompt builder are
//! defined here. The `ExecCommandTool` unit struct itself lives in
//! [`super::tool`] alongside the `Default` / `new` / `Tool` impl so the
//! public type and its public impl stay in the same module.

use terminal_core::ShellType;

/// How long `resolve_remote_shell` waits for the SSH probe to return a shell
/// path. Kept short because a stale probe should not block the tool.
pub(super) const REMOTE_SHELL_PROBE_TIMEOUT_MS: u64 = 3_000;

/// Grace window after a remote non-TTY interrupt before the process group is
/// hard-killed.
pub(super) const REMOTE_NON_TTY_INTERRUPT_GRACE_SECONDS: u64 = 2;

/// Default value for `yield_time_ms` when the model does not override it.
pub(super) const DEFAULT_TOOL_YIELD_TIME_MS: u64 = 30_000;

/// PowerShell command prefix that forces UTF-8 output regardless of host
/// console encoding. Used by the local argv builder and asserted in tests.
pub(super) const POWERSHELL_UTF8_OUTPUT_PREFIX: &str = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;\n";

/// Resolved remote shell: an absolute path on the SSH target and the
/// [`ShellType`] the path maps to.
#[derive(Debug, Clone)]
pub(super) struct RemoteShell {
    pub(super) path: String,
    pub(super) shell_type: ShellType,
}

/// Prompt-builder-facing description of the local shell. Cross-crate
/// consumers in `prompt_builder_impl` call
/// [`crate::agentic::tools::implementations::exec_command::ExecCommandTool::local_shell_prompt_info`]
/// which returns this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecCommandShellPromptInfo {
    pub display_name: String,
    pub shell_type: String,
    pub path: String,
    pub invocation: String,
}
