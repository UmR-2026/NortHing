use crate::util::errors::NortHingResult;
use tool_runtime::shell::{
    render_local_shell_result, render_remote_shell_result, LocalShellResultRenderRequest,
    RemoteShellResultRenderRequest,
};

/// Render the assistant-facing result for a successful local execution.
pub(crate) fn format_local_result(
    terminal_session_id: &str,
    working_directory: &str,
    output_text: &str,
    was_interrupted: bool,
    timed_out: bool,
    exit_code: i32,
    shell_state: Option<&str>,
) -> NortHingResult<String> {
    Ok(render_local_shell_result(LocalShellResultRenderRequest {
        terminal_session_id,
        working_directory,
        output_text,
        interrupted: was_interrupted,
        timed_out,
        exit_code,
        shell_state,
    }))
}

/// Render the assistant-facing result for a successful remote execution.
pub(crate) fn format_remote_result(
    working_directory: &str,
    stdout: &str,
    stderr: &str,
    was_interrupted: bool,
    timed_out: bool,
    exit_code: i32,
) -> NortHingResult<String> {
    Ok(render_remote_shell_result(RemoteShellResultRenderRequest {
        working_directory,
        stdout,
        stderr,
        interrupted: was_interrupted,
        timed_out,
        exit_code,
    }))
}

/// Pass a serde_json Value through unchanged for tool metadata.
pub(crate) fn json_object_metadata(value: serde_json::Value) -> serde_json::Value {
    value
}
