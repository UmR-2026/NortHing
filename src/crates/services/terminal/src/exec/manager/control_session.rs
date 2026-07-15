//! Sub-domain: control_session + session storage helpers.
//!
//! Implements `ExecProcessManager::control_session` and the session map helpers
//! (store_session, remove_session, update_session_cursor, store_completed_session,
//! take_completed_session) as free functions called by sibling delegates and
//! each other within `manager/`.

use std::sync::Arc;

use super::super::output::*;
use super::super::platform::*;
use super::super::types::*;
use crate::{TerminalError, TerminalResult};
use tokio::sync::mpsc;

pub(super) async fn control_session_impl(
    mgr: &ExecProcessManager,
    request: ExecControlRequest,
) -> TerminalResult<ExecCommandResponse> {
    let (process, tty, cursor, lifecycle_tx) = {
        let mut sessions = mgr.sessions.lock().await;
        let entry = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| crate::TerminalError::SessionNotFound(request.session_id.to_string()))?;
        entry.last_used = tokio::time::Instant::now();
        if request.origin == ExecControlOrigin::OutOfBand {
            entry.process.mark_out_of_band_control(request.action);
        }
        (
            Arc::clone(&entry.process),
            entry.tty,
            entry.cursor.clone(),
            entry.lifecycle_tx.clone(),
        )
    };

    match request.action {
        ExecControlAction::Interrupt if tty => {
            process.write_input_bytes(vec![0x03]).await?;
        }
        ExecControlAction::Interrupt | ExecControlAction::Kill => {
            process.request_control(request.action);
        }
    }

    let started_at = tokio::time::Instant::now();
    let collected = process
        .output
        .collect_until(
            cursor,
            deadline_from_now(request.yield_time_ms),
            request.max_output_chars.unwrap_or(usize::MAX),
            None,
        )
        .await;

    let closed = process.output.is_closed().await;
    let exit_code = process.output.exit_code().await;
    let completion = closed.then_some(ExecSessionCompletion {
        status: completion_status_for_control_action(request.action),
        source: match request.origin {
            ExecControlOrigin::ModelTool => ExecSessionCompletionSource::Process,
            ExecControlOrigin::OutOfBand => ExecSessionCompletionSource::OutOfBandControl,
        },
    });
    if closed {
        let status = lifecycle_status_for_completion(completion.expect("closed process should have completion").status);
        emit_lifecycle_impl(
            lifecycle_tx,
            ExecProcessLifecycleEvent {
                session_id: request.session_id,
                status,
                exit_code,
            },
        );
        remove_session_impl(mgr, request.session_id).await;
        if request.origin == ExecControlOrigin::OutOfBand {
            store_completed_session_impl(
                mgr,
                request.session_id,
                CompletedExecSession {
                    output: collected.output.clone(),
                    exit_code,
                    original_output_chars: collected.original_output_chars,
                    completion: completion.expect("closed process should have completion"),
                    completed_at: tokio::time::Instant::now(),
                },
            )
            .await;
        }
    } else {
        if request.origin == ExecControlOrigin::ModelTool {
            let mut sessions = mgr.sessions.lock().await;
            if let Some(entry) = sessions.get_mut(&request.session_id) {
                entry.cursor = collected.cursor.clone();
            }
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

pub(super) async fn store_session_impl(
    mgr: &ExecProcessManager,
    process: Arc<ExecProcess>,
    tty: bool,
    cursor: OutputCursor,
    lifecycle_tx: Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>>,
) -> i32 {
    let (session_id, pruned_entry) = {
        let mut sessions = mgr.sessions.lock().await;
        let pruned = if sessions.len() >= MAX_EXEC_SESSIONS {
            sessions
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(id, _)| *id)
                .and_then(|id| sessions.remove(&id).map(|entry| (id, entry)))
        } else {
            None
        };
        let session_id = new_session_id(&sessions);
        sessions.insert(
            session_id,
            ExecSessionEntry {
                process: Arc::clone(&process),
                tty,
                cursor,
                last_used: tokio::time::Instant::now(),
                lifecycle_tx: lifecycle_tx.clone(),
            },
        );
        (session_id, pruned)
    };

    if let Some((pruned_session_id, entry)) = pruned_entry {
        emit_lifecycle_impl(
            entry.lifecycle_tx.clone(),
            ExecProcessLifecycleEvent {
                session_id: pruned_session_id,
                status: ExecProcessLifecycleStatus::Pruned,
                exit_code: None,
            },
        );
        entry.process.terminate();
    }

    emit_lifecycle_impl(
        lifecycle_tx.clone(),
        ExecProcessLifecycleEvent {
            session_id,
            status: ExecProcessLifecycleStatus::Running,
            exit_code: None,
        },
    );
    spawn_lifecycle_exit_watcher(session_id, process, lifecycle_tx);

    session_id
}

pub(super) async fn remove_session_impl(mgr: &ExecProcessManager, session_id: i32) {
    let mut sessions = mgr.sessions.lock().await;
    sessions.remove(&session_id);
}

pub(super) async fn update_session_cursor_impl(mgr: &ExecProcessManager, session_id: i32, cursor: OutputCursor) {
    let mut sessions = mgr.sessions.lock().await;
    if let Some(entry) = sessions.get_mut(&session_id) {
        entry.cursor = cursor;
    }
}

pub(super) async fn store_completed_session_impl(
    mgr: &ExecProcessManager,
    session_id: i32,
    completed: CompletedExecSession,
) {
    let mut completed_sessions = mgr.completed_sessions.lock().await;
    if completed_sessions.len() >= MAX_COMPLETED_EXEC_SESSIONS {
        if let Some(oldest_session_id) = completed_sessions
            .iter()
            .min_by_key(|(_, session)| session.completed_at)
            .map(|(id, _)| *id)
        {
            completed_sessions.remove(&oldest_session_id);
        }
    }
    completed_sessions.insert(session_id, completed);
}

pub(super) async fn take_completed_session_impl(
    mgr: &ExecProcessManager,
    session_id: i32,
) -> Option<CompletedExecSession> {
    mgr.completed_sessions.lock().await.remove(&session_id)
}

/// Local re-export of `super::super::output::emit_lifecycle` for use by sibling
/// files in `manager/` (command_exec, stdin, control_session).
pub(super) use super::super::output::emit_lifecycle as emit_lifecycle_impl;
