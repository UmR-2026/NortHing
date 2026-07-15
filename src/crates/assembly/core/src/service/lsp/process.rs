//! LSP server process management (R44e facade).
//!
//! Manages the lifecycle of a single LSP server process. Mavis take-over
//! (impl-block god-impl sub-domain split R44e) splits the 1087-line
//! `process.rs` into facade + callbacks + spawn + command + runtime + protocol.
//!
//! - `process_callbacks.rs` -- callback type aliases
//!   (`CrashCallback`, `ProgressCallback`, `TokenCreateCallback`,
//!   `DiagnosticsCallback`).
//! - `process_spawn.rs` -- `spawn` lifecycle method that forks the child
//!   process, captures stdio, and starts the three background tokio tasks.
//! - `process_command.rs` -- `detect_runtime_type` + `build_command`
//!   (cross-platform command construction: `.exe` / `.bat` / `.sh` /
//!   `.js`, plus Git Bash / WSL discovery on Windows).
//! - `process_runtime.rs` -- three `start_*_task` background tasks:
//!   `start_read_task` (stdout JSON-RPC dispatch), `start_stderr_task`
//!   (stderr log filtering), `start_notification_task` (server-pushed
//!   `$/progress`, `textDocument/publishDiagnostics`, `window/*Message`,
//!   `window/workDoneProgress/create`, `client/registerCapability`,
//!   `workspace/configuration`).
//! - `process_protocol.rs` -- outbound LSP protocol operations:
//!   `send_request`, `send_notification`, `initialize`, `shutdown`,
//!   `get_capabilities`, `is_alive`.
//!
//! impl+struct kept in the facade so private field access works through
//! sibling `impl LspServerProcess` blocks. Internal callback types are
//! re-exported here so external code (`manager.rs`) keeps using
//! `super::process::{CrashCallback, LspServerProcess, ...}` paths.

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::process::{Child, ChildStdin};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::debug;

use super::types::{JsonRpcMessage, JsonRpcResponse};

pub use super::process_callbacks::{CrashCallback, DiagnosticsCallback, ProgressCallback, TokenCreateCallback};

/// LSP server process.
pub struct LspServerProcess {
    /// Plugin ID.
    pub id: String,
    /// Child process.
    pub(super) child: Arc<RwLock<Child>>,
    /// Standard input.
    pub(super) stdin: Arc<RwLock<ChildStdin>>,
    /// Request ID counter.
    pub(super) request_id: Arc<AtomicU64>,
    /// Pending requests waiting for a response.
    pub(super) pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    /// Notification sender.
    pub(super) notification_tx: mpsc::UnboundedSender<JsonRpcMessage>,
    /// Server capabilities.
    pub(super) capabilities: Arc<RwLock<Option<serde_json::Value>>>,
    /// Crash callback.
    pub(super) crash_callback: Option<CrashCallback>,
    /// Progress callback.
    pub(super) progress_callback: Option<ProgressCallback>,
    /// Token creation callback.
    pub(super) token_create_callback: Option<TokenCreateCallback>,
    /// Diagnostics callback.
    pub(super) diagnostics_callback: Option<DiagnosticsCallback>,
}

impl Drop for LspServerProcess {
    fn drop(&mut self) {
        debug!("Dropping LSP server process: {}", self.id);
    }
}
