//! ControlHubTool browser sub-domain: connect handler.
//!
//!
//! R18 split: extracted from `control_hub_tool_browser_session.rs` (the
//! 485-line post-R17 file) into a per-action sibling. This sibling owns
//! the `connect` action as a `pub(super)` inherent method on
//! `ControlHubTool`. The thin facade dispatcher in browser_session.rs
//! routes `"connect"` to this handler.

use crate::agentic::tools::browser_control::actions::BrowserActions;
use crate::agentic::tools::browser_control::browser_launcher::{
    BrowserKind, BrowserLauncher, LaunchResult, DEFAULT_CDP_PORT,
};
use crate::agentic::tools::browser_control::cdp_client::CdpClient;
use crate::agentic::tools::browser_control::session_registry::{BrowserSession, BrowserSessionState};
use crate::agentic::tools::framework::ToolResult;
use crate::service::config::{get_global_config_service, GlobalConfig};
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::{json, Value};
use std::sync::Arc;

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::control_hub_tool_browser::{browser_sessions, parse_browser_kind};
use super::ControlHubTool;
// default_browser_connect_hints / headless_browser_connect_hints are
// inherent methods on ControlHubTool (pub(super)) defined in
// control_hub_tool_browser.rs. Call as Self::fn_name(...) — they
// resolve across all `impl ControlHubTool` blocks in the same crate.

