//! Model-facing command execution runtime.
//!
//! This runtime is intentionally separate from terminal sessions. Each
//! `exec_command` starts a fresh local process; a session id is only retained
//! while that process is still running so later calls can poll or write stdin.

use std::sync::{Arc, OnceLock};

pub(crate) mod manager;
pub(crate) mod output;
pub(crate) mod platform;
pub mod types;

pub use types::ExecProcessManager;

const DEFAULT_YIELD_TIME_MS: u64 = 10_000;
pub(crate) const MAX_RETAINED_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_EXEC_SESSIONS: usize = 64;
const MAX_COMPLETED_EXEC_SESSIONS: usize = 64;
#[cfg(unix)]
const PIPE_INTERRUPT_GRACE_TIMEOUT_MS: u64 = 2_000;
pub(crate) const PTY_EXIT_DRAIN_TIMEOUT_MS: u64 = 500;
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
pub(crate) const PIPE_JOB_CLOSE_WAIT_MS: u64 = 2_000;

static GLOBAL_EXEC_MANAGER: OnceLock<Arc<ExecProcessManager>> = OnceLock::new();

pub fn global_exec_process_manager() -> Arc<ExecProcessManager> {
    GLOBAL_EXEC_MANAGER
        .get_or_init(|| Arc::new(ExecProcessManager::default()))
        .clone()
}
