//! Desktop automation (Computer use).
//!
//! This module is the public face of the ComputerUse tool. The actual
//! implementation is split across `metadata`, `validation`, `target_resolver`,
//! `screenshot`, and `actions` siblings — each one owns a coherent slice of
//! state or behavior so this facade stays small and the trait `Tool`
//! implementation can be read end-to-end.
//!
//! Cross-sibling wiring:
//! - Inherent methods on `ComputerUseTool` are resolved by Rust across all
//!   sibling files automatically (no `use` needed).
//! - Cross-sibling free functions and types are pulled in via
//!   `super::metadata::*` / `super::validation::*` style imports in the
//!   sibling that needs them.
//! - Free functions consumed by external modules (`computer_use_mouse_*_tool.rs`)
//!   are re-exported here as `pub(crate)`.

use super::computer_use_locate::execute_computer_use_locate;
use crate::agentic::tools::computer_use_capability::computer_use_desktop_available;
use crate::agentic::tools::framework::{Tool, ToolExposure, ToolResult, ToolUseContext};
use crate::service::config::global::GlobalConfigManager;
use crate::util::errors::{NortHingError, NortHingResult};
use async_trait::async_trait;
use serde_json::Value;

mod actions;
mod metadata;
mod screenshot;
mod target_resolver;
mod validation;

// Re-export the legacy free-function entrypoints consumed by the dedicated
// `computer_use_mouse_*_tool.rs` modules so the call site path stays stable.
pub(crate) use actions::{
    computer_use_execute_mouse_click_tool, computer_use_execute_mouse_precise, computer_use_execute_mouse_step,
};
// Re-export the shared result-augment helper so other top-level tool files
// (e.g. `computer_use_locate.rs`) can keep using the historical path.
pub(crate) use metadata::computer_use_augment_result_json;

pub struct ComputerUseTool;

impl Default for ComputerUseTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputerUseTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        "ComputerUse"
    }

    async fn description(&self) -> NortHingResult<String> {
        Self::description_impl().await
    }

    fn short_description(&self) -> String {
        Self::short_description_impl()
    }

    fn default_exposure(&self) -> ToolExposure {
        ToolExposure::Collapsed
    }

    async fn description_with_context(&self, context: Option<&ToolUseContext>) -> NortHingResult<String> {
        let vision = context
            .map(|c| c.primary_model_supports_image_understanding())
            .unwrap_or(true);
        if vision {
            self.description().await
        } else {
            Ok(Self::description_text_only_impl())
        }
    }

    fn input_schema(&self) -> Value {
        Self::input_schema_impl()
    }

    async fn input_schema_for_model(&self) -> Value {
        Self::input_schema_impl()
    }

    async fn input_schema_for_model_with_context(&self, context: Option<&ToolUseContext>) -> Value {
        let vision = context
            .map(|c| c.primary_model_supports_image_understanding())
            .unwrap_or(true);
        if vision {
            self.input_schema_for_model().await
        } else {
            Self::input_schema_text_only_impl()
        }
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn needs_permissions(&self, _input: Option<&Value>) -> bool {
        true
    }

    async fn is_enabled(&self) -> bool {
        if !computer_use_desktop_available() {
            return false;
        }
        let Ok(service) = GlobalConfigManager::service().await else {
            return false;
        };
        let ai: crate::service::config::types::AIConfig = service.config(Some("ai")).await.unwrap_or_default();
        ai.computer_use_enabled
    }

    async fn is_available_in_context(&self, context: Option<&ToolUseContext>) -> bool {
        if context.map(|ctx| ctx.is_remote()).unwrap_or(false) {
            return false;
        }
        self.is_enabled().await
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        if context.is_remote() {
            return Err(NortHingError::tool(
                "ComputerUse cannot run while the session workspace is remote (SSH).".to_string(),
            ));
        }

        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NortHingError::tool("action is required".to_string()))?;

        // System actions delegate to ComputerUseActions (R37h sibling).
        match action {
            "open_url" | "open_file" | "clipboard_get" | "clipboard_set" | "run_script" | "get_os_info" => {
                return super::computer_use_actions::ComputerUseActions::new()
                    .handle_system(action, input, context)
                    .await;
            }
            _ => {}
        }

        // Migrated desktop actions delegate to ComputerUseActions too.
        if Self::is_controlhub_migrated_desktop_action(action) {
            return super::computer_use_actions::ComputerUseActions::new()
                .handle_desktop(action, input, context)
                .await;
        }

        let host = context.computer_use_host.as_ref().ok_or_else(|| {
            NortHingError::tool("Computer use is only available in the northhing desktop app.".to_string())
        })?;

        let host_ref = host.as_ref();

        match action {
            "locate" => execute_computer_use_locate(input, context).await,
            "click_target" | "move_to_target" => Self::target_action_impl(host_ref, action, input, context).await,
            "click_element" => Self::click_element_impl(host_ref, input, context).await,
            "move_to_text" => Self::move_to_text_impl(host_ref, input, context).await,
            "click" => Self::click_impl(host_ref, input, context).await,
            "mouse_move" => Self::mouse_move_impl(host_ref, input, context).await,
            "scroll" => Self::scroll_impl(host_ref, input, context).await,
            "drag" => Self::drag_impl(host_ref, input, context).await,
            "screenshot" => Self::screenshot_action_impl(host_ref, input, context).await,
            "pointer_move_rel" => Self::pointer_move_rel_impl(host_ref, input, context).await,
            "key_chord" => Self::key_chord_impl(host_ref, input, context).await,
            "type_text" => Self::type_text_impl(host_ref, input, context).await,
            "wait" => Self::wait_impl(host_ref, input, context).await,
            "open_app" => Self::open_app_impl(host_ref, input, context).await,
            "run_apple_script" => Self::run_apple_script_impl(host_ref, input, context).await,
            _ => Err(NortHingError::tool(format!("Unknown action: {}", action))),
        }
    }
}
