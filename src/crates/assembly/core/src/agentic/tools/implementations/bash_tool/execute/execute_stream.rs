use super::execute_loop::deliver_background_bash_result;
use super::execute_signal::deliver_background_stream_end_error;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::NortHingError;
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;
use terminal_core::{CommandCompletionReason, CommandStreamEvent};
use tool_runtime::shell::{
    format_background_command_delivery_text, format_background_command_display_text,
    format_background_command_error_display_text, format_background_command_error_text,
    BackgroundCommandDeliveryTextRequest, BackgroundCommandErrorTextRequest, BackgroundCommandStatusFacts,
};
use tracing::error;

/// Messages sent from the background stream consumer to the I/O writer task.
enum BackgroundStreamCommand {
    Output(String),
    CompletionOutput(String),
}

/// Process one `CommandStreamEvent` for foreground execution.
///
/// Updates `accumulated_output`, `final_exit_code`, `was_interrupted`, `timed_out`,
/// `completion_reason_label`, and `final_shell_state` in place.
///
/// Returns `true` if the event is terminal (Completed or Error) and the loop should break.
pub(crate) fn process_stream_event(
    event: CommandStreamEvent,
    tool_use_id: &str,
    tool_name: &str,
    accumulated_output: &mut String,
    final_exit_code: &mut Option<i32>,
    was_interrupted: &mut bool,
    timed_out: &mut bool,
    completion_reason_label: &mut String,
    final_shell_state: &mut Option<String>,
) -> bool {
    match event {
        CommandStreamEvent::Started { command_id } => {
            tracing::debug!("Bash command started execution, command_id: {}", command_id);
        }
        CommandStreamEvent::Output { data } => {
            accumulated_output.push_str(&data);

            let progress_event = crate::infrastructure::events::event_system::BackendEvent::ToolExecutionProgress(
                crate::util::types::event::ToolExecutionProgressInfo {
                    tool_use_id: tool_use_id.to_string(),
                    tool_name: tool_name.to_string(),
                    progress_message: data,
                    percentage: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            );

            let event_system_clone = crate::infrastructure::events::event_system::global_event_system().clone();
            tokio::spawn(async move {
                let _ = event_system_clone.emit(progress_event).await;
            });
        }
        CommandStreamEvent::Completed {
            exit_code,
            total_output,
            completion_reason,
            shell_state,
        } => {
            tracing::debug!(
                "Bash command completed, exit_code: {:?}, tool_id: {}",
                exit_code,
                tool_use_id
            );
            *final_exit_code = exit_code.or(*final_exit_code);
            *timed_out = completion_reason == CommandCompletionReason::TimedOut;
            *completion_reason_label = format!("{:?}", completion_reason);

            if !*timed_out && matches!(exit_code, Some(130) | Some(-1073741510)) {
                *was_interrupted = true;
            }

            if !total_output.is_empty() {
                *accumulated_output = total_output;
            }

            if shell_state.is_some() {
                *final_shell_state = shell_state;
            }
            return true;
        }
        CommandStreamEvent::Error { message } => {
            error!("Bash command execution error: {}, tool_id: {}", message, tool_use_id);
            // Caller should propagate the error via Result before calling this fn.
        }
    }
    false
}

/// Background stream consumer: reads from the terminal stream and writes to a file.
///
/// Owns the file handle for the duration of the background task so the caller does
/// not need to keep the writer alive across an `await` boundary inside a `tokio::spawn`.
pub(crate) async fn run_background_stream_task(
    terminal_session_id: String,
    command: String,
    working_directory: String,
    output_file_path: String,
    tool_use_id: String,
    parent_session_id: String,
    parent_agent_type: String,
    parent_workspace_path: Option<String>,
) {
    let output_file = match tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_file_path)
        .await
    {
        Ok(f) => f,
        Err(error) => {
            error!(
                "Failed to open background output file: session_id={}, path={}, error={}",
                terminal_session_id, output_file_path, error
            );
            return;
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BackgroundStreamCommand>(16);
    let output_file_reference = super::execute_loop::background_output_file_reference_from_path(&output_file_path);

    let terminal_session_id_for_writer = terminal_session_id.clone();

    // Writer task: owns the file handle, processes writes from the channel.
    let writer_task = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let mut writer = tokio::io::BufWriter::new(output_file);
        let mut persist_error: Option<String> = None;

        while let Some(cmd) = rx.recv().await {
            if persist_error.is_some() {
                continue;
            }
            match cmd {
                BackgroundStreamCommand::Output(data) => {
                    if let Err(error) = writer.write_all(data.as_bytes()).await {
                        persist_error = Some(error.to_string());
                        error!(
                            "Failed to write background Bash output: session_id={}, error={}",
                            terminal_session_id_for_writer, error
                        );
                    } else if let Err(error) = writer.flush().await {
                        persist_error = Some(error.to_string());
                        error!(
                            "Failed to flush background Bash output: session_id={}, error={}",
                            terminal_session_id_for_writer, error
                        );
                    }
                }
                BackgroundStreamCommand::CompletionOutput(data) => {
                    if persist_error.is_none() {
                        if let Err(error) = writer.write_all(data.as_bytes()).await {
                            persist_error = Some(error.to_string());
                            error!(
                                "Failed to persist background Bash completion output: session_id={}, error={}",
                                terminal_session_id_for_writer, error
                            );
                        } else if let Err(error) = writer.flush().await {
                            persist_error = Some(error.to_string());
                            error!(
                                "Failed to flush background Bash completion output: session_id={}, error={}",
                                terminal_session_id_for_writer, error
                            );
                        }
                    }
                }
            }
        }
        persist_error
    });

    // Stream consumer: reads terminal events and sends writes to the writer task.
    let terminal_api = match terminal_core::TerminalApi::from_singleton() {
        Ok(api) => api,
        Err(error) => {
            error!(
                "Background Bash command could not access terminal singleton: session_id={}, error={}",
                terminal_session_id, error
            );
            return;
        }
    };

    let mut stream = terminal_api.execute_command_stream(terminal_core::ExecuteCommandRequest {
        session_id: terminal_session_id.clone(),
        command: command.clone(),
        timeout_ms: None,
        prevent_history: Some(true),
    });

    let mut saw_completion = false;
    let mut delivery_sent = false;

    while let Some(event) = stream.next().await {
        match event {
            terminal_core::CommandStreamEvent::Started { command_id } => {
                tracing::debug!(
                    "Background Bash command started execution, session_id={}, command_id={}",
                    terminal_session_id,
                    command_id
                );
            }
            terminal_core::CommandStreamEvent::Output { data } => {
                let _ = tx.send(BackgroundStreamCommand::Output(data)).await;
            }
            terminal_core::CommandStreamEvent::Completed {
                exit_code,
                total_output,
                completion_reason,
                shell_state: _,
            } => {
                saw_completion = true;

                if !total_output.is_empty() {
                    let _ = tx.send(BackgroundStreamCommand::CompletionOutput(total_output)).await;
                }

                let timed_out = completion_reason == terminal_core::CommandCompletionReason::TimedOut;
                let interrupted = !timed_out && matches!(exit_code, Some(130) | Some(-1073741510));
                let status = BackgroundCommandStatusFacts {
                    exit_code,
                    timed_out,
                    interrupted,
                };
                let delivery_text = format_background_command_delivery_text(BackgroundCommandDeliveryTextRequest {
                    command: &command,
                    terminal_session_id: &terminal_session_id,
                    working_directory: &working_directory,
                    status,
                    output_file_reference: &output_file_reference,
                    output_persist_error: None,
                });
                let display_text = format_background_command_display_text(BackgroundCommandStatusFacts {
                    exit_code,
                    timed_out,
                    interrupted,
                });
                let metadata = json!({
                    "kind": "background_result",
                    "sourceKind": "bash_command",
                    "toolName": "Bash",
                    "toolCallId": tool_use_id.clone(),
                    "terminalSessionId": terminal_session_id.clone(),
                    "command": command.clone(),
                    "workingDirectory": working_directory.clone(),
                    "outputFile": output_file_reference.clone(),
                });

                deliver_background_bash_result(
                    parent_session_id.clone(),
                    parent_agent_type.clone(),
                    parent_workspace_path.clone(),
                    delivery_text,
                    display_text,
                    metadata,
                    terminal_session_id.clone(),
                    "result",
                )
                .await;
                delivery_sent = true;
                break;
            }
            terminal_core::CommandStreamEvent::Error { message } => {
                let delivery_text = format_background_command_error_text(BackgroundCommandErrorTextRequest {
                    command: &command,
                    terminal_session_id: &terminal_session_id,
                    working_directory: &working_directory,
                    output_file_reference: &output_file_reference,
                    error: &message,
                    output_persist_error: None,
                });
                let display_text = format_background_command_error_display_text();
                let metadata = json!({
                    "kind": "background_result",
                    "sourceKind": "bash_command",
                    "toolName": "Bash",
                    "toolCallId": tool_use_id.clone(),
                    "terminalSessionId": terminal_session_id.clone(),
                    "command": command.clone(),
                    "workingDirectory": working_directory.clone(),
                    "outputFile": output_file_reference.clone(),
                    "error": message.clone(),
                });

                deliver_background_bash_result(
                    parent_session_id.clone(),
                    parent_agent_type.clone(),
                    parent_workspace_path.clone(),
                    delivery_text,
                    display_text,
                    metadata,
                    terminal_session_id.clone(),
                    "error result",
                )
                .await;
                delivery_sent = true;
                break;
            }
        }
    }

    drop(tx);
    let _persist_error = writer_task.await;

    if !saw_completion && !delivery_sent {
        deliver_background_stream_end_error(
            &terminal_session_id,
            &command,
            &working_directory,
            &output_file_reference,
            None,
            parent_session_id,
            parent_agent_type,
            parent_workspace_path,
            tool_use_id,
        )
        .await;
    }
}
