//! Code review result submission tool
//!
//! Used to get structured code review results.

use crate::agentic::coordination::global_coordinator;
use crate::agentic::deep_review::report as deep_review_report;
use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use crate::service::config::get_app_language_code;
use crate::service::i18n::code_review_copy_for_language;
use crate::util::errors::NortHingResult;
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;

mod analyze;
mod format;
mod prompt;
#[cfg(test)]
mod tests;

/// Code review tool definition
pub struct CodeReviewTool;

impl CodeReviewTool {
    pub fn new() -> Self {
        Self
    }

    pub fn name_str() -> &'static str {
        "submit_code_review"
    }
}

impl Default for CodeReviewTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CodeReviewTool {
    fn name(&self) -> &str {
        Self::name_str()
    }

    async fn description(&self) -> NortHingResult<String> {
        let lang = get_app_language_code().await;
        Ok(Self::description_for_language(lang.as_str()))
    }

    fn short_description(&self) -> String {
        "Submit a structured code review result.".to_string()
    }

    fn input_schema(&self) -> Value {
        Self::input_schema_value()
    }

    async fn input_schema_for_model(&self) -> Value {
        let lang = get_app_language_code().await;
        Self::input_schema_value_for_language(lang.as_str())
    }

    async fn input_schema_for_model_with_context(&self, context: Option<&ToolUseContext>) -> Value {
        let lang = get_app_language_code().await;
        Self::input_schema_value_for_language_with_mode(lang.as_str(), Self::is_deep_review_context(context))
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    async fn call_impl(&self, input: &Value, context: &ToolUseContext) -> NortHingResult<Vec<ToolResult>> {
        let mut filled_input = input.clone();
        let deep_review = Self::is_deep_review_context(Some(context));
        let compression_contract = deep_review
            .then(|| Self::compression_contract_for_context(context))
            .flatten();
        let mut run_manifest = context.custom_data.get("deep_review_run_manifest").cloned();
        let mut existing_cache = run_manifest
            .as_ref()
            .and_then(|manifest| manifest.get("deepReviewCache"))
            .cloned();
        if deep_review && (run_manifest.is_none() || existing_cache.is_none()) {
            if let (Some(session_id), Some(workspace), Some(coordinator)) = (
                context.session_id.as_deref(),
                context.workspace.as_ref(),
                global_coordinator(),
            ) {
                let session_storage_path = workspace.session_storage_path();
                match coordinator
                    .session_manager()
                    .load_session_metadata(&session_storage_path, session_id)
                    .await
                {
                    Ok(Some(metadata)) => {
                        if run_manifest.is_none() {
                            run_manifest = metadata.deep_review_run_manifest;
                        }
                        if existing_cache.is_none() {
                            existing_cache = metadata.deep_review_cache;
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        warn!(
                            "Failed to load DeepReview session metadata for review cache: session_id={}, error={}",
                            session_id, error
                        );
                    }
                }
            }
        }
        Self::validate_and_fill_defaults(
            &mut filled_input,
            deep_review,
            run_manifest.as_ref(),
            compression_contract.as_ref(),
        );
        if deep_review {
            Self::fill_deep_review_runtime_tracker_signals(&mut filled_input, context.dialog_turn_id.as_deref());
            Self::log_deep_review_runtime_diagnostics(context.dialog_turn_id.as_deref());
            if let Some(cache_update) = deep_review_report::deep_review_cache_from_completed_reviewers(
                &filled_input,
                run_manifest.as_ref(),
                existing_cache.as_ref(),
            ) {
                deep_review_report::fill_deep_review_cache_update_signals(&mut filled_input, &cache_update);
                if let Err(error) = Self::persist_deep_review_cache(context, cache_update.value).await {
                    warn!("Failed to persist DeepReview incremental cache: error={}", error);
                }
            }
        }

        Ok(vec![ToolResult::Result {
            data: filled_input,
            result_for_assistant: Some("Code review results submitted successfully".to_string()),
            image_attachments: None,
        }])
    }
}
