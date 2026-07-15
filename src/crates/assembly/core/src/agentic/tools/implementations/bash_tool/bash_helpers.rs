use crate::agentic::tools::framework::ToolUseContext;
use std::path::Path;

pub(crate) fn background_output_file_reference(
    context: &ToolUseContext,
    chat_session_id: &str,
    tool_use_id: &str,
    output_file_path: &Path,
) -> String {
    context
        .build_session_runtime_artifact_reference(chat_session_id, &format!("tool-results/{}.txt", tool_use_id))
        .unwrap_or_else(|_| output_file_path.display().to_string())
}
