//! Remote command logic for [`ExecCommandTool`].
//!
//! Splits the original `impl ExecCommandTool` block into the pieces that only
//! run when the active workspace is remote (SSH-backed). The local
//! equivalents live in `local.rs`. The single entry point executed by the
//! tool dispatcher is [`ExecCommandTool::call_remote_pipe`]; everything else
//! is a helper that the remote call composes.

use std::collections::HashMap;

use serde_json::{json, Value};
use terminal_core::ShellType;
use tokio::sync::mpsc;

use super::super::background_command_output::{
    background_command_output_capture, BackgroundCommandOutputStatus, StartBackgroundCommandOutputCapture,
};
use super::super::env_snapshot::{remote_env_snapshot_for, RemoteEnvSnapshot};
use super::super::progress::ExecOutputProgressBridge;
use super::shell_helpers::{now_unix_seconds, remote_command_env_words, remote_shell_login_args, shell_escape};
pub(super) use super::tool::ExecCommandTool;
use super::types::{
    RemoteShell, DEFAULT_TOOL_YIELD_TIME_MS, REMOTE_NON_TTY_INTERRUPT_GRACE_SECONDS, REMOTE_SHELL_PROBE_TIMEOUT_MS,
};
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::infrastructure::events::event_system::{global_event_system, BackendEvent::BackgroundCommandLifecycle};
use crate::service::remote_ssh::{
    global_remote_exec_process_manager, remote_workspace_manager, RemoteExecCommandRequest,
    RemoteExecProcessLifecycleEvent, RemoteExecProcessLifecycleStatus, RemoteExecSessionCompletion,
    RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus, SSHCommandOptions, SSHConnectionManager,
};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::event::BackgroundCommandLifecycleInfo;

/// Resolve the remote `workdir` parameter: explicit absolute path wins, else
/// the workspace root. The path must be absolute (starts with `/`) and must
/// be an existing directory in the remote workspace filesystem.
pub(super) async fn resolve_remote_workdir(input: &Value, context: &ToolUseContext) -> NortHingResult<String> {
    let raw = input
        .get("workdir")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|workdir| !workdir.is_empty())
        .map(str::to_string)
        .or_else(|| context.workspace_root().map(|path| path.to_string_lossy().to_string()))
        .ok_or_else(|| NortHingError::tool("workspace root is required for ExecCommand".to_string()))?;

    if !raw.starts_with('/') {
        return Err(NortHingError::tool(
            "workdir must be an absolute remote path for ExecCommand".to_string(),
        ));
    }

    let resolved = context.resolve_workspace_tool_path(&raw)?;
    let fs = context
        .ws_fs()
        .ok_or_else(|| NortHingError::tool("remote workspace filesystem is required for ExecCommand".to_string()))?;
    let is_dir = fs
        .is_dir(&resolved)
        .await
        .map_err(|error| NortHingError::tool(format!("failed to check remote workdir '{}': {}", resolved, error)))?;
    if !is_dir {
        return Err(NortHingError::tool(format!(
            "remote workdir does not exist or is not a directory: {}",
            resolved
        )));
    }
    Ok(resolved)
}

/// Probe the remote target for the user's login shell by running a small
/// POSIX script that prints candidate shell paths. Falls back to
/// `/bin/bash` on any error.
pub(super) async fn resolve_remote_shell(ssh_manager: &SSHConnectionManager, connection_id: &str) -> RemoteShell {
    let probe_command = concat!(
        "printf '%s\\n' \"${SHELL:-}\"; ",
        "getent passwd \"$(id -un)\" 2>/dev/null | cut -d: -f7; ",
        "command -v bash 2>/dev/null; ",
        "command -v zsh 2>/dev/null; ",
        "command -v sh 2>/dev/null"
    );
    let result = ssh_manager
        .execute_command_with_options(
            connection_id,
            probe_command,
            SSHCommandOptions {
                timeout_ms: Some(REMOTE_SHELL_PROBE_TIMEOUT_MS),
                cancellation_token: None,
            },
        )
        .await;

    if let Ok(result) = result {
        if !result.timed_out && !result.interrupted && result.exit_code == 0 {
            if let Some(shell) = super::shell_helpers::parse_remote_shell_probe_output(&result.stdout) {
                return shell;
            }
        }
    }

    RemoteShell {
        path: "/bin/bash".to_string(),
        shell_type: ShellType::Bash,
    }
}

