//! Local (non-remote) command logic for [`ExecCommandTool`].
//!
//! Splits the original `impl ExecCommandTool` block into the pieces that only
//! run when the active workspace is local. The remote equivalents live in
//! `remote.rs`. The single entry point executed by the tool dispatcher is
//! [`ExecCommandTool::call_local_pipe`]; everything else is a helper that the
//! local call composes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use terminal_core::{
    global_exec_process_manager, ExecProcessLifecycleEvent, ExecProcessLifecycleStatus, LocalExecCommandRequest,
    LocalExecSessionCompletion, LocalExecSessionCompletionSource, LocalExecSessionCompletionStatus, ShellType,
};
use tokio::sync::mpsc;

use super::super::background_command_output::{
    background_command_output_capture, BackgroundCommandOutputStatus, StartBackgroundCommandOutputCapture,
};
use super::super::local_shell::{resolve_local_exec_shell, ResolvedLocalExecShell};
use super::super::progress::ExecOutputProgressBridge;
use super::shell_helpers::now_unix_seconds;
pub(super) use super::tool::ExecCommandTool;
use super::types::{DEFAULT_TOOL_YIELD_TIME_MS, POWERSHELL_UTF8_OUTPUT_PREFIX};
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::infrastructure::events::event_system::{global_event_system, BackendEvent::BackgroundCommandLifecycle};
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::event::BackgroundCommandLifecycleInfo;

/// Build the local command environment that suppresses pagers, colours, and
/// interactive git prompts. Returned as a `HashMap` because the underlying
/// `LocalExecCommandRequest` consumes the env by value.
pub(super) fn command_env() -> HashMap<String, String> {
    HashMap::from([
        ("NO_COLOR".to_string(), "1".to_string()),
        ("TERM".to_string(), "dumb".to_string()),
        ("LANG".to_string(), "C.UTF-8".to_string()),
        ("LC_CTYPE".to_string(), "C.UTF-8".to_string()),
        ("COLORTERM".to_string(), String::new()),
        ("CLICOLOR".to_string(), "0".to_string()),
        ("PAGER".to_string(), "cat".to_string()),
        ("GIT_PAGER".to_string(), "cat".to_string()),
        ("GH_PAGER".to_string(), "cat".to_string()),
        ("GIT_TERMINAL_PROMPT".to_string(), "0".to_string()),
        ("GIT_EDITOR".to_string(), "true".to_string()),
        ("northhing_NONINTERACTIVE".to_string(), "1".to_string()),
    ])
}

/// Resolve the local `workdir` parameter: explicit absolute path wins, else
/// the workspace root. Refuses to run on relative or non-existent paths.
pub(super) fn resolve_workdir(input: &Value, context: &ToolUseContext) -> NortHingResult<PathBuf> {
    let raw = input
        .get("workdir")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|workdir| !workdir.is_empty())
        .map(str::to_string)
        .or_else(|| {
            context
                .workspace
                .as_ref()
                .map(|workspace| workspace.session_identity.logical_workspace_path().to_string())
        })
        .ok_or_else(|| NortHingError::tool("workspace root is required for ExecCommand".to_string()))?;

    let path = PathBuf::from(&raw);
    if !path.is_absolute() {
        return Err(NortHingError::tool(
            "workdir must be an absolute path for ExecCommand".to_string(),
        ));
    }
    if !path.is_dir() {
        return Err(NortHingError::tool(format!(
            "workdir does not exist or is not a directory: {}",
            path.display()
        )));
    }
    Ok(path)
}

/// Build the argv vector that drives the local child process. Each
/// `ShellType` has its own flag conventions (`-lc` for POSIX shells,
/// `-Command` for PowerShell, `/c` for cmd.exe). PowerShell is additionally
/// wrapped with the UTF-8 output prefix via
/// [`powershell_command_with_utf8_output`].
pub(super) fn argv_for_shell(path: &Path, shell_type: &ShellType, cmd: &str) -> Vec<String> {
    let shell = path.to_string_lossy().to_string();
    match shell_type {
        ShellType::Bash
        | ShellType::Zsh
        | ShellType::Fish
        | ShellType::Sh
        | ShellType::Ksh
        | ShellType::Csh
        | ShellType::Custom(_) => vec![shell, "-lc".to_string(), cmd.to_string()],
        ShellType::PowerShell | ShellType::PowerShellCore => {
            vec![shell, "-Command".to_string(), powershell_command_with_utf8_output(cmd)]
        }
        ShellType::Cmd => vec![shell, "/c".to_string(), cmd.to_string()],
    }
}

