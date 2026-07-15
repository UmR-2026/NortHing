//! ControlHubTool browser sub-domain: page registry lifecycle actions
//! (tab_new, switch_page).
//!
//!
//! R18 split: extracted from `control_hub_tool_browser_pages.rs` (the
//! 255-line post-R18 file) into a page-lifecycle sibling. Mutating
//! page registry operations: create new tab, switch to existing tab
//! (with optional Page.bringToFront). The thin facade dispatcher in
//! browser_pages.rs routes these actions to this handler.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::browser_control::cdp_client::CdpClient;
use crate::agentic::tools::browser_control::session_registry::{BrowserSession, BrowserSessionState};
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};
use std::sync::Arc;

use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_pages_lifecycle(
        &self,
        action: &str,
        params: &Value,
        port: u16,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "tab_new" => {
                let url = params.get("url").and_then(|v| v.as_str());
                let activate = params.get("activate").and_then(|v| v.as_bool()).unwrap_or(true);
                let page = CdpClient::create_page(port, url).await?;
                let ws_url = page
                    .web_socket_debugger_url
                    .as_ref()
                    .ok_or_else(|| NortHingError::tool("New tab has no WebSocket URL".to_string()))?;
                let client = CdpClient::connect(ws_url).await?;
                let session = BrowserSession {
                    session_id: page.id.clone(),
                    port,
                    client: Arc::new(client),
                    state: Arc::new(BrowserSessionState::new()),
                };
                browser_sessions().register(session.clone()).await;
                let _ = BrowserActions::new(session.client.as_ref()).enable_observers().await;
                if activate {
                    let _ = session.client.send("Page.bringToFront", None).await;
                }
                Ok(vec![ToolResult::ok(
                    json!({
                        "success": true,
                        "session_id": session.session_id,
                        "page_url": page.url,
                        "page_title": page.title,
                        "activated": activate,
                    }),
                    Some(format!(
                        "New tab opened: {} (session {})",
                        page.title, session.session_id
                    )),
                )])
            }

            "switch_page" => {
                let page_id = params
                    .get("page_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("switch_page requires 'page_id'".to_string()))?;
                // Phase 2: by default ALSO surface the chosen tab in the
                // user's actual browser window via `Page.bringToFront`. The
                // legacy behavior only swapped the CDP session under the
                // hood, leaving the user staring at the old tab while the
                // model "drove" an invisible one. Models can opt out by
                // passing `activate: false` for headless background tabs.
                let activate = params.get("activate").and_then(|v| v.as_bool()).unwrap_or(true);

                let registry = browser_sessions();
                let mut reused = false;
                let session = if registry.set_default(page_id).await.is_ok() {
                    reused = true;
                    registry.get(Some(page_id)).await?
                } else {
                    let pages = CdpClient::list_pages(port).await?;
                    let page = pages
                        .iter()
                        .find(|p| p.id == page_id)
                        .ok_or_else(|| NortHingError::tool(format!("Page '{}' not found", page_id)))?;
                    let ws_url = page
                        .web_socket_debugger_url
                        .as_ref()
                        .ok_or_else(|| NortHingError::tool("Page has no WebSocket URL".to_string()))?;
                    let client = CdpClient::connect(ws_url).await?;
                    let session = BrowserSession {
                        session_id: page.id.clone(),
                        port,
                        client: Arc::new(client),
                        state: Arc::new(BrowserSessionState::new()),
                    };
                    registry.register(session.clone()).await;
                    let _ = BrowserActions::new(session.client.as_ref()).enable_observers().await;
                    session
                };

                let mut activated = false;
                let mut activate_warning: Option<String> = None;
                if activate {
                    match session.client.send("Page.bringToFront", None).await {
                        Ok(_) => activated = true,
                        Err(e) => {
                            // Don't fail the whole switch — the session is
                            // still valid, the user just won't see the new
                            // tab front-and-center yet.
                            activate_warning = Some(format!(
                                "Page.bringToFront failed: {} (session is switched, but the tab is not in the foreground)",
                                e
                            ));
                        }
                    }
                }

                let mut body = json!({
                    "success": true,
                    "page_id": page_id,
                    "session_id": session.session_id,
                    "reused": reused,
                    "activated": activated,
                });
                if let Some(w) = &activate_warning {
                    body["warning"] = json!(w);
                }
                Ok(vec![ToolResult::ok(
                    body,
                    Some(format!(
                        "Switched to page {} ({})",
                        page_id,
                        if activated { "brought to front" } else { "background" }
                    )),
                )])
            }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_pages but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
