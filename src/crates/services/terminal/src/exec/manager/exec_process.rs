//! `ExecProcess` implementation block + `Drop` impl.
//!
//! Owned by `ExecProcessManager` but methods are exercised via `Arc<ExecProcess>`.

use super::super::platform::*;
use super::super::types::*;
use crate::{TerminalError, TerminalResult};
use tracing::warn;

impl Drop for ExecProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl ExecProcess {
    pub(super) fn mark_out_of_band_control(&self, action: ExecControlAction) {
        if let Ok(mut out_of_band_action) = self.out_of_band_control_action.lock() {
            *out_of_band_action = Some(action);
        }
    }

    pub(crate) fn out_of_band_control_action(&self) -> Option<ExecControlAction> {
        self.out_of_band_control_action.lock().ok().and_then(|action| *action)
    }

    pub(super) async fn write_input_bytes(&self, bytes: Vec<u8>) -> TerminalResult<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let writer = self.writer.as_ref().ok_or(TerminalError::ProcessNotRunning)?;
        writer.send(bytes).await.map_err(|_| TerminalError::ProcessNotRunning)
    }

    pub(super) fn request_control(&self, action: ExecControlAction) {
        if let Ok(mut terminator) = self.terminator.lock() {
            if let Some(terminator) = terminator.take() {
                match terminator {
                    Terminator::Pty(mut killer) => {
                        if let Err(e) = killer.kill() {
                            warn!("Failed to kill pty process: {e}");
                        }
                    }
                    Terminator::Pipe(tx) => {
                        self.close_windows_pipe_job("request_control");
                        if let Err(e) = tx.try_send(action) {
                            warn!("Failed to send control action to pipe: {e}");
                        }
                    }
                }
            }
        }
    }

    pub(super) fn request_terminate(&self) {
        self.request_control(ExecControlAction::Kill);
    }

    pub(super) fn terminate(&self) {
        self.request_terminate();
        self.close_windows_pipe_job("terminate");

        if let Ok(mut tasks) = self.helper_tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }

        if let Ok(mut handles) = self.pty_handles.lock() {
            handles.take();
        }
    }

    #[cfg(windows)]
    pub(super) fn close_windows_pipe_job(&self, reason: &str) {
        if let Some(pipe_job) = &self.pipe_job {
            let _ = close_windows_pipe_job_handle(pipe_job, reason); // intentionally ignored: best-effort cleanup
        }
    }

    #[cfg(not(windows))]
    pub(super) fn close_windows_pipe_job(&self, _reason: &str) {}
}
