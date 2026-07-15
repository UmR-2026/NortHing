//! `TerminalApi` orchestrator.
//!
//! Standalone sibling — `struct TerminalApi` + `impl TerminalApi` (19 methods).
//! DTOs and `WsRequest`/`WsResponse` enums live in `types.rs`.

use std::sync::Arc;

use std::time::Duration;

use crate::config::TerminalConfig;
use crate::events::TerminalEvent;
use crate::session::{init_session_manager, session_manager, ExecuteOptions, SessionManager};
use crate::shell::ShellDetector;

use super::types::{
    CloseSessionRequest, CreateSessionRequest, ExecuteCommandRequest, ExecuteCommandResponse, GetHistoryRequest,
    GetHistoryResponse, ShellInfo,
};

/// Terminal API service - main interface for external consumers
pub struct TerminalApi {
    /// Session manager (uses singleton)
    session_manager: Arc<SessionManager>,
}

impl TerminalApi {
    /// Create a new Terminal API instance
    ///
    /// This will initialize the global SessionManager singleton if not already initialized.
    /// If the singleton is already initialized, it will use the existing instance.
    ///
    /// # Errors
    ///
    /// Returns an error only when the singleton is not yet initialized **and**
    /// concurrent initialization also fails (the underlying `OnceCell` rejects
    /// double-initialization). The single-threaded race that the previous
    /// `is_session_manager_initialized` + `get_session_manager` pair used to
    /// guard against is impossible on a single `OnceCell`, so the panic was
    /// replaced with a race-safe fallback path.
    pub async fn new(config: TerminalConfig) -> crate::TerminalResult<Self> {
        let session_manager = if let Some(manager) = session_manager() {
            manager
        } else {
            // Race fallback: another thread may have initialized the singleton
            // between our `get_session_manager()` check and `init_session_manager`.
            // On a concurrent win, accept the existing manager; only fail if
            // nothing is actually present.
            match init_session_manager(config).await {
                Ok(manager) => manager,
                Err(_) => session_manager().ok_or_else(|| {
                    crate::TerminalError::Session(
                        "SessionManager initialization failed and no singleton present".to_string(),
                    )
                })?,
            }
        };

        Ok(Self { session_manager })
    }

