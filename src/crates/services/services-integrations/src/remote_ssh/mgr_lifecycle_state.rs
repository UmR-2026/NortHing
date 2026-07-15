//! Remote command execution lifecycle over an existing SSH handle.
//!
//! Owns `execute_command_internal` plus its three phase helpers:
//!
//! - Phase 1 `execute_open_channel` opens a russh exec channel and sends the
//!   command.
//! - Phase 2 `execute_pump_loop` is the cancellation-aware event loop: it
//!   pumps the channel until exit, timeout, or cancellation, collecting
//!   stdout/stderr and the exit code.
//! - Phase 3 `execute_finalize_result` applies the exit-code fallback logic
//!   (`-1` if unknown, `124` on timeout, `130` on interrupt) and emits the
//!   completion log.
//!
//! The pump loop relies on `SSH_COMMAND_INTERRUPT_DRAIN_GRACE` and
//! `SSH_COMMAND_WAIT_POLL_INTERVAL` (declared in `manager.rs`) plus
//! `interrupt_exec_channel` (declared in `mgr_lifecycle_handlers`).

use crate::remote_ssh::manager::{
    truncate_at_char_boundary, SSHConnectionManager, SSH_COMMAND_INTERRUPT_DRAIN_GRACE, SSH_COMMAND_WAIT_POLL_INTERVAL,
};
use crate::remote_ssh::manager_handler::SSHHandler;
use crate::remote_ssh::types::{SSHCommandOptions, SSHCommandResult};
use russh::client::{Handle, Msg};
use russh::Sig;
use std::time::Instant;
use tokio::time::Duration;

impl SSHConnectionManager {
    /// Run a remote command over an existing handle and collect stdout/stderr/exit status,
    /// honouring `options.timeout_ms` and `options.cancellation_token`.
    pub(super) async fn execute_command_internal(
        handle: &Handle<SSHHandler>,
        command: &str,
        options: SSHCommandOptions,
    ) -> std::result::Result<SSHCommandResult, anyhow::Error> {
        let mut session = Self::execute_open_channel(handle, command).await?;
        let (stdout, stderr, exit_status, interrupted, timed_out) =
            Self::execute_pump_loop(&mut session, command, &options).await;
        Self::execute_finalize_result(stdout, stderr, exit_status, interrupted, timed_out)
    }

    /// Phase 1 of `execute_command_internal`: open an exec channel and send the command.
    async fn execute_open_channel(handle: &Handle<SSHHandler>, command: &str) -> anyhow::Result<russh::Channel<Msg>> {
        let session = handle.channel_open_session().await?;
        session.exec(true, command).await?;
        Ok(session)
    }