/// Prepend the PowerShell UTF-8 output prefix unless the script already
/// starts with it, in which case it is returned unchanged. Avoids nesting
/// the prefix when callers compose scripts.
pub(super) fn powershell_command_with_utf8_output(cmd: &str) -> String {
    let trimmed = cmd.trim_start();
    if trimmed.starts_with(POWERSHELL_UTF8_OUTPUT_PREFIX) {
        cmd.to_string()
    } else {
        format!("{POWERSHELL_UTF8_OUTPUT_PREFIX}{cmd}")
    }
}

/// Format a model-facing invocation hint for the resolved local shell.
pub(super) fn shell_invocation_for_model(path: &Path, shell_type: &ShellType) -> String {
    let shell = path.to_string_lossy();
    match shell_type {
        ShellType::Bash
        | ShellType::Zsh
        | ShellType::Fish
        | ShellType::Sh
        | ShellType::Ksh
        | ShellType::Csh
        | ShellType::Custom(_) => format!("`{shell} -lc <cmd>`"),
        ShellType::PowerShell | ShellType::PowerShellCore => {
            format!("`{shell} -Command <cmd>`")
        }
        ShellType::Cmd => format!("`{shell} /c <cmd>`"),
    }
}

/// JSON metadata for the resolved local shell embedded in the tool result.
pub(super) fn shell_metadata_value(shell: &ResolvedLocalExecShell) -> Value {
    json!({
        "name": shell.display_name,
        "type": shell.shell_type.to_string(),
        "path": shell.path.to_string_lossy(),
        "invocation": shell_invocation_for_model(&shell.path, &shell.shell_type),
    })
}

/// Render a [`LocalExecSessionCompletion`] as the `{"status","source"}`
/// shape the rest of the tool result pipeline expects.
pub(super) fn local_completion_value(completion: LocalExecSessionCompletion) -> Value {
    json!({
        "status": match completion.status {
            LocalExecSessionCompletionStatus::Exited => "exited",
            LocalExecSessionCompletionStatus::Interrupted => "interrupted",
            LocalExecSessionCompletionStatus::Killed => "killed",
            LocalExecSessionCompletionStatus::Pruned => "pruned",
        },
        "source": match completion.source {
            LocalExecSessionCompletionSource::Process => "process",
            LocalExecSessionCompletionSource::OutOfBandControl => "out_of_band_control",
        },
    })
}

/// Map a terminal completion status to the [`BackgroundCommandOutputStatus`]
/// enum used by the background command output capture.
pub(super) fn local_background_output_status_for_completion(
    completion: Option<LocalExecSessionCompletion>,
) -> BackgroundCommandOutputStatus {
    match completion.map(|completion| completion.status) {
        Some(LocalExecSessionCompletionStatus::Interrupted) => BackgroundCommandOutputStatus::Interrupted,
        Some(LocalExecSessionCompletionStatus::Killed) => BackgroundCommandOutputStatus::Killed,
        Some(LocalExecSessionCompletionStatus::Pruned) => BackgroundCommandOutputStatus::Pruned,
        Some(LocalExecSessionCompletionStatus::Exited) | None => BackgroundCommandOutputStatus::Exited,
    }
}

/// String label for a local lifecycle status, used in the
/// `BackgroundCommandLifecycleInfo` event.
pub(super) fn local_lifecycle_status(status: ExecProcessLifecycleStatus) -> &'static str {
    match status {
        ExecProcessLifecycleStatus::Running => "running",
        ExecProcessLifecycleStatus::Exited => "exited",
        ExecProcessLifecycleStatus::Interrupted => "interrupted",
        ExecProcessLifecycleStatus::Killed => "killed",
        ExecProcessLifecycleStatus::Pruned => "pruned",
    }
}