    /// Create a Terminal API instance from an existing SessionManager
    pub fn from_manager(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// Create a Terminal API instance using the global singleton
    ///
    /// Returns an error if the singleton has not been initialized.
    pub fn from_singleton() -> crate::TerminalResult<Self> {
        let session_manager = session_manager()
            .ok_or_else(|| crate::TerminalError::Session("SessionManager not initialized".to_string()))?;

        Ok(Self { session_manager })
    }

    /// Get available shells
    pub fn available_shells(&self) -> Vec<ShellInfo> {
        ShellDetector::detect_available_shells()
            .into_iter()
            .map(|shell| ShellInfo {
                shell_type: shell.shell_type,
                name: shell.display_name,
                path: shell.path.to_string_lossy().to_string(),
                version: shell.version,
                available: true,
            })
            .collect()
    }

    /// Create a new terminal session
    pub async fn create_session(
        &self,
        request: CreateSessionRequest,
    ) -> crate::TerminalResult<super::types::SessionResponse> {
        let session = self
            .session_manager
            .create_session(
                request.session_id,
                request.name,
                request.shell_type,
                request.working_directory,
                request.env,
                request.cols,
                request.rows,
                request.source,
            )
            .await?;

        Ok(super::types::SessionResponse::from(session))
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> crate::TerminalResult<super::types::SessionResponse> {
        let session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| crate::TerminalError::SessionNotFound(session_id.to_string()))?;

        Ok(super::types::SessionResponse::from(session))
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> crate::TerminalResult<Vec<super::types::SessionResponse>> {
        let sessions = self.session_manager.list_sessions().await;

        Ok(sessions.into_iter().map(super::types::SessionResponse::from).collect())
    }

    /// Write data to a terminal session
    pub async fn write(&self, request: super::types::WriteRequest) -> crate::TerminalResult<()> {
        self.session_manager
            .write(&request.session_id, request.data.as_bytes())
            .await
    }

    /// Resize a terminal session
    pub async fn resize(&self, request: super::types::ResizeRequest) -> crate::TerminalResult<()> {
        self.session_manager
            .resize(&request.session_id, request.cols, request.rows)
            .await
    }

    /// Send a signal to a terminal session
    pub async fn signal(&self, request: super::types::SignalRequest) -> crate::TerminalResult<()> {
        self.session_manager.signal(&request.session_id, &request.signal).await
    }

    /// Close a terminal session
    pub async fn close_session(&self, request: CloseSessionRequest) -> crate::TerminalResult<()> {
        self.session_manager
            .close_session(&request.session_id, request.immediate.unwrap_or(false))
            .await
    }

    /// Acknowledge data received by frontend
    pub async fn acknowledge_data(&self, request: super::types::AcknowledgeRequest) -> crate::TerminalResult<()> {
        self.session_manager
            .acknowledge_data(&request.session_id, request.char_count)
            .await
    }

    /// Get output history for a session
    ///
    /// This returns the historical output data that was buffered on the backend.
    /// Useful for recovering terminal state when reconnecting.
    pub async fn history(&self, request: GetHistoryRequest) -> crate::TerminalResult<GetHistoryResponse> {
        let session = self
            .session_manager
            .get_session(&request.session_id)
            .await
            .ok_or_else(|| crate::TerminalError::SessionNotFound(request.session_id.to_string()))?;

        let data = session.history();
        let history_size = session.history_size();

        Ok(GetHistoryResponse {
            session_id: request.session_id,
            data,
            history_size,
            cols: session.cols,
            rows: session.rows,
        })
    }

    /// Execute a command in a session and wait for completion
    ///
    /// This function sends a command to the terminal, waits for it to complete
    /// using shell integration, and returns the output and exit code.
    pub async fn execute_command(
        &self,
        request: ExecuteCommandRequest,
    ) -> crate::TerminalResult<ExecuteCommandResponse> {
        let options = ExecuteOptions {
            timeout: request.timeout_ms.map(Duration::from_millis),
            prevent_history: request.prevent_history.unwrap_or(true),
        };

        let result = self
            .session_manager
            .execute_command_with_options(&request.session_id, &request.command, options)
            .await?;

        Ok(ExecuteCommandResponse::from(result))
    }

    /// Check if a session has shell integration enabled
    pub async fn has_shell_integration(&self, session_id: &str) -> bool {
        self.session_manager.has_shell_integration(session_id).await
    }

    /// Execute a command and return a stream of events for real-time output
    ///
    /// This function provides streaming command execution, allowing callers
    /// to receive output as it arrives rather than waiting for completion.
    pub fn execute_command_stream(&self, request: ExecuteCommandRequest) -> super::types::CommandStream {
        let options = ExecuteOptions {
            timeout: request.timeout_ms.map(Duration::from_millis),
            prevent_history: request.prevent_history.unwrap_or(true),
        };

        self.session_manager
            .execute_command_stream_with_options(request.session_id, request.command, options)
    }

    /// Send a command to a session without waiting for completion
    ///
    /// This function waits for the session to be active, then sends a command
    /// to the terminal. Unlike `execute_command`, it does NOT require shell
    /// integration and does NOT wait for command completion or capture output.
    ///
    /// This is useful for:
    /// - Shells that don't support shell integration (e.g., cmd)
    /// - Startup commands where you don't need the result
    /// - Fire-and-forget command execution
    pub async fn send_command(&self, request: super::types::SendCommandRequest) -> crate::TerminalResult<()> {
        self.session_manager
            .send_command(&request.session_id, &request.command)
            .await
    }

    /// Subscribe to raw PTY output of a specific session.
    ///
    /// Returns a receiver that yields raw output strings as they arrive.
    /// The channel closes when the session is destroyed.
    pub fn subscribe_session_output(&self, session_id: &str) -> tokio::sync::mpsc::Receiver<String> {
        self.session_manager.subscribe_session_output(session_id)
    }

    /// Subscribe to terminal events
    pub fn subscribe_events(&self) -> tokio::sync::mpsc::Receiver<TerminalEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);

        let emitter = self.session_manager.event_emitter();

        // Forward events
        tokio::spawn(async move {
            loop {
                if let Some(event) = emitter.recv().await {
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
            }
        });

        rx
    }

    /// Shutdown all sessions
    pub async fn shutdown_all(&self) {
        self.session_manager.shutdown_all().await;
    }

    /// Get the underlying session manager
    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }
}
