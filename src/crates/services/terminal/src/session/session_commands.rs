//! Session command execution + streaming.
//!
//! Standalone sibling — `impl SessionManager` for:
//! - `execute_command` / `execute_command_with_options` (one-shot wait)
//! - `execute_command_stream` / `execute_command_stream_with_options` (event stream)
//! - `send_command` (fire-and-forget, no wait)
//! - `wait_for_session_active` (simpler readiness check)
//!
//! Uses free helpers from `super::session_manager`:
//! - `compute_stream_output_delta`
//! - `get_integration_output_snapshot`
//! - `get_post_command_terminal_state`
//!
//! Together with `session_lifecycle`, `session_events`,
//! and `session_shell_integration` siblings, this replaces the previous
//! monolithic 1391-line `impl SessionManager` block.

use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::shell::CommandState;
use crate::{TerminalError, TerminalResult};

use super::super::SessionStatus;
use super::session_manager::{
    compute_stream_output_delta, get_integration_output_snapshot, get_post_command_terminal_state, SessionManager,
    COMMAND_TIMEOUT_INTERRUPT_GRACE_MS,
};
use super::types::{CommandCompletionReason, CommandExecuteResult, CommandStream, CommandStreamEvent, ExecuteOptions};

impl SessionManager {
    /// Execute a command in a session and wait for completion
    ///
    /// This function sends a command to the terminal, waits for it to complete
    /// using shell integration, and returns the output and exit code.
    pub async fn execute_command(&self, session_id: &str, command: &str) -> TerminalResult<CommandExecuteResult> {
        self.execute_command_with_options(session_id, command, ExecuteOptions::default())
            .await
    }

    /// Execute a command with custom options
    pub async fn execute_command_with_options(
        &self,
        session_id: &str,
        command: &str,
        options: ExecuteOptions,
    ) -> TerminalResult<CommandExecuteResult> {
        let mut stream = self.execute_command_stream_with_options(session_id.to_string(), command.to_string(), options);
        let mut command_id = uuid::Uuid::new_v4().to_string();
        let mut output = String::new();

        while let Some(event) = stream.next().await {
            match event {
                CommandStreamEvent::Started {
                    command_id: started_command_id,
                } => {
                    command_id = started_command_id;
                }
                CommandStreamEvent::Output { data } => {
                    output.push_str(&data);
                }
                CommandStreamEvent::Completed {
                    exit_code,
                    total_output,
                    completion_reason,
                    shell_state: _,
                } => {
                    if !total_output.is_empty() {
                        output = total_output;
                    }

                    return Ok(CommandExecuteResult {
                        command: command.to_string(),
                        command_id,
                        output,
                        exit_code,
                        completion_reason,
                    });
                }
                CommandStreamEvent::Error { message } => {
                    return Err(TerminalError::Session(message));
                }
            }
        }

        Err(TerminalError::Session(format!(
            "Command stream ended unexpectedly for session {}",
            session_id
        )))
    }

    /// Execute a command and return a stream of events
    ///
    /// This function provides real-time streaming of command output,
    /// allowing callers to process output as it arrives.
    pub fn execute_command_stream(&self, session_id: String, command: String) -> CommandStream {
        self.execute_command_stream_with_options(session_id, command, ExecuteOptions::default())
    }

