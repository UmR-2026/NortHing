use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::agentic::workspace::WorkspaceCommandOptions;
use crate::infrastructure::events::event_system::global_event_system;
use crate::infrastructure::events::event_system::BackendEvent::ToolExecutionProgress;
use crate::service_agent_runtime::CoreServiceAgentRuntime;
use crate::util::elapsed_ms_u64;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::event::ToolExecutionProgressInfo;
use futures::StreamExt;
use northhing_runtime_ports::AgentBackgroundResultRequest;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use terminal_core::shell::ShellType;
use terminal_core::{
    CommandCompletionReason, CommandStreamEvent, ExecuteCommandRequest, SignalRequest, TerminalApi,
    TerminalBindingOptions, TerminalSessionBinding,
};
use tool_runtime::shell::{
    command_for_working_directory, format_background_command_delivery_text, format_background_command_display_text,
    format_background_command_error_display_text, format_background_command_error_text, render_local_shell_result,
    render_remote_shell_result, BackgroundCommandDeliveryTextRequest, BackgroundCommandErrorTextRequest,
    BackgroundCommandStatusFacts, LocalShellResultRenderRequest, RemoteShellResultRenderRequest,
    BASH_INTERRUPT_OUTPUT_DRAIN_MS, BASH_RESULT_MAX_OUTPUT_LENGTH,
};
use tracing::{debug, error, info};

use super::execute_format::{format_local_result, format_remote_result};
use super::execute_signal::{close_background_session, emit_terminal_ready_event, send_interrupt_signal};
use super::execute_stream::{process_stream_event, run_background_stream_task};
use crate::agentic::tools::implementations::bash_tool::bash_helpers::background_output_file_reference;
use crate::agentic::tools::implementations::bash_tool::bash_sandbox::{
    cancellation_error, cancellation_requested, command_needs_light_checkpoint,
};
use crate::agentic::tools::implementations::bash_tool::BashTool;

/// Deliver a background Bash result to the agent runtime.
pub(crate) async fn deliver_background_bash_result(
    parent_session_id: String,
    parent_agent_type: String,
    parent_workspace_path: Option<String>,
    delivery_text: String,
    display_text: String,
    metadata: serde_json::Value,
    terminal_session_id: String,
    failure_context: &'static str,
) {
    let metadata_map = metadata.as_object().cloned().unwrap_or_default();

    let runtime = match CoreServiceAgentRuntime::global_agent_runtime_with_lifecycle_delivery() {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(
                "Agent runtime lifecycle delivery is not available; background Bash {} dropped: session_id={}, terminal_session_id={}, error={}",
                failure_context, parent_session_id, terminal_session_id, error
            );
            return;
        }
    };

    if let Err(error) = runtime
        .deliver_background_result(AgentBackgroundResultRequest {
            session_id: parent_session_id.clone(),
            agent_type: parent_agent_type,
            workspace_path: parent_workspace_path,
            content: delivery_text,
            display_content: Some(display_text),
            metadata: metadata_map,
        })
        .await
    {
        error!(
            "Failed to deliver background Bash {}: session_id={}, terminal_session_id={}, error={}",
            failure_context,
            parent_session_id,
            terminal_session_id,
            CoreServiceAgentRuntime::runtime_error_message(error)
        );
    }
}

/// Compute the output file path for a background Bash tool result.
pub(crate) fn background_output_file_path(
    context: &ToolUseContext,
    chat_session_id: &str,
    tool_use_id: &str,
) -> Option<PathBuf> {
    context
        .current_workspace_session_tool_result_path(chat_session_id, &format!("{}.txt", tool_use_id))
        .ok()
}

/// Compute a human-readable reference for an output file given its path.
pub(crate) fn background_output_file_reference_from_path(output_file_path: &str) -> String {
    format!("/runtime/sessions/{}", output_file_path)
}

