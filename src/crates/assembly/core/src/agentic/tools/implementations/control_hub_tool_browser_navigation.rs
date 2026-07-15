//! ControlHubTool browser sub-domain: navigation (navigate, back, forward, reload, get_url, get_title, get_text).
//!
//!
//! R17 split: extracted from `control_hub_tool_browser.rs` (the 1272-line
//! god file post-R16) into per-subdomain sibling files. The facade keeps
//! the BROWSER_SESSIONS registry + thin `handle_browser` dispatcher; this
//! sibling owns the actions listed below as `pub(super)` inherent methods
//! on `ControlHubTool`.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_navigation(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let session = browser_sessions().get(session_id_param.as_deref()).await?;
        let actions = BrowserActions::new(session.client.as_ref());
        match action {
                "navigate" => {
                    let url = params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            NortHingError::tool("navigate requires 'url'".to_string())
                        })?;
                    let result = actions.navigate(url).await?;
                    Ok(vec![ToolResult::ok(result, Some(format!("Navigated to {}", url)))])
                }

                "get_text" => {
                    let selector = params
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            NortHingError::tool("get_text requires 'selector'".to_string())
                        })?;
                    match actions.get_text(selector).await? {
                        Some(text) => Ok(vec![ToolResult::ok(
                            json!({ "text": text, "found": true }),
                            Some(text),
                        )]),
                        None => Ok(err_response(
                            "browser",
                            "get_text",
                            ControlHubError::new(
                                ErrorCode::NotFound,
                                format!("No element matched selector '{}'", selector),
                            )
                            .with_hint(
                                "Take a fresh snapshot and verify the @ref / CSS selector",
                            ),
                        )),
                    }
                }

                "get_url" => {
                    let url = actions.get_url().await?;
                    Ok(vec![ToolResult::ok(
                        json!({ "url": url }),
                        Some(url),
                    )])
                }

                "get_title" => {
                    let title = actions.get_title().await?;
                    Ok(vec![ToolResult::ok(
                        json!({ "title": title }),
                        Some(title),
                    )])
                }

                "back" => {
                    let result = actions.back().await?;
                    Ok(vec![ToolResult::ok(result, Some("Navigated back".to_string()))])
                }

                "forward" => {
                    let result = actions.forward().await?;
                    Ok(vec![ToolResult::ok(result, Some("Navigated forward".to_string()))])
                }

                "reload" | "refresh" => {
                    let ignore_cache = params
                        .get("ignore_cache")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let result = actions.reload(ignore_cache).await?;
                    Ok(vec![ToolResult::ok(result, Some("Page reloaded".to_string()))])
                }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_navigation but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