/// Compose the actual command line sent over SSH. The workdir is `cd`-ed
/// into first, the merged environment is exported via `env KEY=VAL ...`,
/// the resolved remote shell is invoked as a login shell, and the user
/// command is shell-escaped.
pub(super) fn remote_login_shell_command(
    workdir: &str,
    cmd: &str,
    shell: &RemoteShell,
    env_snapshot: Option<&RemoteEnvSnapshot>,
) -> String {
    let env_words = remote_command_env_words(merged_remote_env(env_snapshot));
    let shell_args = remote_shell_login_args().join(" ");

    format!(
        "cd {} && env {} {} {} {}",
        shell_escape(workdir),
        env_words,
        shell_escape(&shell.path),
        shell_args,
        shell_escape(cmd)
    )
}

/// Wrap a command so that it runs detached under `setsid` (or as a plain
/// background process when `setsid` is unavailable) and can be signalled by
/// the orchestrator. The wrapper installs INT/TERM traps that forward to
/// the child process group, then escalate to `KILL` after
/// [`REMOTE_NON_TTY_INTERRUPT_GRACE_SECONDS`].
pub(super) fn remote_non_tty_control_wrapper(cmd: &str, shell_path: &str) -> String {
    let escaped_shell = shell_escape(shell_path);
    let escaped_cmd = shell_escape(cmd);
    format!(
        r#"__northhing_shell={escaped_shell}
__northhing_cmd={escaped_cmd}
if command -v setsid >/dev/null 2>&1; then
  setsid "$__northhing_shell" -lc "$__northhing_cmd" &
else
  "$__northhing_shell" -lc "$__northhing_cmd" &
fi
__northhing_child=$!
__northhing_pgid=$__northhing_child
__northhing_stop() {{
  __northhing_signal=${{1:-INT}}
  __northhing_exit=${{2:-130}}
  __northhing_grace=${{3:-{REMOTE_NON_TTY_INTERRUPT_GRACE_SECONDS}}}
  trap - INT TERM
  kill -"$__northhing_signal" "-$__northhing_pgid" 2>/dev/null || kill -"$__northhing_signal" "$__northhing_child" 2>/dev/null || true
  if [ "$__northhing_grace" -gt 0 ]; then
    sleep "$__northhing_grace"
  fi
  kill -KILL "-$__northhing_pgid" 2>/dev/null || kill -KILL "$__northhing_child" 2>/dev/null || true
  wait "$__northhing_child" 2>/dev/null || true
  exit "$__northhing_exit"
}}
trap '__northhing_stop INT 130 {REMOTE_NON_TTY_INTERRUPT_GRACE_SECONDS}' INT
trap '__northhing_stop KILL 137 0' TERM
wait "$__northhing_child"
__northhing_status=$?
trap - INT TERM
exit "$__northhing_status""#
    )
}

/// Layer the tool-supplied [`super::local::command_env`] on top of the
/// remote environment snapshot. The snapshot's values win for keys it
/// covers (e.g. `PATH`, `TERM`), and the tool layer fills in the rest.
pub(super) fn merged_remote_env(env_snapshot: Option<&RemoteEnvSnapshot>) -> HashMap<String, String> {
    let mut env = env_snapshot.map(|snapshot| snapshot.env.clone()).unwrap_or_default();
    env.extend(super::local::command_env());
    env
}

/// JSON metadata for the resolved remote shell embedded in the tool result.
pub(super) fn remote_shell_metadata(workdir: &str, shell: &RemoteShell, env_snapshot_applied: bool) -> Value {
    json!({
        "name": shell.shell_type.name(),
        "type": shell.shell_type.to_string(),
        "path": shell.path,
        "invocation": format!(
            "`cd {} && env ... {} {} <cmd>`",
            shell_escape(workdir),
            shell_escape(&shell.path),
            remote_shell_login_args().join(" ")
        ),
        "remote_env_snapshot_applied": env_snapshot_applied,
    })
}

