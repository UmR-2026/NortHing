//! Sub-domain: exec_command / exec_command_streaming.
//!
//! Implements `ExecProcessManager::exec_command_inner` as a free function
//! called by the public delegates in `manager.rs`.

use std::sync::Arc;

use super::super::output::*;
use super::super::platform::*;
use super::super::types::*;
use super::control_session::{remove_session_impl, store_session_impl, update_session_cursor_impl};
use crate::TerminalResult;
use tokio::sync::mpsc;

pub(super) async fn exec_command_inner_impl(
    mgr: &ExecProcessManager,
    request: ExecCommandRequest,
    output_tx: Option<mpsc::Sender<String>>,
) -> TerminalResult<ExecCommandResponse> {
    let process = Arc::new(spawn_exec_process(&request).await?);
    let cursor = OutputCursor { next_seq: 0 };
    let session_id = store_session_impl(
        mgr,
        Arc::clone(&process),
        request.tty,
        cursor.clone(),
        request.lifecycle_tx,
    )
    .await;
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

    let exit_code = process.output.exit_code().await;
    let closed = process.output.is_closed().await;
    let completion = if closed {
        Some(completion_for_closed_process(process.out_of_band_control_action()))
    } else {
        None
    };
    if closed {
        remove_session_impl(mgr, session_id).await;
    } else {
        update_session_cursor_impl(mgr, session_id, collected.cursor.clone()).await;
    }

    Ok(ExecCommandResponse {
        chunk_id: new_chunk_id(),
        wall_time_seconds: started_at.elapsed().as_secs_f64(),
        output: collected.output,
        session_id: (!closed).then_some(session_id),
        exit_code,
        original_output_chars: collected.original_output_chars,
        completion,
    })
}
