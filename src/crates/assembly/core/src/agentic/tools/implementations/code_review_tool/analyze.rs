//! Analysis logic for `CodeReviewTool`.
//!
//! Owns input validation/default-filling and all DeepReview-specific
//! runtime integration (packet metadata, reliability signals, cache,
//! runtime tracker signals).

use crate::agentic::core::CompressionContract;
use crate::agentic::deep_review::report as deep_review_report;
use crate::util::errors::NortHingResult;
use serde_json::{json, Value};
use tracing::warn;

impl super::CodeReviewTool {
    /// Validate and fill missing fields with default values.
    ///
    /// When AI-returned data is missing certain fields, fill with default values to avoid entire review failure.
    pub fn validate_and_fill_defaults(
        input: &mut Value,
        deep_review: bool,
        run_manifest: Option<&Value>,
        compression_contract: Option<&CompressionContract>,
    ) {
        // Fill summary default values
        if input.get("summary").is_none() {
            warn!("CodeReview tool missing summary field, using default values");
            input["summary"] = json!({
                "overall_assessment": "None",
                "risk_level": "low",
                "recommended_action": "approve",
                "confidence_note": "AI did not return complete review results"
            });
        } else if let Some(summary) = input.get_mut("summary") {
            if summary.get("overall_assessment").is_none() {
                summary["overall_assessment"] = json!("None");
            }
            if summary.get("risk_level").is_none() {
                summary["risk_level"] = json!("low");
            }
            if summary.get("recommended_action").is_none() {
                summary["recommended_action"] = json!("approve");
            }
        } else {
            warn!("CodeReview tool summary field exists but is not mutable object, using default values");
            input["summary"] = json!({
                "overall_assessment": "None",
                "risk_level": "low",
                "recommended_action": "approve",
                "confidence_note": "AI returned invalid summary format"
            });
        }

        // Fill issues default values
        if input.get("issues").is_none() {
            warn!("CodeReview tool missing issues field, using default values");
            input["issues"] = json!([]);
        }

        // Fill positive_points default values
        if input.get("positive_points").is_none() {
            warn!("CodeReview tool missing positive_points field, using default values");
            input["positive_points"] = json!(["None"]);
        }

        if deep_review {
            input["review_mode"] = json!("deep");
            if input.get("review_scope").is_none() {
                input["review_scope"] = json!("Deep review scope was not provided");
            }
        } else if input.get("review_mode").is_none() {
            input["review_mode"] = json!("standard");
        }

        if input.get("reviewers").is_none() {
            input["reviewers"] = json!([]);
        }
        if deep_review {
            Self::fill_deep_review_packet_metadata(input, run_manifest);
            Self::fill_deep_review_reliability_signals(input, run_manifest, compression_contract);
        }

        if input.get("remediation_plan").is_none() {
            input["remediation_plan"] = json!([]);
        }

        if input.get("schema_version").is_none() {
            input["schema_version"] = json!(1);
        }
    }

    /// Fill DeepReview packet metadata for reviewer entries inferred from the run manifest.
    pub fn fill_deep_review_packet_metadata(input: &mut Value, run_manifest: Option<&Value>) {
        deep_review_report::fill_deep_review_packet_metadata(input, run_manifest);
    }

    /// Append reliability signals derived from the run manifest and compression contract.
    pub fn fill_deep_review_reliability_signals(
        input: &mut Value,
        run_manifest: Option<&Value>,
        compression_contract: Option<&CompressionContract>,
    ) {
        deep_review_report::fill_deep_review_reliability_signals(input, run_manifest, compression_contract);
    }

    /// Append runtime tracker signals for the current dialog turn.
    pub fn fill_deep_review_runtime_tracker_signals(input: &mut Value, dialog_turn_id: Option<&str>) {
        deep_review_report::fill_deep_review_runtime_tracker_signals(input, dialog_turn_id);
    }

    /// Emit shared-context diagnostics for the current dialog turn (if measured).
    pub fn log_deep_review_runtime_diagnostics(dialog_turn_id: Option<&str>) {
        deep_review_report::log_deep_review_runtime_diagnostics(dialog_turn_id);
    }

    /// Persist the incremental DeepReview cache value.
    pub async fn persist_deep_review_cache(
        context: &crate::agentic::tools::framework::ToolUseContext,
        cache_value: Value,
    ) -> NortHingResult<()> {
        deep_review_report::persist_deep_review_cache(context, cache_value).await
    }

    /// Compute the reliability contract limit for the given agent type and model id.
    #[cfg(test)]
    pub fn reliability_contract_limit(agent_type: Option<&str>, model_id: Option<&str>) -> usize {
        deep_review_report::reliability_contract_limit(agent_type, model_id)
    }

    /// Decide whether the `compression_preserved` signal should be reported.
    #[cfg(test)]
    pub fn should_report_compression_preserved(
        compression_count: usize,
        compression_contract: Option<&CompressionContract>,
    ) -> bool {
        deep_review_report::should_report_compression_preserved(compression_count, compression_contract)
    }
}
