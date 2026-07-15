//! Facet extraction (areas and interaction style) for InsightsService.

use crate::agentic::insights::prompt_context::{aggregate_stats_json_for_prompt, summaries_block};
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
    pub(crate) async fn identify_areas(
        ai_client: &Arc<AIClient>,
        aggregate: &InsightsAggregate,
        lang_instruction: &str,
    ) -> NortHingResult<Vec<ProjectArea>> {
        let aggregate_json = aggregate_stats_json_for_prompt(aggregate);
        let summaries = summaries_block(aggregate);

        let prompt = format!(
            "{}{}",
            AREAS_PROMPT_TEMPLATE
                .replace("{aggregate_json}", &aggregate_json)
                .replace("{summaries}", &summaries),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Areas AI call failed: {}", e)))?;

        info!(
            "Areas response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Areas text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse areas JSON: {}", e)))?;

        Ok(value["areas"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ProjectArea {
                            name: v["name"].as_str()?.to_string(),
                            session_count: v["session_count"].as_u64().unwrap_or(0) as u32,
                            description: v["description"].as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub(crate) async fn analyze_interaction_style(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        summaries: &str,
        lang_instruction: &str,
    ) -> NortHingResult<InteractionStyleResult> {
        let prompt = format!(
            "{}{}",
            INTERACTION_STYLE_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{summaries}", summaries),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Interaction Style AI call failed: {}", e)))?;

        info!(
            "Interaction Style response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Interaction Style text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse interaction style JSON: {}", e)))?;

        Ok(InteractionStyleResult {
            narrative: value["narrative"].as_str().unwrap_or("").to_string(),
            key_patterns: value["key_patterns"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }
}
