//! ControlHubTool browser sub-domain: page registry actions thin facade.
//!
//!
//! R18 split: 4 actions (list_pages, tab_query, tab_new, switch_page)
//! previously owned by `control_hub_tool_browser_session.rs`. Now a
//! thin dispatcher that routes each action to a per-action sibling
//! handler via inherent-method dispatch on `ControlHubTool`:
//!   `list_pages` / `tab_query` → `handle_browser_pages_query`
//!                               (control_hub_tool_browser_pages_query.rs)
//!   `tab_new` / `switch_page`  → `handle_browser_pages_lifecycle`
//!                               (control_hub_tool_browser_pages_lifecycle.rs)

use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;

use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_pages(
        &self,
        action: &str,
        params: &Value,
        port: u16,
        _session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "list_pages" | "tab_query" => self.handle_browser_pages_query(action, params, port).await,
            "tab_new" | "switch_page" => self.handle_browser_pages_lifecycle(action, params, port).await,
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_pages but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
