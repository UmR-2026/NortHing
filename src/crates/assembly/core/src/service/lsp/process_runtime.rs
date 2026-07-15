//! LSP server process background tasks.
//!
//! Three tokio tasks spawned by `LspServerProcess::spawn`:
//!
//! - `start_read_task` -- reads framed JSON-RPC messages from the child
//!   stdout, dispatches `Response` to the pending-request oneshot channel
//!   and forwards `Notification` / `Request` to the notification handler.
//!   Invokes the `crash_callback` when the stream ends.
//! - `start_stderr_task` -- reads the child stderr line-by-line to keep
//!   the OS pipe buffer from filling, and applies heuristic filtering
//!   for build-script errors, missing CMake / Spectre mitigation, panics,
//!   and noisy `compiling` / `building` lines.
//! - `start_notification_task` -- consumes the notification channel and
//!   translates the LSP server's push notifications:
//!   `$/progress` -> `progress_callback`,
//!   `textDocument/publishDiagnostics` -> `diagnostics_callback`,
//!   `window/logMessage` and `window/showMessage` -> tracing logs,
//!   server-initiated `Request` messages (`window/workDoneProgress/create`,
//!   `client/registerCapability`, `workspace/configuration`) -> canned
//!   `Result = null` / `[]` JSON-RPC responses, with method-not-found
//!   fallback for unknown methods.

use std::time::Duration;
use tokio::io::BufReader;
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::process::LspServerProcess;
use super::protocol::{read_message, write_message};
use super::types::{JsonRpcError, JsonRpcMessage, JsonRpcResponse};

impl LspServerProcess {
    /// Starts the message reader task.
    pub(super) async fn start_read_task(&self, stdout: ChildStdout) {
        let pending_requests = self.pending_requests.clone();
        let notification_tx = self.notification_tx.clone();
        let id = self.id.clone();
        let crash_callback = self.crash_callback.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut consecutive_timeouts = 0;
            const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

            loop {
                match timeout(Duration::from_secs(30), read_message(&mut reader)).await {
                    Ok(Ok(message)) => {
                        consecutive_timeouts = 0;

                        match &message {
                            JsonRpcMessage::Response(response) => {
                                let request_id = response.id;
                                let mut pending = pending_requests.write().await;

                                if let Some(sender) = pending.remove(&request_id) {
                                    let _ = sender.send(response.clone());
                                } else {
                                    warn!("[{}] Received response for unknown request ID: {}", id, request_id);
                                }
                            }
                            JsonRpcMessage::Notification(_) => {
                                if let Err(e) = notification_tx.send(message) {
                                    error!("[{}] Failed to send notification: {}", id, e);
                                    break;
                                }
                            }
                            JsonRpcMessage::Request(_req) => {
                                if let Err(e) = notification_tx.send(message) {
                                    error!("[{}] Failed to send request: {}", id, e);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("[{}] Failed to read message: {}", id, e);
                        error!(
                            "[{}] This usually means the LSP server is outputting non-protocol data to stdout",
                            id
                        );
                        break;
                    }
                    Err(_) => {
                        consecutive_timeouts += 1;

                        if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                            warn!(
                                "[{}] No LSP messages for {}s (this is normal if idle)",
                                id,
                                30 * MAX_CONSECUTIVE_TIMEOUTS
                            );

                            consecutive_timeouts = 0;
                        }
                    }
                }
            }

            error!("LSP server read task ended abnormally: {}", id);

            {
                let mut pending = pending_requests.write().await;
                let count = pending.len();
                if count > 0 {
                    warn!("Dropping {} pending request(s) for server {}", count, id);
                }
                pending.clear();
            }

            if let Some(callback) = crash_callback {
                error!("Invoking crash callback - server connection lost: {}", id);
                callback(id.clone());
            }
        });
    }

