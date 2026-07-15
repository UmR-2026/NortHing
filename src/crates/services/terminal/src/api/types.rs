//! Public API request/response DTOs + WebSocket envelope types.
//!
//! Standalone sibling — DTOs + their `From` impls + `WsRequest`/`WsResponse` enums.
//! `impl TerminalApi` (the orchestrator) lives in `api_impl.rs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::events::TerminalEvent;
use crate::session::{CommandCompletionReason, CommandExecuteResult, SessionSource, TerminalSession};
use crate::shell::ShellType;

// Re-export streaming types for external use
pub use crate::session::{CommandStream, CommandStreamEvent};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a terminal session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional session ID (if not provided, a UUID will be generated)
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    /// Optional session name
    pub name: Option<String>,
    /// Optional shell type
    #[serde(rename = "shellType")]
    pub shell_type: Option<ShellType>,
    /// Optional working directory
    #[serde(rename = "workingDirectory")]
    pub working_directory: Option<String>,
    /// Optional custom environment variables
    pub env: Option<HashMap<String, String>>,
    /// Optional terminal dimensions
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    /// Optional remote connection ID (for remote workspace sessions)
    #[serde(rename = "remoteConnectionId", skip_serializing_if = "Option::is_none")]
    pub remote_connection_id: Option<String>,
    /// Optional session creation source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SessionSource>,
}

/// Response for session creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    /// Session ID
    pub id: String,
    /// Session name
    pub name: String,
    /// Shell type
    #[serde(rename = "shellType")]
    pub shell_type: ShellType,
    /// Current working directory
    pub cwd: String,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Session status
    pub status: String,
    /// Terminal dimensions
    pub cols: u16,
    pub rows: u16,
    /// Session creation source
    pub source: SessionSource,
}

impl From<TerminalSession> for SessionResponse {
    fn from(session: TerminalSession) -> Self {
        Self {
            id: session.id,
            name: session.name,
            shell_type: session.shell_type,
            cwd: session.cwd,
            pid: session.pid,
            status: format!("{:?}", session.status),
            cols: session.cols,
            rows: session.rows,
            source: session.source,
        }
    }
}

/// Request to write data to a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Data to write
    pub data: String,
}

/// Request to resize terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// New column count
    pub cols: u16,
    /// New row count
    pub rows: u16,
}

/// Request to close a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Whether to force immediate close
    pub immediate: Option<bool>,
}

/// Request to send a signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Signal name (e.g., "SIGINT")
    pub signal: String,
}

/// Request to acknowledge data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Number of characters acknowledged
    #[serde(rename = "charCount")]
    pub char_count: usize,
}

/// Request to get session output history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHistoryRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Response for session history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHistoryResponse {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Output history data
    pub data: String,
    /// Current history size in bytes
    #[serde(rename = "historySize")]
    pub history_size: usize,
    /// Terminal column count when history was recorded (PTY current size)
    pub cols: u16,
    /// Terminal row count when history was recorded (PTY current size)
    pub rows: u16,
}

/// Shell information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    /// Shell type
    #[serde(rename = "shellType")]
    pub shell_type: ShellType,
    /// Display name
    pub name: String,
    /// Path to executable
    pub path: String,
    /// Shell version (if detected)
    pub version: Option<String>,
    /// Whether the shell is available
    pub available: bool,
}

/// Request to execute a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Command to execute
    pub command: String,
    /// Timeout in milliseconds (default: 30000)
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    /// Whether to prevent the command from being added to shell history
    #[serde(rename = "preventHistory")]
    pub prevent_history: Option<bool>,
}

/// Response for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandResponse {
    /// The command that was executed
    pub command: String,
    /// Unique command ID
    #[serde(rename = "commandId")]
    pub command_id: String,
    /// Command output
    pub output: String,
    /// Exit code (if available)
    #[serde(rename = "exitCode")]
    pub exit_code: Option<i32>,
    /// Why command execution stopped.
    #[serde(rename = "completionReason")]
    pub completion_reason: CommandCompletionReason,
}

impl From<CommandExecuteResult> for ExecuteCommandResponse {
    fn from(result: CommandExecuteResult) -> Self {
        Self {
            command: result.command,
            command_id: result.command_id,
            output: result.output,
            exit_code: result.exit_code,
            completion_reason: result.completion_reason,
        }
    }
}

/// Request to send a command without waiting for completion
///
/// Unlike ExecuteCommandRequest, this does NOT require shell integration
/// and does NOT wait for command completion or capture output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendCommandRequest {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Command to send
    pub command: String,
}

// ============================================================================
// WebSocket Envelope Types
// ============================================================================

/// WebSocket message from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WsRequest {
    /// Create session
    CreateSession(CreateSessionRequest),
    /// Write to session
    Write(WriteRequest),
    /// Resize session
    Resize(ResizeRequest),
    /// Send signal
    Signal(SignalRequest),
    /// Close session
    CloseSession(CloseSessionRequest),
    /// Acknowledge data
    Acknowledge(AcknowledgeRequest),
    /// List sessions
    ListSessions,
    /// Get available shells
    GetShells,
}

/// WebSocket message from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsResponse {
    /// Success response
    Success {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    /// Error response
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    /// Terminal event
    Event(TerminalEvent),
}

impl WsResponse {
    /// Create a success response
    pub fn success<T: Serialize>(data: T) -> Self {
        WsResponse::Success {
            data: Some(serde_json::to_value(data).unwrap_or(serde_json::Value::Null)),
        }
    }

    /// Create an empty success response
    pub fn ok() -> Self {
        WsResponse::Success { data: None }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        WsResponse::Error {
            message: message.into(),
            code: None,
        }
    }

    /// Create an error response with code
    pub fn error_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        WsResponse::Error {
            message: message.into(),
            code: Some(code.into()),
        }
    }
}
