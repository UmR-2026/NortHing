//! `ExecCommandTool` god-split facade.
//!
//! The original 1157-line `exec_command/command.rs` was decomposed into focused
//! sibling files. This module re-exports the public surface and declares the
//! internal sub-modules so the rest of `exec_command/` keeps working
//! unchanged.
//!
//! Sub-domain split:
//! - [`types`]: constants, [`types::RemoteShell`], [`types::ExecCommandShellPromptInfo`].
//! - [`shell_helpers`]: free functions used by local + remote paths
//!   (escape, remote shell probe parsing, remote env word rendering).
//! - [`local`]: local exec command logic (argv, workdir, env, completion, lifecycle bridge,
//!   `call_local_pipe`).
//! - [`remote`]: remote exec command logic (workdir, shell, login shell command,
//!   non-TTY wrapper, env snapshot merge, completion, lifecycle bridge,
//!   `call_remote_pipe`).
//! - [`response`]: `response_for_assistant` shared by local + remote paths.
//! - [`tool`]: the `ExecCommandTool` unit struct, `Default`, `new`,
//!   `local_shell_prompt_info`, and the `Tool` trait impl with the
//!   `call_impl` dispatcher.
//! - [`tests`]: unit tests covering argv, PowerShell UTF-8 prefix,
//!   remote login shell command, non-TTY wrapper, remote shell probe,
//!   shell login args, and the contextual description stability test.

pub(super) mod local;
pub(super) mod remote;
pub(super) mod response;
pub(super) mod shell_helpers;
pub(super) mod tests;
pub(crate) mod tool;
pub(super) mod types;

pub use tool::ExecCommandTool;