/// Map a local mid-flight lifecycle status to the
/// [`BackgroundCommandOutputStatus`] the capture needs.
pub(super) fn local_background_output_status(status: ExecProcessLifecycleStatus) -> BackgroundCommandOutputStatus {
    match status {
        ExecProcessLifecycleStatus::Running => BackgroundCommandOutputStatus::Running,
        ExecProcessLifecycleStatus::Exited => BackgroundCommandOutputStatus::Exited,
        ExecProcessLifecycleStatus::Interrupted => BackgroundCommandOutputStatus::Interrupted,
        ExecProcessLifecycleStatus::Killed => BackgroundCommandOutputStatus::Killed,
        ExecProcessLifecycleStatus::Pruned => BackgroundCommandOutputStatus::Pruned,
    }
}

/// Spawn the task that translates local process lifecycle events into
/// `BackgroundCommandLifecycle` global events. Returns `None` when the
/// call has no `tool_call_id` to attach metadata to.
pub(super) fn start_local_lifecycle_bridge(
    context: &ToolUseContext,
    _tool_name: &str,
) -> Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>> {
    let capture_id = context.tool_call_id.clone()?;
    let agent_session_id = context.session_id.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<ExecProcessLifecycleEvent>();
    tokio::spawn(async move {
        let event_system = global_event_system();
        let output_capture = background_command_output_capture();
        while let Some(event) = rx.recv().await {
            let capture_status = local_background_output_status(event.status);
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
                        remote: false,
                        tty: metadata.tty,
                        status: local_lifecycle_status(event.status).to_string(),
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
    /// Execute the tool against a local workspace. Returns one
    /// [`ToolResult::Result`] whose `data` mirrors the original
    /// `command.rs` implementation and whose `result_for_assistant` is the
    /// model-facing string.
    pub(super) async fn call_local_pipe(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let cmd = input
            .get("cmd")
            .and_then(Value::as_str)
            .ok_or_else(|| NortHingError::tool("cmd is required for ExecCommand".to_string()))?;
        let workdir = resolve_workdir(input, context)?;
        let tty = input.get("tty").and_then(Value::as_bool).unwrap_or(false);
        let shell = resolve_local_exec_shell().await;
        let yield_time_ms = input
            .get("yield_time_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TOOL_YIELD_TIME_MS);
        let output_capture_tx = if let Some(capture_id) = context.tool_call_id.as_ref() {
            Some(
                background_command_output_capture()
                    .start_capture(StartBackgroundCommandOutputCapture {
                        capture_id: capture_id.clone(),
                        agent_session_id: context.session_id.clone(),
                        command: cmd.to_string(),
                        workdir: Some(workdir.to_string_lossy().to_string()),
                        remote: false,
                        tty,
                    })
                    .await,
            )
        } else {
            None
        };

        let request = LocalExecCommandRequest {
            argv: argv_for_shell(&shell.path, &shell.shell_type, cmd),
            cwd: workdir.clone(),
            env: command_env(),
            tty,
            yield_time_ms: Some(yield_time_ms),
            max_output_chars: None,
            lifecycle_tx: start_local_lifecycle_bridge(context, self.name()),
            output_capture_tx,
        };
        let progress_bridge = ExecOutputProgressBridge::start(context, self.name());
        let response_result = if let Some(bridge) = progress_bridge.as_ref() {
            global_exec_process_manager()
                .exec_command_streaming(request, bridge.sender())
                .await
        } else {
            global_exec_process_manager().exec_command(request).await
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
                        local_background_output_status_for_completion(response.completion),
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
            "completion": response.completion.map(local_completion_value),
            "workdir": workdir.to_string_lossy(),
            "tty": tty,
            "shell": shell_metadata_value(&shell),
        });
        let result_for_assistant = super::response::response_for_assistant(&data);

        Ok(vec![ToolResult::Result {
            data,
            result_for_assistant: Some(result_for_assistant),
            image_attachments: None,
        }])
    }
}
