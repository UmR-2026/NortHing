//! ControlHubTool browser sub-domain: page registry query actions
//! (list_pages, tab_query).
//!
//!
//! R18 split: extracted from `control_hub_tool_browser_pages.rs` (the
//! 255-line post-R18 file) into a read-only-query sibling. Read-only
//! page registry operations: list pages, filter by URL/title substring.
//! The thin facade dispatcher in browser_pages.rs routes these
//! actions to this handler.

use crate::agentic::tools::browser_control::cdp_client::CdpClient;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::NortHingResult;
use serde_json::{json, Value};

use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_pages_query(
        &self,
        action: &str,
        params: &Value,
        port: u16,
    ) -> NortHingResult<Vec<ToolResult>> {
        match action {
            "list_pages" => {
                let pages = CdpClient::list_pages(port).await?;
                let default_id = browser_sessions().default_id().await;
                let summary: Vec<Value> = pages
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "title": p.title,
                            "url": p.url,
                            "type": p.page_type,
                            "is_default_session": Some(&p.id) == default_id.as_ref(),
                        })
                    })
                    .collect();
                Ok(vec![ToolResult::ok(
                    json!({
                        "pages": summary,
                        "default_session_id": default_id,
                    }),
                    Some(format!("{} page(s) found", pages.len())),
                )])
            }

            // Phase 2: filter pages by url substring / title substring without
            // forcing the model to ingest the entire `list_pages` payload.
            // This is essential when the user has dozens of tabs open and we
            // don't want to dump 50 KB of CDP page records into context.
            "tab_query" => {
                let url_contains = params
                    .get("url_contains")
                    .and_then(|v| v.as_str())
                    .map(str::to_lowercase);
                let title_contains = params
                    .get("title_contains")
                    .and_then(|v| v.as_str())
                    .map(str::to_lowercase);
                let only_pages = params.get("only_pages").and_then(|v| v.as_bool()).unwrap_or(true);
                let limit = params
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(20)
                    .max(1);

                let pages = CdpClient::list_pages(port).await?;
                let default_id = browser_sessions().default_id().await;
                let total = pages.len();
                let filtered: Vec<Value> = pages
                    .into_iter()
                    .filter(|p| {
                        if only_pages && p.page_type.as_deref() != Some("page") {
                            return false;
                        }
                        if let Some(ref needle) = url_contains {
                            if !p.url.to_lowercase().contains(needle) {
                                return false;
                            }
                        }
                        if let Some(ref needle) = title_contains {
                            if !p.title.to_lowercase().contains(needle) {
                                return false;
                            }
                        }
                        true
                    })
                    .take(limit)
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "title": p.title,
                            "url": p.url,
                            "type": p.page_type,
                            "is_default_session": Some(&p.id) == default_id.as_ref(),
                        })
                    })
                    .collect();
                let matched = filtered.len();
                Ok(vec![ToolResult::ok(
                    json!({
                        "pages": filtered,
                        "matched": matched,
                        "total": total,
                        "default_session_id": default_id,
                    }),
                    Some(format!("{} of {} page(s) matched", matched, total)),
                )])
            }
            other => Err(crate::util::errors::NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_pages but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
