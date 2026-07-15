//! Public request/response/error/lifecycle types for remote SSH command execution.

use crate::remote_ssh::SSHConnectionManager;
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) const DEFAULT_YIELD_TIME_MS: u64 = 10_000;
pub(crate) const MAX_RETAINED_OUTPUT_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_REMOTE_EXEC_SESSIONS: usize = 64;
pub(crate) const MAX_COMPLETED_REMOTE_EXEC_SESSIONS: usize = 64;
pub(crate) const REMOTE_INTERRUPT_GRACE_TIMEOUT_MS: u64 = 3_000;
pub(crate) const REMOTE_CONTROL_DRAIN_TIMEOUT_MS: u64 = 500;

#[derive(Clone)]
pub struct RemoteExecCommandRequest {
    pub ssh_manager: SSHConnectionManager,
    pub connection_id: String,
    pub command: String,
    pub tty: bool,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
    pub lifecycle_tx: Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>>,
    pub output_capture_tx: Option<mpsc::UnboundedSender<String>>,
}

#[derive(Debug, Clone)]
pub struct RemoteWriteStdinRequest {
    pub session_id: i32,
    pub chars: String,
    pub append_enter: bool,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RemoteSendStdinRequest {
    pub session_id: i32,
    pub chars: String,
    pub append_enter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExecControlAction {
    Interrupt,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExecControlOrigin {
    ModelTool,
    OutOfBand,
}

#[derive(Debug, Clone)]
pub struct RemoteExecControlRequest {
    pub session_id: i32,
    pub action: RemoteExecControlAction,
    pub origin: RemoteExecControlOrigin,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExecSessionCompletionStatus {
    Exited,
    Interrupted,
    Killed,
    Pruned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExecSessionCompletionSource {
    Process,
    OutOfBandControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteExecSessionCompletion {
    pub status: RemoteExecSessionCompletionStatus,
    pub source: RemoteExecSessionCompletionSource,
}

#[derive(Debug, Clone)]
pub struct RemoteExecCommandResponse {
    pub chunk_id: String,
    pub wall_time_seconds: f64,
    pub output: String,
    pub session_id: Option<i32>,
    pub exit_code: Option<i32>,
    pub original_output_chars: usize,
    pub completion: Option<RemoteExecSessionCompletion>,
}

pub type RemoteExecResult<T> = std::result::Result<T, RemoteExecError>;

#[derive(Debug, thiserror::Error)]
pub enum RemoteExecError {
    #[error("session not found: {0}")]
    SessionNotFound(i32),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<String> for RemoteExecError {
    fn from(value: String) -> Self {
        Self::Other(anyhow::anyhow!(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExecProcessLifecycleStatus {
    Running,
    Exited,
    Interrupted,
    Killed,
    Pruned,
}

#[derive(Debug, Clone)]
pub struct RemoteExecProcessLifecycleEvent {
    pub session_id: i32,
    pub status: RemoteExecProcessLifecycleStatus,
    pub exit_code: Option<i32>,
}

// Suppress unused warning for Arc import
#[allow(dead_code)]
fn _ensure_arc_used(_: Arc<()>) {}
