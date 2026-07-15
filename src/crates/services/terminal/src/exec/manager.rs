//! `ExecProcessManager` facade.
//!
//! Public `impl ExecProcessManager` lives here; sub-domain method bodies
//! (exec_command, stdin, control_session) live in sibling files in `manager/`
//! subdir and are called via free functions taking `&ExecProcessManager`.
//! `impl ExecProcess` + `impl Drop for ExecProcess` live in
//! `manager/exec_process.rs`.

mod command_exec;
mod control_session;
mod exec_process;
mod stdin;

use crate::TerminalResult;
use tokio::sync::mpsc;

use self::command_exec::exec_command_inner_impl;
use self::control_session::control_session_impl;
use self::stdin::{send_stdin_impl, write_stdin_inner_impl};

use super::types::{
    ExecCommandRequest, ExecCommandResponse, ExecControlRequest, ExecProcessManager, SendStdinRequest,
    WriteStdinRequest,
};

impl ExecProcessManager {
    pub async fn exec_command(&self, request: ExecCommandRequest) -> TerminalResult<ExecCommandResponse> {
        exec_command_inner_impl(self, request, None).await
    }

    pub async fn exec_command_streaming(
        &self,
        request: ExecCommandRequest,
        output_tx: mpsc::Sender<String>,
    ) -> TerminalResult<ExecCommandResponse> {
        exec_command_inner_impl(self, request, Some(output_tx)).await
    }

    pub async fn write_stdin(&self, request: WriteStdinRequest) -> TerminalResult<ExecCommandResponse> {
        write_stdin_inner_impl(self, request, None).await
    }

    pub async fn write_stdin_streaming(
        &self,
        request: WriteStdinRequest,
        output_tx: mpsc::Sender<String>,
    ) -> TerminalResult<ExecCommandResponse> {
        write_stdin_inner_impl(self, request, Some(output_tx)).await
    }

    pub async fn send_stdin(&self, request: SendStdinRequest) -> TerminalResult<()> {
        send_stdin_impl(self, request).await
    }

    pub async fn control_session(&self, request: ExecControlRequest) -> TerminalResult<ExecCommandResponse> {
        control_session_impl(self, request).await
    }
}
