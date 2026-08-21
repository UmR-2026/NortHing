use serde_json::json;

use super::execute_loop::{background_output_file_path, deliver_background_bash_result};
use crate::agentic::tools::framework::ToolUseContext;
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use std::time::Instant;
use terminal_core::{SignalRequest, TerminalApi};
use tool_runtime::shell::{
    format_background_command_delivery_text, format_background_command_display_text,
    format_background_command_error_display_text, format_background_command_error_text,
    BackgroundCommandDeliveryTextRequest, BackgroundCommandErrorTextRequest, BackgroundCommandStatusFacts,
    BASH_INTERRUPT_OUTPUT_DRAIN_MS,
};
use tracing::error;

/// Check cancellation before creating a background session or sending a background command.
pub(crate) fn cancellation_requested(context: &ToolUseContext) -> bool {
    context.cancellation_token().is_some_and(|token| token.is_cancelled())
}

/// Build a cancellation error for use before creating a session.
pub(crate) fn cancellation_error(context: &'static str) -> NortHingError {
    NortHingError::tool(format!("Bash {} cancelled by user", context))
}

/// Send SIGINT to a terminal session for interactive cancellation.
pub(crate) async fn send_interrupt_signal(terminal_api: &TerminalApi, session_id: &str) -> NortHingResult<()> {
    terminal_api
        .signal(SignalRequest {
            session_id: session_id.to_string(),
            signal: "SIGINT".to_string(),
        })
        .await
        .map_err(|e| NortHingError::tool(format!("Failed to send interrupt signal: {}", e)))?;
    Ok(())
}

/// Close a background terminal session when cancellation is detected before the command starts.
pub(crate) async fn close_background_session(session_id: &str) -> NortHingResult<()> {
    let terminal_api =
        TerminalApi::from_singleton().map_err(|e| NortHingError::tool(format!("Terminal not initialized: {}", e)))?;
    terminal_api
        .close_session(terminal_core::CloseSessionRequest {
            session_id: session_id.to_string(),
            immediate: Some(true),
        })
        .await
        .map_err(|e| NortHingError::tool(format!("Failed to close background session: {}", e)))?;
    Ok(())
}

/// Deliver a "stream ended without completion" error for a background command.
pub(crate) async fn deliver_background_stream_end_error(
    terminal_session_id: &str,
    command: &str,
    working_directory: &str,
    output_file_reference: &str,
    output_persist_error: Option<&str>,
    parent_session_id: String,
    parent_agent_type: String,
    parent_workspace_path: Option<String>,
    tool_use_id: String,
) {
    let delivery_text = format_background_command_error_text(BackgroundCommandErrorTextRequest {
        command,
        terminal_session_id,
        working_directory,
        output_file_reference,
        error: "Background Bash command stream ended without a completion event.",
        output_persist_error,
    });
    let display_text = format_background_command_error_display_text();
    let metadata = json!({
        "kind": "background_result",
        "sourceKind": "bash_command",
        "toolName": "Bash",
        "toolCallId": tool_use_id,
        "terminalSessionId": terminal_session_id,
        "command": command,
        "workingDirectory": working_directory,
        "outputFile": output_file_reference,
        "error": "stream_ended_without_completion",
    });

    deliver_background_bash_result(
        parent_session_id,
        parent_agent_type,
        parent_workspace_path,
        delivery_text,
        display_text,
        metadata,
        terminal_session_id.to_string(),
        "stream-end result",
    )
    .await;
}
