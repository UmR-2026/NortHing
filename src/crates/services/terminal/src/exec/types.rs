//! Type definitions for the `exec` runtime.
//!
//! All public DTOs/enums (`ExecCommandRequest`, `ExecCommandResponse`,
//! `ExecControlAction`, `ExecControlOrigin`, `ExecControlRequest`,
//! `WriteStdinRequest`, `SendStdinRequest`, `ExecProcessLifecycleEvent`,
//! `ExecProcessLifecycleStatus`, `ExecSessionCompletion`,
//! `ExecSessionCompletionSource`, `ExecSessionCompletionStatus`), the
//! `ExecProcessManager` storage struct, and the internal helper types
//! (`ExecSessionEntry`, `CompletedExecSession`, `ExecProcess`, `Terminator`,
//! `PtyKeepAlive`, `WindowsPipeJobHandle`, `WindowsPipeJob`,
//! `LocalPipeControlState`, `OutputState`, `OutputInner`, `OutputCursor`,
//! `HeadTailText`) live here. Implementation blocks for the manager and
//! output/headtail helpers live in `manager` and `output` siblings
//! respectively; this file only carries the data definitions.
//!
//! Mavis R22e promoted internal structs to `pub(crate)` so sibling files
//! (manager/output/platform) can use them.

pub(crate) const MAX_EXEC_SESSIONS: usize = 64;
pub(crate) const MAX_COMPLETED_EXEC_SESSIONS: usize = 64;
pub(crate) const MAX_RETAINED_OUTPUT_BYTES: usize = 1024 * 1024;
pub(crate) const DEFAULT_YIELD_TIME_MS: u64 = 10_000;
pub(crate) const PTY_EXIT_DRAIN_TIMEOUT_MS: u64 = 500;

use portable_pty::{MasterPty, SlavePty};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ExecCommandRequest {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub tty: bool,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
    pub lifecycle_tx: Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>>,
    pub output_capture_tx: Option<mpsc::UnboundedSender<String>>,
}

#[derive(Debug, Clone)]
pub struct WriteStdinRequest {
    pub session_id: i32,
    pub chars: String,
    pub append_enter: bool,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SendStdinRequest {
    pub session_id: i32,
    pub chars: String,
    pub append_enter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecControlAction {
    Interrupt,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecControlOrigin {
    ModelTool,
    OutOfBand,
}

#[derive(Debug, Clone)]
pub struct ExecControlRequest {
    pub session_id: i32,
    pub action: ExecControlAction,
    pub origin: ExecControlOrigin,
    pub yield_time_ms: Option<u64>,
    pub max_output_chars: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecSessionCompletionStatus {
    Exited,
    Interrupted,
    Killed,
    Pruned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecSessionCompletionSource {
    Process,
    OutOfBandControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecSessionCompletion {
    pub status: ExecSessionCompletionStatus,
    pub source: ExecSessionCompletionSource,
}

#[derive(Debug, Clone)]
pub struct ExecCommandResponse {
    pub chunk_id: String,
    pub wall_time_seconds: f64,
    pub output: String,
    pub session_id: Option<i32>,
    pub exit_code: Option<i32>,
    pub original_output_chars: usize,
    pub completion: Option<ExecSessionCompletion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecProcessLifecycleStatus {
    Running,
    Exited,
    Interrupted,
    Killed,
    Pruned,
}

#[derive(Debug, Clone)]
pub struct ExecProcessLifecycleEvent {
    pub session_id: i32,
    pub status: ExecProcessLifecycleStatus,
    pub exit_code: Option<i32>,
}

pub struct ExecProcessManager {
    pub(crate) sessions: Mutex<HashMap<i32, ExecSessionEntry>>,
    pub(crate) completed_sessions: Mutex<HashMap<i32, CompletedExecSession>>,
}

impl Default for ExecProcessManager {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            completed_sessions: Mutex::new(HashMap::new()),
        }
    }
}

pub(crate) struct ExecSessionEntry {
    pub(crate) process: Arc<ExecProcess>,
    pub(crate) tty: bool,
    pub(crate) cursor: OutputCursor,
    pub(crate) last_used: tokio::time::Instant,
    pub(crate) lifecycle_tx: Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>>,
}

#[derive(Clone)]
pub(crate) struct CompletedExecSession {
    pub(crate) output: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) original_output_chars: usize,
    pub(crate) completion: ExecSessionCompletion,
    pub(crate) completed_at: tokio::time::Instant,
}

pub(crate) struct ExecProcess {
    pub(crate) output: Arc<OutputState>,
    pub(crate) writer: Option<mpsc::Sender<Vec<u8>>>,
    pub(crate) terminator: StdMutex<Option<Terminator>>,
    pub(crate) out_of_band_control_action: StdMutex<Option<ExecControlAction>>,
    pub(crate) helper_tasks: StdMutex<Vec<JoinHandle<()>>>,
    pub(crate) pty_handles: Arc<StdMutex<Option<PtyKeepAlive>>>,
    #[cfg(windows)]
    pub(crate) pipe_job: Option<WindowsPipeJobHandle>,
}

pub(crate) enum Terminator {
    Pty(Box<dyn portable_pty::ChildKiller + Send + Sync>),
    Pipe(mpsc::Sender<ExecControlAction>),
}

pub(crate) struct PtyKeepAlive {
    pub(crate) _master: Box<dyn MasterPty + Send>,
    pub(crate) _slave: Option<Box<dyn SlavePty + Send>>,
}

#[cfg(windows)]
pub(crate) type WindowsPipeJobHandle = Arc<StdMutex<Option<WindowsPipeJob>>>;

#[cfg(windows)]
pub(crate) struct WindowsPipeJob {
    pub(crate) _job: win32job::Job,
    pub(crate) _pid: u32,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum LocalPipeControlState {
    InterruptGrace { deadline: tokio::time::Instant },
}

#[cfg(unix)]
impl LocalPipeControlState {
    fn deadline(self) -> tokio::time::Instant {
        match self {
            Self::InterruptGrace { deadline } => deadline,
        }
    }
}

pub(crate) struct OutputState {
    pub(crate) inner: Mutex<OutputInner>,
    pub(crate) notify: Notify,
    pub(crate) output_capture_tx: Option<mpsc::UnboundedSender<String>>,
}

pub(crate) struct OutputInner {
    pub(crate) chunks: VecDeque<(u64, Vec<u8>)>,
    pub(crate) next_seq: u64,
    pub(crate) retained_bytes: usize,
    pub(crate) closed: bool,
    pub(crate) exit_code: Option<i32>,
}

#[derive(Clone)]
pub(crate) struct OutputCursor {
    pub(crate) next_seq: u64,
}

pub(crate) struct HeadTailText {
    pub(crate) max_chars: usize,
    pub(crate) head_budget: usize,
    pub(crate) tail_budget: usize,
    pub(crate) head: String,
    pub(crate) tail: VecDeque<char>,
    pub(crate) head_chars: usize,
    pub(crate) tail_chars: usize,
    pub(crate) omitted_chars: usize,
    pub(crate) total_chars: usize,
}
