//! Sub-domain: stdin operations — write_stdin / write_stdin_streaming / send_stdin.
//!
//! Implements `ExecProcessManager::write_stdin_inner` and `send_stdin` as free
//! functions called by the public delegates in `manager.rs`.

use std::sync::Arc;

use super::super::output::*;
use super::super::platform::*;
use super::super::types::*;
use super::control_session::{emit_lifecycle_impl, remove_session_impl, take_completed_session_impl};
use crate::{TerminalError, TerminalResult};
use tokio::sync::mpsc;

pub(super) async fn write_stdin_inner_impl(
    mgr: &ExecProcessManager,
    request: WriteStdinRequest,
    output_tx: Option<mpsc::Sender<String>>,
) -> TerminalResult<ExecCommandResponse> {
    let (process, tty, cursor, lifecycle_tx) = {
        let mut sessions = mgr.sessions.lock().await;
        let Some(entry) = sessions.get_mut(&request.session_id) else {
            drop(sessions);
            if request.chars.is_empty() {
                if let Some(completed) = take_completed_session_impl(mgr, request.session_id).await {
                    return Ok(ExecCommandResponse {
                        chunk_id: new_chunk_id(),
                        wall_time_seconds: 0.0,
                        output: completed.output,
                        session_id: None,
                        exit_code: completed.exit_code,
                        original_output_chars: completed.original_output_chars,
                        completion: Some(completed.completion),
                    });
                }
            }
            return Err(TerminalError::SessionNotFound(request.session_id.to_string()));
        };
        entry.last_used = tokio::time::Instant::now();
        (
            Arc::clone(&entry.process),
            entry.tty,
            entry.cursor.clone(),
            entry.lifecycle_tx.clone(),
        )
    };

    let input = input_bytes_for_write(&request.chars, request.append_enter);
    if !input.is_empty() && tty {
        let writer = process.writer.as_ref().ok_or(TerminalError::ProcessNotRunning)?;
        writer.send(input).await.map_err(|_| TerminalError::ProcessNotRunning)?;
    }

    let started_at = tokio::time::Instant::now();
    let collected = process
        .output
        .collect_until(
            cursor,
            deadline_from_now(request.yield_time_ms),
            request.max_output_chars.unwrap_or(usize::MAX),
            output_tx.as_ref(),
        )
        .await;

    let closed = process.output.is_closed().await;
    let exit_code = process.output.exit_code().await;
    let completion = if closed {
        Some(completion_for_closed_process(process.out_of_band_control_action()))
    } else {
        None
    };
    if closed {
        emit_lifecycle_impl(
            lifecycle_tx,
            ExecProcessLifecycleEvent {
                session_id: request.session_id,
                status: lifecycle_status_for_completion(
                    completion.expect("closed process should have completion").status,
                ),
                exit_code,
            },
        );
        remove_session_impl(mgr, request.session_id).await;
    } else {
        let mut sessions = mgr.sessions.lock().await;
        if let Some(entry) = sessions.get_mut(&request.session_id) {
            entry.cursor = collected.cursor.clone();
        }
    }

    Ok(ExecCommandResponse {
        chunk_id: new_chunk_id(),
        wall_time_seconds: started_at.elapsed().as_secs_f64(),
        output: collected.output,
        session_id: (!closed).then_some(request.session_id),
        exit_code,
        original_output_chars: collected.original_output_chars,
        completion,
    })
}

pub(super) async fn send_stdin_impl(mgr: &ExecProcessManager, request: SendStdinRequest) -> TerminalResult<()> {
    let (process, tty) = {
        let mut sessions = mgr.sessions.lock().await;
        let entry = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(request.session_id.to_string()))?;
        entry.last_used = tokio::time::Instant::now();
        (Arc::clone(&entry.process), entry.tty)
    };

    let input = input_bytes_for_write(&request.chars, request.append_enter);
    if input.is_empty() {
        return Ok(());
    }
    if !tty {
        return Err(TerminalError::InvalidConfig(
            "stdin input requires a tty session".to_string(),
        ));
    }

    process.write_input_bytes(input).await
}