/// Render a [`RemoteExecSessionCompletion`] as the `{"status","source"}`
/// shape the rest of the tool result pipeline expects.
pub(super) fn remote_completion_value(completion: RemoteExecSessionCompletion) -> Value {
    json!({
        "status": match completion.status {
            RemoteExecSessionCompletionStatus::Exited => "exited",
            RemoteExecSessionCompletionStatus::Interrupted => "interrupted",
            RemoteExecSessionCompletionStatus::Killed => "killed",
            RemoteExecSessionCompletionStatus::Pruned => "pruned",
        },
        "source": match completion.source {
            RemoteExecSessionCompletionSource::Process => "process",
            RemoteExecSessionCompletionSource::OutOfBandControl => "out_of_band_control",
        },
    })
}

/// Map a terminal remote completion status to the
/// [`BackgroundCommandOutputStatus`] used by the capture.
pub(super) fn remote_background_output_status_for_completion(
    completion: Option<RemoteExecSessionCompletion>,
) -> BackgroundCommandOutputStatus {
    match completion.map(|completion| completion.status) {
        Some(RemoteExecSessionCompletionStatus::Interrupted) => BackgroundCommandOutputStatus::Interrupted,
        Some(RemoteExecSessionCompletionStatus::Killed) => BackgroundCommandOutputStatus::Killed,
        Some(RemoteExecSessionCompletionStatus::Pruned) => BackgroundCommandOutputStatus::Pruned,
        Some(RemoteExecSessionCompletionStatus::Exited) | None => BackgroundCommandOutputStatus::Exited,
    }
}

/// String label for a remote lifecycle status, used in the
/// `BackgroundCommandLifecycleInfo` event.
pub(super) fn remote_lifecycle_status(status: RemoteExecProcessLifecycleStatus) -> &'static str {
    match status {
        RemoteExecProcessLifecycleStatus::Running => "running",
        RemoteExecProcessLifecycleStatus::Exited => "exited",
        RemoteExecProcessLifecycleStatus::Interrupted => "interrupted",
        RemoteExecProcessLifecycleStatus::Killed => "killed",
        RemoteExecProcessLifecycleStatus::Pruned => "pruned",
    }
}

/// Map a remote mid-flight lifecycle status to the
/// [`BackgroundCommandOutputStatus`] the capture needs.
pub(super) fn remote_background_output_status(
    status: RemoteExecProcessLifecycleStatus,
) -> BackgroundCommandOutputStatus {
    match status {
        RemoteExecProcessLifecycleStatus::Running => BackgroundCommandOutputStatus::Running,
        RemoteExecProcessLifecycleStatus::Exited => BackgroundCommandOutputStatus::Exited,
        RemoteExecProcessLifecycleStatus::Interrupted => BackgroundCommandOutputStatus::Interrupted,
        RemoteExecProcessLifecycleStatus::Killed => BackgroundCommandOutputStatus::Killed,
        RemoteExecProcessLifecycleStatus::Pruned => BackgroundCommandOutputStatus::Pruned,
    }
}

/// Spawn the task that translates remote process lifecycle events into
/// `BackgroundCommandLifecycle` global events. Returns `None` when the
/// call has no `tool_call_id` to attach metadata to.
pub(super) fn start_remote_lifecycle_bridge(
    context: &ToolUseContext,
    _tool_name: &str,
) -> Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>> {
    let capture_id = context.tool_call_id.clone()?;
    let agent_session_id = context.session_id.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<RemoteExecProcessLifecycleEvent>();
    tokio::spawn(async move {
        let event_system = global_event_system();
        let output_capture = background_command_output_capture();
        while let Some(event) = rx.recv().await {
            let capture_status = remote_background_output_status(event.status);
            if let Some(metadata) = output_capture
                .update_lifecycle(&capture_id, event.session_id, capture_status, event.exit_code)
                .await
            {
                let timestamp = now_unix_seconds();
                let _ = event_system
                    .emit(BackgroundCommandLifecycle(BackgroundCommandLifecycleInfo {
                        agent_session_id: metadata.agent_session_id.or(agent_session_id.clone()),
                        exec_session_id: event.session_id,
                        command: metadata.command,
                        workdir: metadata.workdir,
                        remote: true,
                        tty: metadata.tty,
                        status: remote_lifecycle_status(event.status).to_string(),
                        exit_code: event.exit_code,
                        started_at: metadata.started_at,
                        ended_at: metadata.ended_at,
                        timestamp,
                    }))
                    .await;
            }
        }
    });
    Some(tx)
}