/// Main entry point for synchronous Bash tool execution.
///
/// Handles three paths:
/// 1. Remote workspace execution (via SSH).
/// 2. Background command (spawns a long-running terminal session).
/// 3. Foreground terminal execution (stream events, accumulate output, return result).
pub(crate) async fn execute_call(input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
    let start_time = Instant::now();

    let command_str = input
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NortHingError::tool("command is required".to_string()))?;
    let requested_working_directory = BashTool::resolve_working_directory(input, context)?;

    if command_needs_light_checkpoint(command_str) {
        context.record_light_checkpoint("Bash", command_str, Vec::new()).await;
    }

    // --- Remote workspace path ---
    if context.is_remote() {
        let Some(ws_shell) = context.ws_shell() else {
            return Err(NortHingError::tool(
                "Remote workspace shell is unavailable; refusing to run Bash locally for a remote session.".to_string(),
            ));
        };

        info!("Executing command on remote workspace via SSH: {}", command_str);
        let remote_command = command_for_working_directory(command_str, requested_working_directory.as_deref());

        let timeout_ms = input.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(120_000);

        let exec_result = ws_shell
            .exec_with_options(
                &remote_command,
                WorkspaceCommandOptions {
                    timeout_ms: Some(timeout_ms),
                    cancellation_token: context.cancellation_token().cloned(),
                },
            )
            .await
            .map_err(|e| NortHingError::tool(format!("Remote command execution failed: {}", e)))?;

        let output = exec_result.combined_output();
        let execution_time_ms = elapsed_ms_u64(start_time);
        let working_directory = context
            .workspace_root()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let working_directory = requested_working_directory.unwrap_or(working_directory);

        let result_for_assistant = format_remote_result(
            &working_directory,
            &exec_result.stdout,
            &exec_result.stderr,
            exec_result.interrupted,
            exec_result.timed_out,
            exec_result.exit_code,
        )?;

        let result = ToolResult::Result {
            data: json!({
                "success": exec_result.exit_code == 0,
                "command": command_str,
                "stdout": exec_result.stdout,
                "stderr": exec_result.stderr,
                "output": output,
                "exit_code": exec_result.exit_code,
                "interrupted": exec_result.interrupted,
                "timed_out": exec_result.timed_out,
                "working_directory": working_directory,
                "execution_time_ms": execution_time_ms,
                "duration_ms": execution_time_ms,
                "is_remote": true
            }),
            result_for_assistant: Some(result_for_assistant),
            image_attachments: None,
        };
        return Ok(vec![result]);
    }

    // --- Background vs foreground local execution ---
    let run_in_background = input
        .get("run_in_background")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let chat_session_id = context
        .session_id
        .as_ref()
        .ok_or_else(|| NortHingError::tool("session_id is required for Bash tool".to_string()))?;

    let tool_use_id = context
        .tool_call_id
        .clone()
        .unwrap_or_else(|| format!("bash_{}", uuid::Uuid::new_v4()));

    let terminal_api =
        TerminalApi::from_singleton().map_err(|e| NortHingError::tool(format!("Terminal not initialized: {}", e)))?;

    let shell_type = crate::agentic::tools::implementations::bash_tool::bash_sandbox::resolve_shell()
        .await
        .shell_type;

    let binding = terminal_api.session_manager().binding();
    let workspace_path = context
        .workspace_root()
        .ok_or_else(|| NortHingError::tool("workspace_path is required for Bash tool".to_string()))?
        .to_string_lossy()
        .to_string();

    if run_in_background {
        if cancellation_requested(context) {
            return Err(cancellation_error("before creating background session"));
        }

        let initial_cwd = if let Some(requested_dir) = requested_working_directory.as_ref() {
            requested_dir.clone()
        } else if let Some(existing_id) = binding.get(chat_session_id) {
            terminal_api
                .get_session(&existing_id)
                .await
                .map(|s| s.cwd)
                .unwrap_or_else(|_| workspace_path.clone())
        } else {
            workspace_path.clone()
        };

        return call_background(
            command_str,
            chat_session_id,
            &initial_cwd,
            context,
            shell_type,
            &binding,
            start_time,
        )
        .await;
    }

    // --- Foreground terminal execution ---
    let terminal_ready_started_at = Instant::now();
    let primary_session_id = binding
        .get_or_create(
            chat_session_id,
            TerminalBindingOptions {
                working_directory: Some(workspace_path.clone()),
                session_id: Some(chat_session_id.to_string()),
                session_name: Some(format!("Chat-{}", &chat_session_id[..8.min(chat_session_id.len())])),
                shell_type: shell_type.clone(),
                env: Some(BashTool::noninteractive_env()),
                source: Some(terminal_core::session::SessionSource::Agent),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| NortHingError::tool(format!("Failed to create Terminal session: {}", e)))?;
    let terminal_ready_ms = elapsed_ms_u64(terminal_ready_started_at);

    emit_terminal_ready_event(&tool_use_id, &primary_session_id);

    let primary_cwd = terminal_api
        .get_session(&primary_session_id)
        .await
        .map(|s| s.cwd)
        .unwrap_or_else(|_| workspace_path.clone());
    let execution_working_directory = requested_working_directory
        .as_ref()
        .cloned()
        .unwrap_or_else(|| primary_cwd.clone());
    let command_to_execute = command_for_working_directory(command_str, requested_working_directory.as_deref());

    let tool_name = "Bash".to_string();

    const DEFAULT_TIMEOUT_MS: u64 = 120_000;
    const MAX_TIMEOUT_MS: u64 = 600_000;
    let timeout_ms = Some(
        input
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .min(MAX_TIMEOUT_MS),
    );

    debug!(
        "Bash tool executing command: {}, session_id: {}, tool_id: {}",
        command_to_execute, chat_session_id, tool_use_id
    );

    let request = ExecuteCommandRequest {
        session_id: primary_session_id.clone(),
        command: command_to_execute,
        timeout_ms,
        prevent_history: Some(true),
    };

    let mut stream = terminal_api.execute_command_stream(request);
    let mut accumulated_output = String::new();
    let mut final_exit_code: Option<i32> = None;
    let mut was_interrupted = false;
    let mut timed_out = false;
    let mut final_shell_state: Option<String> = None;
    let mut command_started_after_ms: Option<u64> = None;
    let mut completion_reason_label = "stream_end".to_string();
    let mut interrupt_drain_deadline: Option<tokio::time::Instant> = None;
    let command_stream_started_at = Instant::now();

    let event_system = global_event_system();

    loop {
        let next_event = if let Some(deadline) = interrupt_drain_deadline {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }

            match tokio::time::timeout_at(deadline, stream.next()).await {
                Ok(event) => event,
                Err(_) => break,
            }
        } else {
            stream.next().await
        };

        let Some(event) = next_event else {
            break;
        };

        if let Some(token) = context.cancellation_token() {
            if token.is_cancelled() && !was_interrupted {
                debug!(
                    "Bash tool received cancellation request, sending interrupt signal, tool_id: {}",
                    tool_use_id
                );
                was_interrupted = true;
                interrupt_drain_deadline =
                    Some(tokio::time::Instant::now() + Duration::from_millis(BASH_INTERRUPT_OUTPUT_DRAIN_MS));

                if let Err(e) = send_interrupt_signal(&terminal_api, &primary_session_id).await {
                    error!("Failed to send interrupt signal: {}", e);
                }

                #[cfg(windows)]
                {
                    final_exit_code = Some(-1073741510);
                }
                #[cfg(not(windows))]
                {
                    final_exit_code = Some(130);
                }
            }
        }

        // Check for error events before passing to the processor so the caller can
        // propagate them via Result rather than silently continuing the loop.
        if let CommandStreamEvent::Error { ref message } = event {
            return Err(NortHingError::tool(format!("Command execution error: {}", message)));
        }

        let should_break = process_stream_event(
            event,
            &tool_use_id,
            &tool_name,
            &mut accumulated_output,
            &mut final_exit_code,
            &mut was_interrupted,
            &mut timed_out,
            &mut completion_reason_label,
            &mut final_shell_state,
        );
        if should_break {
            break;
        }
    }

    let execution_time_ms = elapsed_ms_u64(start_time);
    let command_stream_ms = elapsed_ms_u64(command_stream_started_at);
    info!(
        "Bash command completed: tool_id={}, terminal_session_id={}, duration_ms={}, terminal_ready_ms={}, command_started_after_ms={:?}, command_stream_ms={}, output_bytes={}, exit_code={:?}, interrupted={}, timed_out={}, completion_reason={}",
        tool_use_id,
        primary_session_id,
        execution_time_ms,
        terminal_ready_ms,
        command_started_after_ms,
        command_stream_ms,
        accumulated_output.len(),
        final_exit_code,
        was_interrupted,
        timed_out,
        completion_reason_label
    );

    let result_for_assistant = format_local_result(
        &primary_session_id,
        &execution_working_directory,
        &accumulated_output,
        was_interrupted,
        timed_out,
        final_exit_code.unwrap_or(-1),
        final_shell_state.as_deref(),
    )?;

    Ok(vec![ToolResult::Result {
        data: json!({
            "success": final_exit_code.unwrap_or(-1) == 0,
            "command": command_str,
            "output": accumulated_output,
            "exit_code": final_exit_code,
            "interrupted": was_interrupted,
            "timed_out": timed_out,
            "working_directory": execution_working_directory,
            "execution_time_ms": execution_time_ms,
            "terminal_session_id": primary_session_id,
        }),
        result_for_assistant: Some(result_for_assistant),
        image_attachments: None,
    }])
}

