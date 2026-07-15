//! Computer Use desktop and OS/system action implementations.
//!
//! This module owns the action logic that used to live behind ControlHub's
//! desktop/system domains. ControlHub may still share the common error envelope
//! types, but it no longer owns these Computer Use behaviors.

use crate::agentic::tools::computer_use_host::ComputerUseForegroundApplication;
use crate::agentic::tools::framework::ToolUseContext;

use super::control_hub::{ControlHubError, ErrorCode};

mod ax_click;
mod ax_input;
mod ax_query;
mod ax_types;
mod desktop_actions;
mod desktop_ax_actions;
mod system_actions;
mod utilities;

// Wildcard re-exports preserve every existing `super::computer_use_actions::*`
// import path (control_hub_tool_browser, control_hub_tool_meta,
// control_hub_tool_tests, computer_use_tool all still resolve via the facade).
pub(crate) use desktop_actions::*;
pub(crate) use desktop_ax_actions::*;
pub(crate) use system_actions::*;
pub(crate) use utilities::*;
pub(crate) struct ComputerUseActions;

impl Default for ComputerUseActions {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputerUseActions {
    pub(crate) fn new() -> Self {
        Self
    }
    fn desktop_browser_guard_error(
        action: &str,
        foreground: Option<&ComputerUseForegroundApplication>,
    ) -> ControlHubError {
        let app_name = foreground
            .and_then(|app| app.name.as_deref())
            .unwrap_or("a web browser");
        ControlHubError::new(
            ErrorCode::GuardRejected,
            format!(
                "desktop.{} is blocked while {} is frontmost. Use ControlHub domain=\"browser\" for all browser interaction; desktop mouse/keyboard browser control is forbidden.",
                action, app_name
            ),
        )
        .with_hints([
            "Use browser.connect to attach via the test port, then drive the page with snapshot/click/fill/press_key",
            "For login/cookies/extensions, guide the user to start their default browser with the test port enabled before calling browser.connect",
            "For isolated project Web UI testing, use the headless browser flow instead of desktop automation",
        ])
    }
    fn is_probably_browser_app(foreground: &ComputerUseForegroundApplication) -> bool {
        let name = foreground.name.as_deref().unwrap_or("").to_ascii_lowercase();
        let bundle = foreground.bundle_id.as_deref().unwrap_or("").to_ascii_lowercase();

        const NAME_HINTS: &[&str] = &[
            "chrome",
            "chromium",
            "edge",
            "brave",
            "arc",
            "firefox",
            "safari",
            "browser",
            "浏览器",
        ];
        const BUNDLE_HINTS: &[&str] = &[
            "chrome", "chromium", "edge", "brave", "arc", "firefox", "safari", "browser",
        ];

        NAME_HINTS.iter().any(|hint| name.contains(hint)) || BUNDLE_HINTS.iter().any(|hint| bundle.contains(hint))
    }
    /// Returns `Some(ControlHubError)` when the requested `desktop.<action>`
    /// targets a foreground browser process and must be redirected to the
    /// browser domain instead of the desktop mouse/keyboard path. The list
    /// of guarded actions is intentionally narrow: it only covers actions
    /// that physically drive the cursor / keyboard. Read-only actions such
    /// as `screenshot` / `list_displays` are intentionally NOT in the list.
    pub(super) async fn desktop_action_targets_browser(
        &self,
        action: &str,
        context: &ToolUseContext,
    ) -> Option<ControlHubError> {
        let guarded_actions = [
            "click",
            "click_target",
            "click_element",
            "move_to_target",
            "mouse_move",
            "pointer_move_rel",
            "scroll",
            "drag",
            "key_chord",
            "type_text",
            "paste",
            "locate",
            "move_to_text",
        ];
        if !guarded_actions.contains(&action) {
            return None;
        }
        let host = context.computer_use_host.as_ref()?;
        let snapshot = host.computer_use_session_snapshot().await;
        let foreground = snapshot.foreground_application.as_ref()?;
        if Self::is_probably_browser_app(foreground) {
            return Some(Self::desktop_browser_guard_error(action, Some(foreground)));
        }
        None
    }
}