impl ExecCommandTool {
    /// Execute the tool against a remote workspace. Returns one
    /// [`ToolResult::Result`] whose `data` mirrors the original
    /// `command.rs` implementation and whose `result_for_assistant` is the
    /// model-facing string.
    pub(super) async fn call_remote_pipe(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let cmd = input
            .get("cmd")
            .and_then(Value::as_str)
            .ok_or_else(|| NortHingError::tool("cmd is required for ExecCommand".to_string()))?;
        let tty = input.get("tty").and_then(Value::as_bool).unwrap_or(false);

        let workdir = resolve_remote_workdir(input, context).await?;
        let connection_id = context
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.connection_id())
            .ok_or_else(|| NortHingError::tool("remote connection id is required for ExecCommand".to_string()))?
            .to_string();
        let ssh_manager = remote_workspace_manager()
            .ok_or_else(|| {
                NortHingError::tool("remote workspace manager is not initialized for ExecCommand".to_string())
            })?
            .get_ssh_manager()
            .await
            .ok_or_else(|| NortHingError::tool("remote SSH manager is not initialized for ExecCommand".to_string()))?;
        let yield_time_ms = input
            .get("yield_time_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TOOL_YIELD_TIME_MS);
        let shell = resolve_remote_shell(&ssh_manager, &connection_id).await;
        let env_snapshot =
            remote_env_snapshot_for(ssh_manager.clone(), &connection_id, &shell.path, &shell.shell_type).await;
        let command_body = if tty {
            cmd.to_string()
        } else {
            remote_non_tty_control_wrapper(cmd, &shell.path)
        };
        let command = remote_login_shell_command(&workdir, &command_body, &shell, env_snapshot.as_ref());
        let output_capture_tx = if let Some(capture_id) = context.tool_call_id.as_ref() {
            Some(
                background_command_output_capture()
                    .start_capture(StartBackgroundCommandOutputCapture {
                        capture_id: capture_id.clone(),
                        agent_session_id: context.session_id.clone(),
                        command: cmd.to_string(),
                        workdir: Some(workdir.clone()),
                        remote: true,
                        tty,
                    })
                    .await,
            )
        } else {
            None
        };

        let request = RemoteExecCommandRequest {
            ssh_manager,
            connection_id,
            command,
            tty,
            yield_time_ms: Some(yield_time_ms),
            max_output_chars: None,
            lifecycle_tx: start_remote_lifecycle_bridge(context, self.name()),
            output_capture_tx,
        };
        let progress_bridge = ExecOutputProgressBridge::start(context, self.name());
        let response_result = if let Some(bridge) = progress_bridge.as_ref() {
            global_remote_exec_process_manager()
                .exec_command_streaming(request, bridge.sender())
                .await
        } else {
            global_remote_exec_process_manager().exec_command(request).await
        };
        if let Some(bridge) = progress_bridge {
            bridge.finish().await;
        }
        let response = match response_result {
            Ok(response) => response,
            Err(error) => {
                if let Some(capture_id) = context.tool_call_id.as_ref() {
                    background_command_output_capture()
                        .finish(capture_id, BackgroundCommandOutputStatus::Failed, None)
                        .await;
                }
                return Err(NortHingError::tool(format!("ExecCommand failed: {error}")));
            }
        };
        if let Some(capture_id) = context.tool_call_id.as_ref() {
            if let Some(session_id) = response.session_id {
                background_command_output_capture()
                    .set_session_id(capture_id, Some(session_id))
                    .await;
            }
            if response.session_id.is_none() {
                background_command_output_capture()
                    .finish(
                        capture_id,
                        remote_background_output_status_for_completion(response.completion),
                        response.exit_code,
                    )
                    .await;
            }
        }

        let data = json!({
            "chunk_id": response.chunk_id,
            "wall_time_seconds": response.wall_time_seconds,
            "output": response.output,
            "session_id": response.session_id,
            "exit_code": response.exit_code,
            "original_output_chars": response.original_output_chars,
            "completion": response.completion.map(remote_completion_value),
            "workdir": workdir.clone(),
            "tty": tty,
            "remote": true,
            "shell": remote_shell_metadata(&workdir, &shell, env_snapshot.is_some()),
        });
        let result_for_assistant = super::response::response_for_assistant(&data);

        Ok(vec![ToolResult::Result {
            data,
            result_for_assistant: Some(result_for_assistant),
            image_attachments: None,
        }])
    }
}