    /// Starts the stderr reader task.
    ///
    /// This task continuously reads the LSP server's stderr output to prevent the pipe buffer from
    /// filling up and blocking the process.
    /// The LSP protocol specifies using stdout for protocol communication; stderr is used for the
    /// server's diagnostic logs.
    pub(super) async fn start_stderr_task(&self, stderr: ChildStderr) {
        let id = self.id.clone();

        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            let mut line_count = 0;
            let mut error_count = 0;
            let mut warn_count = 0;

            let mut missing_cmake = false;
            let mut missing_spectre = false;
            let mut build_script_errors = std::collections::HashSet::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            line_count += 1;

                            let lower = trimmed.to_lowercase();

                            if lower.contains("missing dependency: cmake")
                                || (lower.contains("failed to spawn") && lower.contains("cmake"))
                            {
                                if !missing_cmake {
                                    missing_cmake = true;
                                    warn!("[{}] Missing build dependency: CMake not installed or not in PATH", id);
                                    info!("[{}] Tip: Some Rust crates require CMake to compile C/C++ code. Download: https://cmake.org/download/", id);
                                }
                                continue;
                            }

                            if lower.contains("no spectre-mitigated libs") {
                                if !missing_spectre {
                                    missing_spectre = true;
                                    warn!("[{}] Missing build dependency: MSVC Spectre mitigation libraries not installed", id);
                                    info!("[{}] Tip: Some Rust crates require MSVC Spectre libraries. Install via Visual Studio Installer", id);
                                }
                                continue;
                            }

                            if lower.contains("failed to run custom build command") {
                                if let Some(start) = trimmed.find("for `") {
                                    if let Some(end) = trimmed[start + 5..].find('`') {
                                        let package = &trimmed[start + 5..start + 5 + end];
                                        if build_script_errors.insert(package.to_string()) {
                                            warn!("[{}] Build script failed for package: {} (LSP may still work but code analysis accuracy may be affected)", id, package);
                                        }
                                    }
                                }
                                continue;
                            }

                            if lower.contains("compiling")
                                || lower.contains("building")
                                || lower.contains("cargo:rerun-if")
                            {
                                continue;
                            }

                            if lower.contains("panic") {
                                error_count += 1;
                                if error_count <= 3 {
                                    debug!("[{}] Build script panic: {}", id, trimmed);
                                }
                                continue;
                            }

                            if lower.contains("error") || lower.contains("fatal") {
                                error_count += 1;

                                if error_count <= 5 {
                                    error!("[{}] stderr: {}", id, trimmed);
                                } else if error_count % 10 == 0 {
                                    error!("[{}] stderr: ... (omitted {} errors)", id, error_count);
                                }
                            } else if lower.contains("warn") || lower.contains("warning") {
                                warn_count += 1;

                                if warn_count <= 10 {
                                    warn!("[{}] stderr: {}", id, trimmed);
                                } else if warn_count % 100 == 0 {
                                    warn!("[{}] stderr: ... (omitted {} warnings)", id, warn_count);
                                }
                            } else {
                                if line_count <= 5 || line_count % 1000 == 0 {
                                    debug!("[{}] stderr: {}", id, trimmed);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read stderr from {}: {}", id, e);
                        break;
                    }
                }
            }

