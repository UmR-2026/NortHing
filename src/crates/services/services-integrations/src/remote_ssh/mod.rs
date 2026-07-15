//! Remote SSH service contracts.
//!
//! `northhing-core::service::remote_ssh` remains as the compatibility facade for
//! the legacy public path.

pub mod paths;
pub mod types;
pub mod workspace_registry;
#[cfg(feature = "workspace-search")]
pub mod workspace_search;

#[cfg(feature = "remote-ssh-concrete")]
pub mod manager;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_command_dispatch;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_handler;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_known_hosts;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_port_forward;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_remote_workspace;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_saved_connections;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_session;
// Sibling modules under `manager_session_lifecycle::SSHConnectionManager` are
// declared here (the parent) rather than inside `manager_session_lifecycle.rs`
// because `mod child;` inside a `foo.rs` facade resolves to `foo/child.rs` (a
// sub-directory), not to a same-directory sibling. See R42e memory for the
// underlying rustc E0583 rule.
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_session_lifecycle;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_sftp;
#[cfg(feature = "remote-ssh-concrete")]
pub mod manager_ssh_config;
#[cfg(feature = "remote-ssh-concrete")]
mod mgr_lifecycle_handlers;
#[cfg(feature = "remote-ssh-concrete")]
mod mgr_lifecycle_persist;
#[cfg(feature = "remote-ssh-concrete")]
mod mgr_lifecycle_state;
#[cfg(feature = "remote-ssh-concrete")]
mod password_vault;
#[cfg(feature = "remote-ssh-concrete")]
mod remote_exec;
#[cfg(feature = "remote-ssh-concrete")]
pub mod remote_fs;
#[cfg(feature = "remote-ssh-concrete")]
pub mod remote_terminal;

#[cfg(all(feature = "remote-ssh-concrete", test))]
pub mod manager_tests;

pub use paths::*;
pub use types::*;
pub use workspace_registry::*;

#[cfg(feature = "remote-ssh-concrete")]
pub use manager::SSHConnectionManager;
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_known_hosts::KnownHostEntry;
// `manager_handler::{HandlerError, SSHHandler}` are crate-private (`pub(crate)`) — see spec §1.2
// They are NOT re-exported outside the services-integrations crate.
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_port_forward::{PortForward, PortForwardDirection, PortForwardManager};
#[cfg(feature = "remote-ssh-concrete")]
pub use manager_session::PTYSession;
#[cfg(feature = "remote-ssh-concrete")]
pub use remote_exec::{
    global_remote_exec_process_manager, RemoteExecCommandRequest, RemoteExecCommandResponse, RemoteExecControlAction,
    RemoteExecControlOrigin, RemoteExecControlRequest, RemoteExecError, RemoteExecProcessLifecycleEvent,
    RemoteExecProcessLifecycleStatus, RemoteExecProcessManager, RemoteExecResult, RemoteExecSessionCompletion,
    RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus, RemoteSendStdinRequest,
    RemoteWriteStdinRequest,
};
#[cfg(feature = "remote-ssh-concrete")]
pub use remote_fs::RemoteFileService;
#[cfg(feature = "remote-ssh-concrete")]
pub use remote_terminal::{RemoteTerminalManager, RemoteTerminalSession, SessionStatus};
