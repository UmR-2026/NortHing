//! Model-facing remote command execution runtime.
//!
//! This mirrors the local `terminal_core::ExecProcessManager` semantics for SSH
//! workspaces while keeping tool-owned command sessions separate from UI
//! terminal sessions.
//!
//! Module structure (post-R72b split):
//! - `types`: public request/response/error/lifecycle types
//! - `process`: RemoteExecProcess + Drop + spawn_* + remote_pipe_owner + remote_pty_owner
//! - `manager`: RemoteExecProcessManager + Default + global accessor
//! - `output`: OutputState + CollectedOutput + HeadTailText + utility fns

use std::sync::{Arc, OnceLock};

pub use self::manager::{global_remote_exec_process_manager, RemoteExecProcessManager};
pub use self::process::RemoteExecProcess;
pub use self::types::{
    RemoteExecCommandRequest, RemoteExecCommandResponse, RemoteExecControlAction,
    RemoteExecControlOrigin, RemoteExecControlRequest, RemoteExecError, RemoteExecProcessLifecycleEvent,
    RemoteExecProcessLifecycleStatus, RemoteExecResult, RemoteExecSessionCompletion,
    RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus, RemoteSendStdinRequest,
    RemoteWriteStdinRequest,
};

mod manager;
mod output;
mod process;
mod types;
