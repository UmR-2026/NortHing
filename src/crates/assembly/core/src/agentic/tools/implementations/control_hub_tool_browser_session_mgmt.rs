//! ControlHubTool browser sub-domain: session registry actions
//! (list_sessions, close).
//!
//!
//! R18 split: extracted from `control_hub_tool_browser_session.rs` (the
//! 485-line post-R17 file) into a per-action sibling. Both actions act
//! on the **session registry** (not the page registry). The thin
//! facade dispatcher in browser_session.rs routes these actions to
//! this handler.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::NortHingResult;
use serde_json::json;

use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_session_mgmt(
        &self,
        action: &str,
        _params: &serde_json::Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "list_sessions" => {
                let registry = browser_sessions();
                let ids = registry.list().await;
                let default = registry.default_id().await;
                Ok(vec![ToolResult::ok(
                    json!({
                        "sessions": ids,
                        "default_session_id": default,
                    }),
                    Some(format!("{} session(s) tracked", ids.len())),
                )])
            }
            "close" => {
                let session = browser_sessions().get(session_id_param.as_deref()).await?;
                let actions = BrowserActions::new(session.client.as_ref());
                let result = actions.close_page().await?;
                // After a close, drop the session so subsequent calls
                // don't try to talk through a half-dead WebSocket.
                browser_sessions().remove(&session.session_id).await;
                Ok(vec![ToolResult::ok(result, Some("Page closed".to_string()))])
            }
            other => Err(crate::util::errors::NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_session but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