    /// Phase 2 of `execute_command_internal`: pump the channel until exit, timeout,
    /// or cancellation, collecting stdout/stderr and the exit code.
    async fn execute_pump_loop(
        session: &mut russh::Channel<Msg>,
        command: &str,
        options: &SSHCommandOptions,
    ) -> (String, String, Option<i32>, bool, bool) {
        let execution_started_at = Instant::now();
        let command_preview = if command.len() > 160 {
            format!("{}...", truncate_at_char_boundary(command, 160))
        } else {
            command.to_string()
        };
        tracing::debug!(
            "Remote exec started: timeout_ms={:?}, has_cancellation={}, command_preview={}",
            options.timeout_ms,
            options.cancellation_token.is_some(),
            command_preview
        );
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_status: Option<i32> = None;
        let mut interrupted = false;
        let mut timed_out = false;
        let stdout_first_chunk_once = std::sync::Once::new();
        let stderr_first_chunk_once = std::sync::Once::new();
        let mut eof_logged = false;
        let mut close_logged = false;
        let timeout_deadline = options.timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
        let mut interrupt_drain_deadline: Option<Instant> = None;

        loop {
            let now = Instant::now();

            if !interrupted
                && options
                    .cancellation_token
                    .as_ref()
                    .is_some_and(|token| token.is_cancelled())
            {
                interrupted = true;
                interrupt_drain_deadline = Some(now + SSH_COMMAND_INTERRUPT_DRAIN_GRACE);
                tracing::warn!(
                    "Remote exec cancellation requested: timeout_ms={:?}, stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                    options.timeout_ms,
                    stdout.len(),
                    stderr.len(),
                    execution_started_at.elapsed().as_millis(),
                    command_preview
                );
                if let Err(e) = Self::interrupt_exec_channel(session, Sig::INT).await {
                    tracing::debug!("Failed to interrupt remote exec channel via SIGINT: {}", e);
                }
            }

            if !timed_out && timeout_deadline.is_some_and(|deadline| now >= deadline) {
                timed_out = true;
                interrupt_drain_deadline = Some(now + SSH_COMMAND_INTERRUPT_DRAIN_GRACE);
                tracing::warn!(
                    "Remote exec timeout reached: timeout_ms={:?}, stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                    options.timeout_ms,
                    stdout.len(),
                    stderr.len(),
                    execution_started_at.elapsed().as_millis(),
                    command_preview
                );
                if let Err(e) = Self::interrupt_exec_channel(session, Sig::INT).await {
                    tracing::debug!("Failed to interrupt timed out remote exec channel: {}", e);
                }
            }

            let wait_budget = if let Some(deadline) = interrupt_drain_deadline {
                if now >= deadline {
                    let _ = session.close().await;
                    break;
                }
                (deadline - now).min(SSH_COMMAND_WAIT_POLL_INTERVAL)
            } else if let Some(deadline) = timeout_deadline {
                if now >= deadline {
                    SSH_COMMAND_WAIT_POLL_INTERVAL
                } else {
                    (deadline - now).min(SSH_COMMAND_WAIT_POLL_INTERVAL)
                }
            } else {
                SSH_COMMAND_WAIT_POLL_INTERVAL
            };

            let next_msg = match tokio::time::timeout(wait_budget, session.wait()).await {
                Ok(msg) => msg,
                Err(_) => continue,
            };

            match next_msg {
                Some(russh::ChannelMsg::Data { ref data }) => {
                    stdout_first_chunk_once.call_once(|| {
                        tracing::debug!(
                            "Remote exec first stdout chunk received: timeout_ms={:?}, chunk_len={}, duration_ms={}, command_preview={}",
                            options.timeout_ms,
                            data.len(),
                            execution_started_at.elapsed().as_millis(),
                            command_preview
                        );
                    });
                    stdout.push_str(&String::from_utf8_lossy(data));
                }
                Some(russh::ChannelMsg::ExtendedData { ref data, .. }) => {
                    stderr_first_chunk_once.call_once(|| {
                        tracing::debug!(
                            "Remote exec first stderr chunk received: timeout_ms={:?}, chunk_len={}, duration_ms={}, command_preview={}",
                            options.timeout_ms,
                            data.len(),
                            execution_started_at.elapsed().as_millis(),
                            command_preview
                        );
                    });
                    stderr.push_str(&String::from_utf8_lossy(data));
                }
                Some(russh::ChannelMsg::ExitStatus { exit_status: status }) => {
                    exit_status = Some(status as i32);
                    tracing::debug!(
                        "Remote exec exit status received: exit_code={}, stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                        status,
                        stdout.len(),
                        stderr.len(),
                        execution_started_at.elapsed().as_millis(),
                        command_preview
                    );
                }
                Some(russh::ChannelMsg::ExitSignal { signal_name, .. }) => {
                    interrupted = interrupted || matches!(signal_name, Sig::INT | Sig::TERM);
                    tracing::debug!(
                        "Remote exec exit signal received: signal={:?}, stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                        signal_name,
                        stdout.len(),
                        stderr.len(),
                        execution_started_at.elapsed().as_millis(),
                        command_preview
                    );
                }
                Some(russh::ChannelMsg::Eof) => {
                    if !eof_logged {
                        eof_logged = true;
                        tracing::debug!(
                            "Remote exec EOF received: stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                            stdout.len(),
                            stderr.len(),
                            execution_started_at.elapsed().as_millis(),
                            command_preview
                        );
                    }
                }
                Some(russh::ChannelMsg::Close) => {
                    if !close_logged {
                        close_logged = true;
                        tracing::debug!(
                            "Remote exec channel close received: stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                            stdout.len(),
                            stderr.len(),
                            execution_started_at.elapsed().as_millis(),
                            command_preview
                        );
                    }
                }
                None => {
                    tracing::debug!(
                        "Remote exec stream ended: stdout_len={}, stderr_len={}, duration_ms={}, command_preview={}",
                        stdout.len(),
                        stderr.len(),
                        execution_started_at.elapsed().as_millis(),
                        command_preview
                    );
                    break;
                }
                Some(_) => {}
            }
        }

        (stdout, stderr, exit_status, interrupted, timed_out)
    }

    /// Phase 3 of `execute_command_internal`: apply the exit-code fallback logic
    /// (`-1` if unknown, `124` on timeout, `130` on interrupt) and emit the
    /// completion log.
    fn execute_finalize_result(
        stdout: String,
        stderr: String,
        exit_status: Option<i32>,
        interrupted: bool,
        timed_out: bool,
    ) -> std::result::Result<SSHCommandResult, anyhow::Error> {
        let result = SSHCommandResult {
            stdout,
            stderr,
            exit_code: exit_status.unwrap_or({
                if timed_out {
                    124
                } else if interrupted {
                    130
                } else {
                    -1
                }
            }),
            interrupted,
            timed_out,
        };
        tracing::debug!(
            "Remote exec completed: exit_code={}, interrupted={}, timed_out={}, stdout_len={}, stderr_len={}",
            result.exit_code,
            result.interrupted,
            result.timed_out,
            result.stdout.len(),
            result.stderr.len()
        );
        Ok(result)
    }
}