    /// Execute a command with options and return a stream of events
    pub fn execute_command_stream_with_options(
        &self,
        session_id: String,
        command: String,
        options: ExecuteOptions,
    ) -> CommandStream {
        let sessions = self.sessions.clone();
        let session_integrations = self.session_integrations.clone();
        let pty_service = self.pty_service.clone();
        let timeout_duration = options.timeout; // None means no timeout
        let prevent_history = options.prevent_history;

        let (tx, rx) = mpsc::channel::<CommandStreamEvent>(256);

        // Spawn the execution task
        tokio::spawn(async move {
            // Helper to send events
            let send = |event: CommandStreamEvent| {
                let tx = tx.clone();
                async move {
                    let _ = tx.send(event).await;
                }
            };

            // Wait for session to be ready before executing command
            // (static method on sibling impl SessionManager in session_shell_integration)
            if let Err(e) = Self::wait_for_session_ready_static(&sessions, &session_integrations, &session_id).await {
                send(CommandStreamEvent::Error {
                    message: format!("Session not ready: {}", e),
                })
                .await;
                return;
            }

            // Check if session exists
            let pty_id = {
                let sessions_guard = sessions.read().await;
                match sessions_guard.get(&session_id) {
                    Some(session) => session.pty_id,
                    None => {
                        send(CommandStreamEvent::Error {
                            message: format!("Session not found: {}", session_id),
                        })
                        .await;
                        return;
                    }
                }
            };

            let pty_id = match pty_id {
                Some(id) => id,
                None => {
                    send(CommandStreamEvent::Error {
                        message: "Session has no PTY".to_string(),
                    })
                    .await;
                    return;
                }
            };

            // Generate command ID
            let command_id = uuid::Uuid::new_v4().to_string();

            // Clear any previous output
            {
                let mut integrations = session_integrations.write().await;
                if let Some(integration) = integrations.get_mut(&session_id) {
                    integration.clear_output();
                }
            }

            // Send started event
            send(CommandStreamEvent::Started {
                command_id: command_id.clone(),
            })
            .await;

            // Prepare the command
            let cmd_to_send = if prevent_history {
                format!(" {}\r", command)
            } else {
                format!("{}\r", command)
            };

            // Send the command
            if let Err(e) = pty_service.write(pty_id, cmd_to_send.as_bytes()).await {
                send(CommandStreamEvent::Error {
                    message: format!("Failed to send command: {}", e),
                })
                .await;
                return;
            }

            // Poll for output and completion
            let poll_interval = Duration::from_millis(50);
            let max_idle_checks = 20;
            let mut idle_count = 0;
            let mut last_output_len = 0;
            let mut last_sent_output = String::new();
            let start_time = std::time::Instant::now();
            let mut finished_exit_code: Option<Option<i32>> = None;
            let mut post_finish_idle_count = 0;
            let post_finish_idle_required = 4; // 200ms of idle after finish
            let mut timed_out = false;
            let mut timeout_interrupt_deadline: Option<tokio::time::Instant> = None;

            loop {
                if !timed_out {
                    if let Some(timeout_dur) = timeout_duration {
                        if start_time.elapsed() > timeout_dur {
                            timed_out = true;
                            timeout_interrupt_deadline =
                                Some(tokio::time::Instant::now() + COMMAND_TIMEOUT_INTERRUPT_GRACE_MS);

                            debug!("Command timed out in session {}, sending SIGINT", session_id);
                            if let Err(err) = pty_service.signal(pty_id, "SIGINT").await {
                                warn!(
                                    "Failed to interrupt timed out command in session {}: {}",
                                    session_id, err
                                );
                            }
                        }
                    }
                } else if let Some(deadline) = timeout_interrupt_deadline {
                    if tokio::time::Instant::now() >= deadline {
                        let output = get_integration_output_snapshot(&session_integrations, &session_id).await;
                        let shell_state = get_post_command_terminal_state(&session_integrations, &session_id).await;
                        send(CommandStreamEvent::Completed {
                            exit_code: finished_exit_code.flatten(),
                            total_output: output,
                            completion_reason: CommandCompletionReason::TimedOut,
                            shell_state,
                        })
                        .await;
                        return;
                    }
                }

                tokio::time::sleep(poll_interval).await;

                // Get current state, output, and command finished flag
                let (state, output, cmd_finished, last_exit) = {
                    let integrations = session_integrations.read().await;
                    if let Some(integration) = integrations.get(&session_id) {
                        let output = integration.output().to_string();
                        let cmd_finished = integration.command_just_finished();
                        let last_exit = integration.last_exit_code();
                        (integration.state().clone(), output, cmd_finished, last_exit)
                    } else {
                        send(CommandStreamEvent::Error {
                            message: "Integration not found".to_string(),
                        })
                        .await;
                        return;
                    }
                };

                // If command just finished, record it even if state already changed
                if cmd_finished && finished_exit_code.is_none() {
                    finished_exit_code = Some(last_exit);
                    post_finish_idle_count = 0;
                    last_output_len = output.len();
                    // Clear the flag
                    let mut integrations = session_integrations.write().await;
                    if let Some(integration) = integrations.get_mut(&session_id) {
                        integration.clear_command_finished();
                    }
                }

                let output_len = output.len();

                if let Some(new_data) = compute_stream_output_delta(&mut last_sent_output, output.as_str()) {
                    send(CommandStreamEvent::Output { data: new_data }).await;
                }

                // Check if command finished
                match state {
                    CommandState::Finished { exit_code } => {
                        // First time seeing Finished state - record it
                        if finished_exit_code.is_none() {
                            finished_exit_code = Some(exit_code);
                            post_finish_idle_count = 0;
                            last_output_len = output_len;
                        } else {
                            // Wait for output to stabilize after finish
                            if output_len == last_output_len {
                                post_finish_idle_count += 1;
                                if post_finish_idle_count >= post_finish_idle_required {
                                    let shell_state =
                                        get_post_command_terminal_state(&session_integrations, &session_id).await;
                                    send(CommandStreamEvent::Completed {
                                        exit_code: finished_exit_code.flatten(),
                                        total_output: output,
                                        completion_reason: if timed_out {
                                            CommandCompletionReason::TimedOut
                                        } else {
                                            CommandCompletionReason::Completed
                                        },
                                        shell_state,
                                    })
                                    .await;
                                    return;
                                }
                            } else {
                                post_finish_idle_count = 0;
                                last_output_len = output_len;
                            }
                        }
                    }
                    CommandState::Idle | CommandState::Prompt | CommandState::Input => {
                        // If we previously saw Finished and now see Prompt, we're done
                        // But wait for output to stabilize first (fix for intermittent output loss)
                        if finished_exit_code.is_some() {
                            if output_len == last_output_len {
                                post_finish_idle_count += 1;
                                // Wait at least 10 poll cycles (500ms) after seeing Prompt to ensure all output arrived
                                if post_finish_idle_count >= 10 {
                                    let shell_state =
                                        get_post_command_terminal_state(&session_integrations, &session_id).await;
                                    send(CommandStreamEvent::Completed {
                                        exit_code: finished_exit_code.flatten(),
                                        total_output: output,
                                        completion_reason: if timed_out {
                                            CommandCompletionReason::TimedOut
                                        } else {
                                            CommandCompletionReason::Completed
                                        },
                                        shell_state,
                                    })
                                    .await;
                                    return;
                                }
                            } else {
                                // New output arrived, reset counter
                                post_finish_idle_count = 0;
                                last_output_len = output_len;
                            }
                        } else {
                            // No finished_exit_code yet, use idle detection as fallback
                            if output_len == last_output_len {
                                idle_count += 1;
                                if idle_count >= max_idle_checks {
                                    let shell_state =
                                        get_post_command_terminal_state(&session_integrations, &session_id).await;
                                    send(CommandStreamEvent::Completed {
                                        exit_code: None,
                                        total_output: output,
                                        completion_reason: if timed_out {
                                            CommandCompletionReason::TimedOut
                                        } else {
                                            CommandCompletionReason::Completed
                                        },
                                        shell_state,
                                    })
                                    .await;
                                    return;
                                }
                            } else {
                                idle_count = 0;
                                last_output_len = output_len;
                            }
                        }
                    }

                    CommandState::Executing => {
                        idle_count = 0;
                        finished_exit_code = None;
                        last_output_len = output_len;
                    }
                }
            }
        });

        // Convert receiver to stream
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
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
    pub async fn send_command(&self, session_id: &str, command: &str) -> TerminalResult<()> {
        // Wait for session to be active
        self.wait_for_session_active(session_id).await?;

        // Format the command with carriage return
        let cmd_to_send = format!("{}\r", command);

        // Send the command
        self.write(session_id, cmd_to_send.as_bytes()).await
    }

    /// Wait for a session to become active (simpler than wait_for_session_ready)
    ///
    /// This only checks that the session exists and is in Active status.
    /// It does NOT require shell integration.
    async fn wait_for_session_active(&self, session_id: &str) -> TerminalResult<()> {
        let ready_timeout = Duration::from_secs(30);
        let ready_start = std::time::Instant::now();

        while ready_start.elapsed() < ready_timeout {
            let session_status = {
                let sessions = self.sessions.read().await;
                sessions.get(session_id).map(|s| s.status.clone())
            };

            match session_status {
                Some(SessionStatus::Active) => {
                    return Ok(());
                }
                Some(SessionStatus::Terminating) | Some(SessionStatus::Exited { .. }) => {
                    return Err(TerminalError::Session(format!("Session {} is terminated", session_id)));
                }
                Some(SessionStatus::Starting)
                | Some(SessionStatus::Orphaned)
                | Some(SessionStatus::Restoring)
                | None => {
                    // Still starting, restoring, or not found yet, wait
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Err(TerminalError::Session(format!(
            "Session {} did not become active in {:?}",
            session_id, ready_timeout
        )))
    }
}
