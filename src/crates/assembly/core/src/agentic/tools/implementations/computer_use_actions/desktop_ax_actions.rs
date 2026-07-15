//! Computer Use desktop AX-first action dispatch.
//!
//! `handle_desktop_ax` is the single entry point for the seven app-targeted
//! Computer Use actions. It dispatches to per-action siblings:
//!
//! - [`super::ax_query`] — `list_apps`, `get_app_state`, `build_interactive_view`,
//!   `build_visual_mark_view` (pure read / view-build)
//! - [`super::ax_click`] — `app_click`, `interactive_click`, `visual_click`
//!   (click targets with loop-warning + post-snapshot envelope)
//! - [`super::ax_input`] — `app_type_text`, `app_key_chord`, `app_scroll`,
//!   `app_wait_for`, `interactive_type_text`, `interactive_scroll`
//!
//! Cross-sibling shared helpers (parameter parsers, snapshot/view JSON
//! builders, multimodal image-result wrappers) live in [`super::ax_types`].

use crate::agentic::tools::computer_use_host::ComputerUseHostRef;
use crate::agentic::tools::framework::ToolResult;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;

use super::ComputerUseActions;

impl ComputerUseActions {
    pub(super) async fn handle_desktop_ax(
        &self,
        host: &ComputerUseHostRef,
        action: &str,
        params: &Value,
    ) -> NortHingResult<Vec<ToolResult>> {
        let bg = host.supports_background_input();
        let ax = host.supports_ax_tree();
        match action {
            "list_apps" => super::ax_query::list_apps(host, params, bg, ax).await,
            "get_app_state" => super::ax_query::get_app_state(host, params, bg, ax).await,
            "build_interactive_view" => super::ax_query::build_interactive_view(host, params).await,
            "build_visual_mark_view" => super::ax_query::build_visual_mark_view(host, params).await,
            "app_click" => super::ax_click::app_click(host, params, bg).await,
            "interactive_click" => super::ax_click::interactive_click(host, params).await,
            "visual_click" => super::ax_click::visual_click(host, params).await,
            "app_type_text" => super::ax_input::app_type_text(host, params, bg).await,
            "app_key_chord" => super::ax_input::app_key_chord(host, params, bg).await,
            "app_scroll" => super::ax_input::app_scroll(host, params, bg).await,
            "app_wait_for" => super::ax_input::app_wait_for(host, params, bg).await,
            "interactive_type_text" => super::ax_input::interactive_type_text(host, params).await,
            "interactive_scroll" => super::ax_input::interactive_scroll(host, params).await,
            other => Err(NortHingError::tool(format!(
                "[INTERNAL] handle_desktop_ax called with unknown action: {}",
                other
            ))),
        }
    }
}
