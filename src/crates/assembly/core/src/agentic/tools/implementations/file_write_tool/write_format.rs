use crate::agentic::tools::framework::ToolResult;
use serde_json::json;
use tool_runtime::fs::WriteLocalFileMode;

pub fn write_success_result(
    logical_path: &str,
    mode: WriteLocalFileMode,
    bytes_written: usize,
    lines_written: usize,
    status: &str,
    assistant_message: String,
) -> ToolResult {
    ToolResult::Result {
        data: json!({
            "file_path": logical_path,
            "mode": mode_label(mode),
            "bytes_written": bytes_written,
            "lines_written": lines_written,
            "success": true,
            "status": status,
            "message": assistant_message,
        }),
        result_for_assistant: Some(assistant_message),
        image_attachments: None,
    }
}

fn mode_label(mode: WriteLocalFileMode) -> &'static str {
    match mode {
        WriteLocalFileMode::Write => "w",
        WriteLocalFileMode::Append => "a",
    }
}
