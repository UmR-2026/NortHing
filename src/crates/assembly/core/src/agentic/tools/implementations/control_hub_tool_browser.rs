//! ControlHubTool browser domain.

//!

//! R16 split: BROWSER_SESSIONS registry, browser connect-mode / hint

//! helpers, the CDP-method allowlist, and the full handle_browser

//! dispatcher extracted into this sibling. Everything used only by

//! handle_browser stays private; the entry point is `pub(super)` so

//! the facade's `dispatch()` can resolve it.

use crate::agentic::tools::browser_control::actions::BrowserActions;

use crate::agentic::tools::browser_control::browser_launcher::{
    BrowserKind, BrowserLauncher, LaunchResult, DEFAULT_CDP_PORT,
};

use crate::agentic::tools::framework::ToolResult;

use crate::agentic::tools::browser_control::cdp_client::CdpClient;

use crate::agentic::tools::browser_control::session_registry::{
    BrowserSession, BrowserSessionRegistry, BrowserSessionState, DialogHandler,
};

use crate::service::config::{get_global_config_service, GlobalConfig};

use crate::util::errors::{NortHingError, NortHingResult};

use serde_json::{json, Value};

use std::sync::{Arc, OnceLock};

use super::computer_use_actions::truncate_with_marker;

use super::control_hub::{err_response, ControlHubError, ErrorCode};
use super::ControlHubTool;

// parse_browser_kind — moved from control_hub_tool_helpers.rs in R18.
// Only used by browser siblings (was used by handle_browser_session's
// "connect" arm, which is now in control_hub_tool_browser_connect.rs).
pub(super) fn parse_browser_kind(browser: &str) -> crate::util::errors::NortHingResult<BrowserKind> {
    match BrowserLauncher::browser_kind_from_config(browser) {
        Some(kind) => Ok(kind),
        None => BrowserLauncher::detect_default_browser(),
    }
}

// Process-wide registry of CDP sessions (replaces the prior global

// Option<CdpClient> singleton that lost pages on every connect).

/// Process-wide registry of CDP sessions. Replaces the previous single
/// global `Option<CdpClient>` slot whose `*slot = Some(client)` semantics
/// silently dropped the prior page connection on every `connect` /
/// `switch_page`, breaking concurrent multi-tab work and racing
/// in-flight `wait` / lifecycle subscriptions.
static BROWSER_SESSIONS: std::sync::OnceLock<Arc<BrowserSessionRegistry>> = std::sync::OnceLock::new();

pub(super) fn browser_sessions() -> Arc<BrowserSessionRegistry> {
    BROWSER_SESSIONS
        .get_or_init(|| Arc::new(BrowserSessionRegistry::new()))
        .clone()
}

// connect-mode / hint helpers — used only by handle_browser.
impl ControlHubTool {
    pub(super) fn browser_connect_mode_from_params(params: &Value) -> &'static str {
        match params.get("mode").and_then(|v| v.as_str()) {
            Some("headless") => "headless",
            Some("default") => "default",
            _ => "default",
        }
    }

    pub(super) fn default_browser_connect_hints(kind: &BrowserKind, port: u16) -> Vec<String> {
        let exe = BrowserLauncher::browser_executable(kind);
        vec![
        "For login/cookies/extensions, use the user's default browser via CDP — never fall back to desktop mouse/keyboard automation.".to_string(),
        format!(
            "If CDP is not ready, restart the browser with the test port enabled: \"{}\" --remote-debugging-port={}",
            exe, port
        ),
        "After the browser is listening on the test port, use browser.connect / snapshot / click / fill to drive the DOM directly.".to_string(),
    ]
    }

    pub(super) fn headless_browser_connect_hints(port: u16) -> Vec<String> {
        vec![
        "For project Web UI testing that does not depend on user login state, use the dedicated headless browser flow instead of the user's browser.".to_string(),
        format!(
            "Start or attach a headless test browser on the test port {} and then drive it through browser DOM actions only.",
            port
        ),
        "Do not switch to desktop mouse/keyboard browser control in headless mode.".to_string(),
    ]
    }
}

// is_allowed_browser_cdp_method — used only by handle_browser.
impl ControlHubTool {
    pub(super) fn is_allowed_browser_cdp_method(method: &str) -> bool {
        matches!(
            method,
            "Accessibility.getFullAXTree"
                | "DOM.getDocument"
                | "DOM.getBoxModel"
                | "DOM.getContentQuads"
                | "DOM.querySelector"
                | "DOM.querySelectorAll"
                | "DOM.scrollIntoViewIfNeeded"
                | "DOM.setFileInputFiles"
                | "DOMSnapshot.captureSnapshot"
                | "Input.dispatchMouseEvent"
                | "Input.dispatchKeyEvent"
                | "Input.insertText"
                | "Network.getCookies"
                | "Network.getResponseBody"
                | "Network.setCookie"
                | "Page.getLayoutMetrics"
                | "Page.captureScreenshot"
                | "Runtime.enable"
                | "Emulation.setDeviceMetricsOverride"
                | "Emulation.clearDeviceMetricsOverride"
        )
    }
}
// handle_browser — thin dispatcher that maps action → sub-handler method on
// `ControlHubTool` defined in sibling files. Inherent-method dispatch resolves
// across `pub(super)` impl blocks in the sibling sub-domain files.

impl ControlHubTool {
    pub(super) async fn handle_browser(&self, action: &str, params: &Value) -> NortHingResult<Vec<ToolResult>> {
        let port = params
            .get("port")
            .and_then(|v| v.as_u64())
            .map(|p| p as u16)
            .unwrap_or(DEFAULT_CDP_PORT);

        let session_id_param = params.get("session_id").and_then(|v| v.as_str()).map(str::to_string);

        match action {
            "connect" | "list_pages" | "tab_query" | "tab_new" | "switch_page"
            | "list_sessions" | "close" => {
                self.handle_browser_session(action, params, session_id_param).await
            }
            "network" | "network_requests" | "console" | "errors" | "trace" => {
                self.handle_browser_telemetry(action, params, session_id_param).await
            }
            "navigate" | "back" | "forward" | "reload" | "refresh"
            | "get_url" | "get_title" | "get_text" => {
                self.handle_browser_navigation(action, params, session_id_param).await
            }
            "click" | "fill" | "type" | "select" | "press_key" | "scroll"
            | "hover" | "check" | "uncheck" => {
                self.handle_browser_interact(action, params, session_id_param).await
            }
            "snapshot" | "screenshot" | "evaluate" | "wait"
            | "get" | "get_html" | "content"
            | "auto_scroll" | "fetch" | "cookies" | "get_cookies"
            | "set_cookies" | "set_file_input_files" | "file_upload"
            | "read_article" => {
                self.handle_browser_extract(action, params, session_id_param).await
            }
            "cdp" | "dialog" | "frame" | "frame_main" => {
                self.handle_browser_advanced(action, params, session_id_param).await
            }
            other => Err(NortHingError::tool(format!(
                "Unknown browser action: '{}'. Valid: connect, tab_new, navigate, back, forward, reload, snapshot, click, hover, fill, type, check, uncheck, select, press_key, scroll, auto_scroll, wait, get, get_text, get_url, get_title, get_html, screenshot, evaluate, fetch, cookies, set_cookies, set_file_input_files, cdp, network, console, errors, trace, dialog, frame, frame_main, read_article, close, list_pages, tab_query, switch_page, list_sessions",
                other
            ))),
        }
    }
}
