//! `LspServerProcess::spawn` lifecycle entry point.
//!
//! Forks the child process, captures stdio (stdin / stdout / stderr),
//! constructs the `LspServerProcess` instance, and starts the three
//! background tokio tasks (stdout reader, stderr filter, notification
//! dispatcher). Command construction helpers (`detect_runtime_type`,
//! `build_command`) live in `process_command.rs`.

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::types::ServerConfig;

use super::process::{CrashCallback, DiagnosticsCallback, LspServerProcess, ProgressCallback, TokenCreateCallback};

impl LspServerProcess {
    /// Spawns a new LSP server process.
    pub async fn spawn(
        id: String,
        server_bin: PathBuf,
        config: &ServerConfig,
        crash_callback: Option<CrashCallback>,
        progress_callback: Option<ProgressCallback>,
        token_create_callback: Option<TokenCreateCallback>,
        diagnostics_callback: Option<DiagnosticsCallback>,
    ) -> Result<Self> {
        info!("Spawning LSP server: {} at {:?}", id, server_bin);
        debug!("LSP config - args: {:?}, env: {:?}", config.args, config.env);

        if !server_bin.exists() {
            error!("LSP server binary not found: {:?}", server_bin);
            return Err(anyhow!("LSP server binary not found: {:?}", server_bin));
        }

        let runtime_type = Self::detect_runtime_type(config, &server_bin);
        debug!("Detected runtime type: {:?}", runtime_type);

        let mut cmd = Self::build_command(&runtime_type, &server_bin, config)?;

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            error!("Failed to spawn LSP server {}: {}", id, e);
            anyhow!("Failed to spawn LSP server {}: {}", id, e)
        })?;

        if let Some(pid) = child.id() {
            debug!("LSP server process started with PID: {}", pid);
        }

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;

        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let (notification_tx, notification_rx) = mpsc::unbounded_channel();

        let process = Self {
            id: id.clone(),
            child: Arc::new(RwLock::new(child)),
            stdin: Arc::new(RwLock::new(stdin)),
            request_id: Arc::new(AtomicU64::new(1)),
            pending_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            notification_tx,
            capabilities: Arc::new(RwLock::new(None)),
            crash_callback,
            progress_callback,
            token_create_callback,
            diagnostics_callback,
        };

        process.start_read_task(stdout).await;

        process.start_stderr_task(stderr).await;

        process.start_notification_task(notification_rx).await;

        info!("LSP server process spawned: {}", id);

        Ok(process)
    }
}
