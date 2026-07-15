//! Shared model-facing response rendering for [`ExecCommandTool`].
//!
//! Both the local and remote call paths converge here to render the final
//! `result_for_assistant` string. The shape mirrors the original
//! `command.rs::response_for_assistant` so the model's view of the result
//! does not change.

use serde_json::Value;

use super::super::rendering::render_exec_response_for_assistant;

/// Build the assistant-facing status lines (interruption/exit code/etc.)
/// and feed them through the shared renderer. Status lines are derived from
/// the same `data` JSON the tool result embeds, so the rendering is
/// deterministic for any given input.
pub(super) fn response_for_assistant(data: &Value) -> String {
    let mut status_lines = Vec::new();
    let completion = data.get("completion");
    let completion_source = completion.and_then(|value| value.get("source")).and_then(Value::as_str);
    let completion_status = completion.and_then(|value| value.get("status")).and_then(Value::as_str);
    if completion_source == Some("out_of_band_control") {
        match completion_status {
            Some("interrupted") => status_lines.push("Process was interrupted externally.".to_string()),
            Some("killed") => status_lines.push("Process was terminated externally.".to_string()),
            Some(status) => status_lines.push(format!("Process ended externally with status {status}.")),
            None => status_lines.push("Process ended externally.".to_string()),
        }
        if let Some(exit_code) = data.get("exit_code").and_then(Value::as_i64) {
            status_lines.push(format!("Process exited with code {exit_code}."));
        }
    } else if let Some(exit_code) = data.get("exit_code").and_then(Value::as_i64) {
        status_lines.push(format!("Process exited with code {exit_code}."));
    } else if let Some(session_id) = data.get("session_id").and_then(Value::as_i64) {
        status_lines.push(format!("Process is still running. session_id: {session_id}"));
    }
    render_exec_response_for_assistant(data, status_lines, 3)
}
