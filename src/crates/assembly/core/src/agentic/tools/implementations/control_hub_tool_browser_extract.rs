//! ControlHubTool browser sub-domain: DOM/screenshot/data extraction (snapshot, screenshot, evaluate, wait, get, get_html, auto_scroll, fetch, cookies, set_cookies, set_file_input_files, read_article).
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

use super::computer_use_actions::truncate_with_marker;
use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_browser::browser_sessions;
use super::ControlHubTool;

impl ControlHubTool {
    pub(super) async fn handle_browser_extract(
        &self,
        action: &str,
        params: &Value,
        session_id_param: Option<String>,
    ) -> NortHingResult<Vec<ToolResult>> {
        let session = browser_sessions().get(session_id_param.as_deref()).await?;
        let actions = BrowserActions::new(session.client.as_ref());
        match action {
            "snapshot" => {
                let with_backend = params
                    .get("with_backend_node_ids")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let result = actions.snapshot_with_options(with_backend).await?;
                let el_count = result
                    .get("elements")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Snapshot: {} interactive elements", el_count)),
                )])
            }

            "wait" => {
                let ms = params.get("duration_ms").and_then(|v| v.as_u64());
                let cond = params.get("condition").and_then(|v| v.as_str());
                let result = actions.wait(ms, cond).await?;
                Ok(vec![ToolResult::ok(result, Some("Wait completed".to_string()))])
            }

            "screenshot" => {
                let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("jpeg");
                let quality = params.get("quality").and_then(|v| v.as_u64()).map(|q| q as u8);
                let from_surface = params.get("from_surface").and_then(|v| v.as_bool()).unwrap_or(true);
                let full_page = params
                    .get("full_page")
                    .or_else(|| params.get("fullPage"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let result = actions
                    .screenshot_with_options_ext(format, quality, from_surface, full_page)
                    .await?;
                let data_len = result.get("data_length").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Screenshot captured ({} bytes base64)", data_len)),
                )])
            }

            "evaluate" => {
                let expression = params
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("evaluate requires 'expression'".to_string()))?;
                let await_promise = params
                    .get("await_promise")
                    .or_else(|| params.get("awaitPromise"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let return_by_value = params
                    .get("return_by_value")
                    .or_else(|| params.get("returnByValue"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                // Bound the size of the returned value so a runaway
                // `JSON.stringify(document)` can't blow up the model
                // context window. Default 16 KiB; clamp to [1 KiB, 256 KiB].
                let max_value_bytes = params
                    .get("max_value_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(16 * 1024)
                    .clamp(1024, 256 * 1024) as usize;
                let mut result = actions
                    .evaluate_with_options(expression, await_promise, return_by_value)
                    .await?;
                let mut truncated = false;
                if let Some(value) = result.pointer_mut("/result/value") {
                    let serialized = value.to_string();
                    if serialized.len() > max_value_bytes {
                        let (clip, was) = truncate_with_marker(&serialized, max_value_bytes);
                        truncated = was;
                        *value = json!(clip);
                    }
                }
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("truncated".to_string(), json!(truncated));
                }
                let display = result
                    .get("result")
                    .and_then(|r| r.get("value"))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| result.to_string());
                Ok(vec![ToolResult::ok(result, Some(display))])
            }

            "get" => {
                let selector = params
                    .get("selector")
                    .or_else(|| params.get("ref"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("get requires 'selector'".to_string()))?;
                let attribute = params.get("attribute").and_then(|v| v.as_str()).unwrap_or("text");
                match actions.get_attribute(selector, attribute).await? {
                    Some(value) => {
                        let display = value.to_string();
                        Ok(vec![ToolResult::ok(
                            json!({ "value": value, "found": true, "selector": selector, "attribute": attribute }),
                            Some(display),
                        )])
                    }
                    None => Ok(err_response(
                        "browser",
                        "get",
                        ControlHubError::new(
                            ErrorCode::NotFound,
                            format!("No element matched selector '{}'", selector),
                        )
                        .with_hint("Take a fresh snapshot and verify the @ref / CSS selector"),
                    )),
                }
            }

            "get_html" | "content" => {
                let selector = params.get("selector").and_then(|v| v.as_str());
                let result = if let Some(sel) = selector {
                    actions.get_attribute(sel, "html").await?
                } else {
                    actions.get_attribute("html", "html").await? // will fallback to document
                };
                match result {
                    Some(value) => {
                        let html = value.as_str().unwrap_or("").to_string();
                        Ok(vec![ToolResult::ok(
                            json!({ "html": html, "found": true }),
                            Some(format!("HTML: {} chars", html.len())),
                        )])
                    }
                    None => {
                        // Fallback: evaluate document.documentElement.outerHTML
                        let result = actions.evaluate("document.documentElement.outerHTML").await?;
                        let html = result
                            .get("result")
                            .and_then(|r| r.get("value"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        Ok(vec![ToolResult::ok(
                            json!({ "html": html, "found": true }),
                            Some(format!("HTML: {} chars", html.len())),
                        )])
                    }
                }
            }

            "auto_scroll" => {
                let direction = params.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                let max_scrolls = params
                    .get("max_scrolls")
                    .or_else(|| params.get("maxScrolls"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20);
                let delay_ms = params
                    .get("delay_ms")
                    .or_else(|| params.get("delayMs"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(800);
                let result = actions.auto_scroll(direction, max_scrolls, delay_ms).await?;
                Ok(vec![ToolResult::ok(
                    result,
                    Some(format!("Auto-scrolled {}", direction)),
                )])
            }

            "fetch" => {
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NortHingError::tool("fetch requires 'url'".to_string()))?;
                let method = params.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                let headers = params.get("headers").cloned().unwrap_or(json!({}));
                let body = params.get("body").and_then(|v| v.as_str());
                let result = actions.fetch(url, method, headers, body).await?;
                Ok(vec![ToolResult::ok(result, Some(format!("Fetched {}", url)))])
            }

            "cookies" | "get_cookies" => {
                let urls = params
                    .get("urls")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect());
                let result = actions.get_cookies(urls).await?;
                let cookies = result
                    .get("cookies")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                Ok(vec![ToolResult::ok(result, Some(format!("{} cookie(s)", cookies)))])
            }

            "set_cookies" => {
                let cookies = params
                    .get("cookies")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| NortHingError::tool("set_cookies requires 'cookies' array".to_string()))?;
                let result = actions.set_cookies(cookies).await?;
                let set = result.get("set").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(vec![ToolResult::ok(result, Some(format!("{} cookie(s) set", set)))])
            }

            "set_file_input_files" | "file_upload" => {
                let selector = params.get("selector").and_then(|v| v.as_str());
                let files: Vec<String> = params
                    .get("files")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| NortHingError::tool("set_file_input_files requires 'files' array".to_string()))?
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                let result = actions.set_file_input_files(selector, &files).await?;
                Ok(vec![ToolResult::ok(result, Some("Files set on input".to_string()))])
            }

            "read_article" => {
                if let Some(url) = params.get("url").and_then(|v| v.as_str()) {
                    actions.navigate(url).await?;
                }
                let result = actions.read_article().await?;
                let article = result.get("article").cloned().unwrap_or(Value::Null);
                let excerpt = article.get("excerpt").and_then(|v| v.as_str()).unwrap_or("");
                Ok(vec![ToolResult::ok(result, Some(format!("Article: {}", excerpt)))])
            }
            other => Err(NortHingError::tool(format!(
                "action '{}' dispatched to handle_browser_extract but is not in its match arms (facade dispatch bug)",
                other
            ))),
        }
    }
}