            if line_count > 0 || error_count > 0 || warn_count > 0 {
                info!(
                    "LSP server stderr task ended: {} (read {} lines, {} errors, {} warnings)",
                    id, line_count, error_count, warn_count
                );

                if !build_script_errors.is_empty() {
                    warn!(
                        "[{}] {} package(s) had build script failures, but LSP service is still running",
                        id,
                        build_script_errors.len()
                    );
                }

                if missing_cmake || missing_spectre {
                    info!(
                        "[{}] Tip: Installing missing dependencies may improve code analysis accuracy",
                        id
                    );
                }
            }
        });
    }

    /// Starts the notification handler task.
    pub(super) async fn start_notification_task(&self, mut notification_rx: mpsc::UnboundedReceiver<JsonRpcMessage>) {
        let id = self.id.clone();
        let progress_callback = self.progress_callback.clone();
        let token_create_callback = self.token_create_callback.clone();
        let diagnostics_callback = self.diagnostics_callback.clone();
        let stdin = self.stdin.clone();

        tokio::spawn(async move {
            while let Some(message) = notification_rx.recv().await {
                match message {
                    JsonRpcMessage::Notification(notif) => match notif.method.as_str() {
                        "$/progress" => {
                            if let Some(params) = &notif.params {
                                let token = params
                                    .get("token")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                if let Some(value) = params.get("value") {
                                    if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
                                        match kind {
                                            "begin" => {
                                                let title = value.get("title").and_then(|t| t.as_str()).unwrap_or("");
                                                info!("[{}] Indexing started: {}", id, title);

                                                if let Some(ref callback) = progress_callback {
                                                    callback(
                                                        "begin".to_string(),
                                                        token.clone(),
                                                        Some(0),
                                                        title.to_string(),
                                                    );
                                                }
                                            }
                                            "report" => {
                                                let percentage = value.get("percentage").and_then(|p| p.as_u64());
                                                let message =
                                                    value.get("message").and_then(|m| m.as_str()).unwrap_or("");

                                                if let Some(ref callback) = progress_callback {
                                                    callback(
                                                        "report".to_string(),
                                                        token.clone(),
                                                        percentage.map(|p| p as u32),
                                                        message.to_string(),
                                                    );
                                                }
                                            }
                                            "end" => {
                                                let message =
                                                    value.get("message").and_then(|m| m.as_str()).unwrap_or("");
                                                info!("[{}] Indexing completed: {}", id, message);

                                                if let Some(ref callback) = progress_callback {
                                                    callback(
                                                        "end".to_string(),
                                                        token.clone(),
                                                        Some(100),
                                                        message.to_string(),
                                                    );
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        "textDocument/publishDiagnostics" => {
                            if let Some(params) = &notif.params {
                                if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                                    if let Some(diagnostics_arr) = params.get("diagnostics").and_then(|d| d.as_array())
                                    {
                                        let diags: Vec<serde_json::Value> = diagnostics_arr.clone();

                                        debug!("[{}] Diagnostics: {} items for {}", id, diags.len(), uri);

                                        if let Some(callback) = &diagnostics_callback {
                                            callback(uri.to_string(), diags);
                                        }
                                    }
                                }
                            }
                        }
                        "window/logMessage" => {
                            if let Some(params) = &notif.params {
                                let msg_type = params.get("type").and_then(|t| t.as_u64()).unwrap_or(3);
                                if let Some(msg) = params.get("message").and_then(|m| m.as_str()) {
                                    match msg_type {
                                        1 => error!("[{}] Server log: {}", id, msg),
                                        2 => warn!("[{}] Server log: {}", id, msg),
                                        3 => info!("[{}] Server log: {}", id, msg),
                                        4 => debug!("[{}] Server log: {}", id, msg),
                                        _ => debug!("[{}] Server log: {}", id, msg),
                                    }
                                }
                            }
                        }
                        "window/showMessage" => {
                            if let Some(params) = &notif.params {
                                let msg_type = params.get("type").and_then(|t| t.as_u64()).unwrap_or(3);
                                if let Some(msg) = params.get("message").and_then(|m| m.as_str()) {
                                    match msg_type {
                                        1 => error!("[{}] Server message: {}", id, msg),
                                        2 => warn!("[{}] Server message: {}", id, msg),
                                        3 => info!("[{}] Server message: {}", id, msg),
                                        4 => debug!("[{}] Server message: {}", id, msg),
                                        _ => info!("[{}] Server message: {}", id, msg),
                                    }
                                }
                            }
                        }
                        _ => {}
                    },

                    JsonRpcMessage::Request(req) => match req.method.as_str() {
                        "window/workDoneProgress/create" => {
                            if let Some(params) = &req.params {
                                if let Some(token) = params.get("token") {
                                    let token_str = token.as_str().unwrap_or("unknown").to_string();

                                    if let Some(ref callback) = token_create_callback {
                                        callback(token_str);
                                    }
                                }
                            }

                            let response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(serde_json::Value::Null),
                                error: None,
                            };

                            let response_message = JsonRpcMessage::Response(response);
                            let mut stdin_lock = stdin.write().await;
                            if let Err(e) = write_message(&mut stdin_lock, &response_message).await {
                                error!("[{}] Failed to send workDoneProgress/create response: {}", id, e);
                            }
                        }
                        "client/registerCapability" => {
                            let response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(serde_json::Value::Null),
                                error: None,
                            };

                            let response_message = JsonRpcMessage::Response(response);
                            let mut stdin_lock = stdin.write().await;
                            if let Err(e) = write_message(&mut stdin_lock, &response_message).await {
                                error!("[{}] Failed to send registerCapability response: {}", id, e);
                            }
                        }
                        "workspace/configuration" => {
                            let response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(serde_json::json!([])),
                                error: None,
                            };

                            let response_message = JsonRpcMessage::Response(response);
                            let mut stdin_lock = stdin.write().await;
                            if let Err(e) = write_message(&mut stdin_lock, &response_message).await {
                                error!("[{}] Failed to send configuration response: {}", id, e);
                            }
                        }
                        _ => {
                            warn!("[{}] Unhandled server request: {}", id, req.method);

                            let response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32601,
                                    message: format!("Method not supported: {}", req.method),
                                    data: None,
                                }),
                            };

                            let response_message = JsonRpcMessage::Response(response);
                            let mut stdin_lock = stdin.write().await;
                            if let Err(e) = write_message(&mut stdin_lock, &response_message).await {
                                error!("[{}] Failed to send error response: {}", id, e);
                            }
                        }
                    },
                    _ => {}
                }
            }

            info!("LSP notification task ended: {}", id);
        });
    }
}