impl ControlHubTool {
    pub(super) async fn handle_browser_connect(
        &self,
        _action: &str,
        params: &Value,
        port: u16,
    ) -> NortHingResult<Vec<ToolResult>> {
        let mode = Self::browser_connect_mode_from_params(params);

        if mode == "headless" && !BrowserLauncher::is_cdp_available(port).await {
            return Ok(err_response(
                "browser",
                "connect",
                ControlHubError::new(
                    ErrorCode::NotAvailable,
                    format!(
                        "Headless browser test port {} is not available. Start the dedicated headless browser first, then connect via ControlHub browser actions.",
                        port
                    ),
                )
                .with_hints(Self::headless_browser_connect_hints(port)),
            ));
        }

        let kind = if let Some(browser_str) = params.get("browser").and_then(|v| v.as_str()) {
            parse_browser_kind(browser_str)
        } else if mode == "headless" {
            Ok(BrowserKind::Chrome)
        } else {
            let config = get_global_config_service().await?.config::<GlobalConfig>(None).await?;
            BrowserLauncher::resolve_browser_kind(Some(&config.ai.browser_control_preferred_browser))
        }?;

        let user_data_dir = params.get("user_data_dir").and_then(|v| v.as_str());
        let launch_result = if mode == "headless" {
            LaunchResult::AlreadyConnected
        } else {
            BrowserLauncher::launch_with_cdp_opts(&kind, port, user_data_dir).await?
        };

        // UX shortcut: a frequent flow is "drive my Gmail tab" /
        // "drive the GitHub PR I'm looking at". Without `target_*`
        // the model needed `connect` → `list_pages` → `switch_page`
        // (3 round-trips and one chance to pick the wrong id). With
        // `target_url` / `target_title` we collapse those into a
        // single `connect` call: pick the first page whose URL or
        // title contains the substring, register it as the default
        // session, and bring it to the front.
        let target_url = params.get("target_url").and_then(|v| v.as_str()).map(str::to_lowercase);
        let target_title = params
            .get("target_title")
            .and_then(|v| v.as_str())
            .map(str::to_lowercase);
        let activate = params.get("activate").and_then(|v| v.as_bool()).unwrap_or(true);

        match &launch_result {
            LaunchResult::AlreadyConnected | LaunchResult::Launched => {
                let pages = CdpClient::list_pages(port).await?;
                let connected_browser = if mode == "headless" {
                    "Headless test browser".to_string()
                } else {
                    kind.to_string()
                };

                // Selection: explicit target_* > first real page > first.
                let matched_by_target = if target_url.is_some() || target_title.is_some() {
                    pages.iter().find(|p| {
                        if p.web_socket_debugger_url.is_none() {
                            return false;
                        }
                        let url_ok = target_url
                            .as_ref()
                            .map(|n| p.url.to_lowercase().contains(n))
                            .unwrap_or(true);
                        let title_ok = target_title
                            .as_ref()
                            .map(|n| p.title.to_lowercase().contains(n))
                            .unwrap_or(true);
                        p.page_type.as_deref() == Some("page") && url_ok && title_ok
                    })
                } else {
                    None
                };

                // Tell the model when its filter found nothing instead
                // of silently falling back to the first tab and
                // confusing the next action.
                if (target_url.is_some() || target_title.is_some()) && matched_by_target.is_none() {
                    return Ok(err_response(
                        "browser",
                        "connect",
                        ControlHubError::new(
                            ErrorCode::WrongTab,
                            format!(
                                "No open tab matched target_url={:?} target_title={:?}",
                                target_url, target_title
                            ),
                        )
                        .with_hints([
                            "Call browser.list_pages or browser.tab_query first to inspect open tabs",
                            "Loosen the substring (e.g. domain only) and try again",
                        ]),
                    ));
                }

                let page = matched_by_target
                    .or_else(|| {
                        pages
                            .iter()
                            .find(|p| p.page_type.as_deref() == Some("page") && p.web_socket_debugger_url.is_some())
                    })
                    .or_else(|| pages.first())
                    .ok_or_else(|| NortHingError::tool("No browser pages found via CDP".to_string()))?;
                let ws_url = page
                    .web_socket_debugger_url
                    .as_ref()
                    .ok_or_else(|| NortHingError::tool("Page has no WebSocket debugger URL".to_string()))?;
                let client = CdpClient::connect(ws_url).await?;
                let version = CdpClient::get_version(port).await?;
                let session = BrowserSession {
                    session_id: page.id.clone(),
                    port,
                    client: Arc::new(client),
                    state: Arc::new(BrowserSessionState::new()),
                };
                browser_sessions().register(session.clone()).await;

                // Enable CDP observers so network/console/error events
                // start recording immediately for later query via
                // browser.network / browser.console / browser.errors.
                let _ = BrowserActions::new(session.client.as_ref()).enable_observers().await;

                // If the model targeted a specific tab AND wants it
                // foregrounded (default), bring it to front the same
                // way switch_page does. Failure here is non-fatal —
                // we still return the connected session.
                let mut activated = false;
                let mut activate_warning: Option<String> = None;
                let targeted = matched_by_target.is_some();
                if targeted && activate {
                    match session.client.send("Page.bringToFront", None).await {
                        Ok(_) => activated = true,
                        Err(e) => {
                            activate_warning = Some(format!(
                                "Page.bringToFront failed: {} (session is connected, but the tab is not in the foreground)",
                                e
                            ));
                        }
                    }
                }

                let mut result = json!({
                    "success": true,
                    "browser": connected_browser,
                    "browser_mode": mode,
                    "browser_version": version.browser,
                    "port": port,
                    "session_id": session.session_id,
                    "page_url": page.url,
                    "page_title": page.title,
                    "matched_by_target": targeted,
                    "activated": activated,
                    "status": if mode == "headless" {
                        "attached"
                    } else if matches!(launch_result, LaunchResult::AlreadyConnected) {
                        "already_connected"
                    } else {
                        "launched"
                    },
                });
                if let Some(w) = activate_warning {
                    result["warning"] = json!(w);
                }
                let summary = if targeted {
                    format!(
                        "Connected to {} via DOM/CDP (session {}, page '{}')",
                        connected_browser, session.session_id, page.title
                    )
                } else {
                    format!(
                        "Connected to {} on test port {} via DOM/CDP (session {})",
                        connected_browser, port, session.session_id
                    )
                };
                Ok(vec![ToolResult::ok(result, Some(summary))])
            }
            LaunchResult::LaunchedButCdpNotReady { message, .. } => Ok(err_response(
                "browser",
                "connect",
                ControlHubError::new(ErrorCode::Timeout, message.clone())
                    .with_hints(Self::default_browser_connect_hints(&kind, port)),
            )),
            LaunchResult::BrowserRunningWithoutCdp { instructions, .. } => Ok(err_response(
                "browser",
                "connect",
                ControlHubError::new(
                    ErrorCode::NotAvailable,
                    "The user's default browser is running without the test port enabled.",
                )
                .with_hint(instructions)
                .with_hints(Self::default_browser_connect_hints(&kind, port)),
            )),
        }
    }
}
