//! Suggestions generation for InsightsService.

use crate::agentic::insights::prompt_context::{
    aggregate_stats_json_for_prompt, friction_block, summaries_block, user_instructions_block,
};
use crate::agentic::insights::types::*;
use crate::infrastructure::ai::AIClient;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::Message;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info};

use super::super::ins_types::*;
use super::InsightsService;

impl InsightsService {
    pub(crate) async fn generate_suggestions(
        ai_client: &Arc<AIClient>,
        aggregate: &InsightsAggregate,
        lang_instruction: &str,
    ) -> NortHingResult<InsightsSuggestions> {
        let aggregate_json = aggregate_stats_json_for_prompt(aggregate);
        let summaries = summaries_block(aggregate);
        let friction_details = friction_block(aggregate);
        let user_instructions = user_instructions_block(aggregate);

        let prompt = format!(
            "{}{}",
            SUGGESTIONS_PROMPT_TEMPLATE
                .replace("{aggregate_json}", &aggregate_json)
                .replace("{summaries}", &summaries)
                .replace("{friction_details}", &friction_details)
                .replace("{user_instructions}", &user_instructions),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Suggestions AI call failed: {}", e)))?;

        info!(
            "Suggestions response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Suggestions text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str).map_err(|e| {
            NortHingError::Deserialization(format!(
                "Failed to parse suggestions JSON: {}. Raw: {}",
                e,
                safe_truncate(&json_str, 500)
            ))
        })?;

        debug!(
            "Suggestions parsed: md_additions={}, features={}, patterns={}",
            value["northhing_md_additions"].as_array().map(|a| a.len()).unwrap_or(0),
            value["features_to_try"].as_array().map(|a| a.len()).unwrap_or(0),
            value["usage_patterns"].as_array().map(|a| a.len()).unwrap_or(0),
        );

        Ok(InsightsSuggestions {
            northhing_md_additions: value["northhing_md_additions"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(MdAddition {
                                section: v["section"].as_str()?.to_string(),
                                content: v["content"].as_str()?.to_string(),
                                rationale: v["rationale"].as_str().or(v["why"].as_str()).unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            features_to_try: value["features_to_try"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(FeatureRecommendation {
                                feature: v["feature"].as_str()?.to_string(),
                                description: v["description"]
                                    .as_str()
                                    .or(v["one_liner"].as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                example_usage: v["example_usage"]
                                    .as_str()
                                    .or(v["example_code"].as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                benefit: v["benefit"]
                                    .as_str()
                                    .or(v["why_for_you"].as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            usage_patterns: value["usage_patterns"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|v| UsagePattern {
                            pattern: v["pattern"].as_str().or(v["title"].as_str()).unwrap_or("").to_string(),
                            description: v["description"]
                                .as_str()
                                .or(v["suggestion"].as_str())
                                .unwrap_or("")
                                .to_string(),
                            detail: v["detail"].as_str().unwrap_or("").to_string(),
                            suggested_prompt: v["suggested_prompt"]
                                .as_str()
                                .or(v["copyable_prompt"].as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}
