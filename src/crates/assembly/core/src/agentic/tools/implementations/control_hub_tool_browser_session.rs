//! ControlHubTool browser sub-domain: session lifecycle thin facade.
//!
//!
//! R18 split: previously a 485-line file owning 7 action arms. Now a
//! thin dispatcher that routes each action to a per-action sibling
//! handler via inherent-method dispatch on `ControlHubTool`:
//!   `"connect"`           → `handle_browser_connect`         (control_hub_tool_browser_connect.rs)
//!   `list_pages` / `tab_query` / `tab_new` / `switch_page`
//!                          → `handle_browser_pages`           (control_hub_tool_browser_pages.rs)
//!   `list_sessions` / `close`
//!                          → `handle_browser_session_mgmt`    (control_hub_tool_browser_session_mgmt.rs)

use crate::agentic::tools::browser_control::browser_launcher::DEFAULT_CDP_PORT;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;

use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_session(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let port = params
            .get("port")
            .and_then(|v| v.as_u64())
            .map(|p| p as u16)
            .unwrap_or(DEFAULT_CDP_PORT);
        match action {
            "connect" => self.handle_browser_connect(action, params, port).await,
            "list_pages" | "tab_query" | "tab_new" | "switch_page" => {
                self.handle_browser_pages(action, params, port, session_id_param).await
            }
            "list_sessions" | "close" => self.handle_browser_session_mgmt(action, params, session_id_param).await,
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_session but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