/// Entry point for background (long-running) Bash tool execution.
///
/// Creates a background terminal session, opens an output file, and spawns the
/// stream consumer task. Returns immediately with a "started" result; the final
/// result is delivered asynchronously via `deliver_background_bash_result`.
pub(crate) async fn call_background(
    command_str: &str,
    chat_session_id: &str,
    initial_cwd: &str,
    context: &ToolUseContext,
    shell_type: Option<ShellType>,
    binding: &TerminalSessionBinding,
    start_time: Instant,
) -> NortHingResult<Vec<ToolResult>> {
    debug!(
        "Bash tool starting background command: {}, owner: {}",
        command_str, chat_session_id
    );

    if cancellation_requested(context) {
        return Err(cancellation_error("before creating background terminal"));
    }

    let bg_session_id = binding
        .create_background_session(
            chat_session_id,
            TerminalBindingOptions {
                working_directory: Some(initial_cwd.to_string()),
                session_id: None,
                session_name: None,
                shell_type,
                env: Some(BashTool::noninteractive_env()),
                source: Some(terminal_core::session::SessionSource::Agent),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| NortHingError::tool(format!("Failed to create background terminal session: {}", e)))?;

    let tool_use_id = context
        .tool_call_id
        .clone()
        .unwrap_or_else(|| format!("bash_{}", uuid::Uuid::new_v4()));
    emit_terminal_ready_event(&tool_use_id, &bg_session_id);

    if cancellation_requested(context) {
        if let Err(e) = close_background_session(&bg_session_id).await {
            error!("Failed to close cancelled background session: {}", e);
        }
        return Err(cancellation_error("before sending background command"));
    }

    let output_file_path = background_output_file_path(context, chat_session_id, &tool_use_id)
        .ok_or_else(|| NortHingError::tool("Failed to prepare a background output file for Bash tool".to_string()))?;
    if let Some(parent) = output_file_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| NortHingError::tool(format!("Failed to create background output directory: {}", e)))?;
    }

    let parent_session_id = chat_session_id.to_string();
    let parent_agent_type = context.agent_type.clone().unwrap_or_else(|| "Agentic".to_string());
    let parent_workspace_path = context.workspace_root().map(|path| path.to_string_lossy().to_string());
    let command = command_str.to_string();
    let working_directory = initial_cwd.to_string();
    let terminal_session_id = bg_session_id.clone();
    let tool_use_id_for_task = tool_use_id.clone();

    tokio::spawn(run_background_stream_task(
        terminal_session_id,
        command,
        working_directory,
        output_file_path.to_string_lossy().to_string(),
        tool_use_id_for_task,
        parent_session_id,
        parent_agent_type,
        parent_workspace_path,
    ));

    let execution_time_ms = elapsed_ms_u64(start_time);
    let output_file_note = format!(
        "\nFull output will be saved to: {}",
        background_output_file_reference(context, chat_session_id, &tool_use_id, &output_file_path)
    );

    let result_data = json!({
        "success": true,
        "command": command_str,
        "output": format!("Command started in background terminal session.{}", output_file_note),
        "exit_code": null,
        "interrupted": false,
        "working_directory": initial_cwd,
        "execution_time_ms": execution_time_ms,
        "terminal_session_id": bg_session_id,
        "output_file": background_output_file_reference(context, chat_session_id, &tool_use_id, &output_file_path),
        "run_in_background": true,
    });

    let result_for_assistant = format!(
        "Command started in background terminal session (id: {}). Working directory: {}.{} Its final result will be delivered back automatically when it finishes. Do not poll for status updates. If your current path is blocked on this result and there is no other useful local work to do, it is fine to end the current turn.",
        bg_session_id, initial_cwd, output_file_note
    );

    Ok(vec![ToolResult::Result {
        data: result_data,
        result_for_assistant: Some(result_for_assistant),
        image_attachments: None,
    }])
}
