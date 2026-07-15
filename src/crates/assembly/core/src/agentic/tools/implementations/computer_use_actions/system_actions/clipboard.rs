//! Clipboard read/write handlers.

use crate::agentic::tools::framework::ToolResult;
use crate::agentic::tools::implementations::computer_use_actions::utilities::{
    clipboard_read, clipboard_write, linux_clipboard_install_hints, truncate_with_marker,
};
use crate::agentic::tools::implementations::computer_use_actions::ComputerUseActions;
use crate::agentic::tools::implementations::control_hub::{err_response, ControlHubError, ErrorCode};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::json;

impl ComputerUseActions {
    pub(crate) async fn handle_clipboard_get(
        &self,
        params: &serde_json::Value,
        _context: &crate::agentic::tools::framework::ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let max_bytes = params
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(64 * 1024)
            .clamp(64, 1024 * 1024);

        match clipboard_read().await {
            Ok(text) => {
                let (truncated, was_truncated) = truncate_with_marker(&text, max_bytes);
                let len = text.len();
                Ok(vec![ToolResult::ok(
                    json!({
                        "text": truncated,
                        "byte_length": len,
                        "truncated": was_truncated,
                    }),
                    Some(format!("{} bytes on clipboard", len)),
                )])
            }
            Err(e) => Ok(err_response(
                "system",
                "clipboard_get",
                ControlHubError::new(ErrorCode::NotAvailable, format!("Clipboard read failed: {}", e))
                    .with_hints(linux_clipboard_install_hints()),
            )),
        }
    }

    pub(crate) async fn handle_clipboard_set(
        &self,
        params: &serde_json::Value,
        _context: &crate::agentic::tools::framework::ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("clipboard_set requires 'text'".to_string()))?;
        match clipboard_write(text).await {
            Ok(()) => Ok(vec![ToolResult::ok(
                json!({
                    "success": true,
                    "byte_length": text.len(),
                }),
                Some(format!("Wrote {} bytes to clipboard", text.len())),
            )]),
            Err(e) => Ok(err_response(
                "system",
                "clipboard_set",
                ControlHubError::new(ErrorCode::NotAvailable, format!("Clipboard write failed: {}", e))
                    .with_hints(linux_clipboard_install_hints()),
            )),
        }
    }
}
