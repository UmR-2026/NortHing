//! Computer Use system actions facade.
//!
//! Delegates to sibling modules by action name.

use crate::agentic::tools::framework::{ToolResult, ToolUseContext};
use crate::agentic::tools::implementations::computer_use_actions::ComputerUseActions;
use crate::agentic::tools::implementations::control_hub::{err_response, ControlHubError, ErrorCode};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;

mod app_control;
mod clipboard;
mod system_info;

pub(crate) use app_control::*;
pub(crate) use clipboard::*;
pub(crate) use system_info::*;

impl ComputerUseActions {
    pub(crate) async fn handle_system(
        &self,
        action: &str,
        params: &Value,
        context: &ToolUseContext,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "open_app" => self.handle_open_app(params, context).await,
            "run_script" => self.handle_run_script(params, context).await,
            "get_os_info" => self.handle_get_os_info(params, context).await,
            "clipboard_get" => self.handle_clipboard_get(params, context).await,
            "clipboard_set" => self.handle_clipboard_set(params, context).await,
            "open_url" => self.handle_open_url(params, context).await,
            "open_file" => self.handle_open_file(params, context).await,
            other => Err(NortHingError::tool(format!(
                "Unknown system action: '{}'. Valid: open_app, run_script, get_os_info, open_url, open_file, clipboard_get, clipboard_set",
                other
            ))),
        }
    }
}
