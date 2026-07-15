//! Aggregate overview generation (at-a-glance, horizon, fun ending) for InsightsService.

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
    pub(crate) async fn generate_at_a_glance(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        areas_text: &str,
        suggestions_text: &str,
        wins_friction_text: &str,
        interaction_text: &str,
        lang_instruction: &str,
    ) -> NortHingResult<AtAGlance> {
        let prompt = format!(
            "{}{}",
            AT_A_GLANCE_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{areas}", areas_text)
                .replace("{suggestions}", suggestions_text)
                .replace("{wins_and_friction}", wins_friction_text)
                .replace("{interaction_style}", interaction_text),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("At a Glance AI call failed: {}", e)))?;

        info!(
            "At a Glance response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("At a Glance text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse at-a-glance JSON: {}", e)))?;

        let looking_ahead = {
            let v = json_value_to_string(&value["looking_ahead"]);
            if v.is_empty() {
                json_value_to_string(&value["ambitious_workflows"])
            } else {
                v
            }
        };

        Ok(AtAGlance {
            whats_working: json_value_to_string(&value["whats_working"]),
            whats_hindering: json_value_to_string(&value["whats_hindering"]),
            quick_wins: json_value_to_string(&value["quick_wins"]),
            looking_ahead,
        })
    }

    pub(crate) async fn generate_horizon(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        summaries: &str,
        friction_details: &str,
        lang_instruction: &str,
    ) -> NortHingResult<HorizonResult> {
        let prompt = format!(
            "{}{}",
            HORIZON_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{summaries}", summaries)
                .replace("{friction_details}", friction_details),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Horizon AI call failed: {}", e)))?;

        info!(
            "Horizon response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Horizon text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse horizon JSON: {}", e)))?;

        Ok(HorizonResult {
            intro: value["intro"].as_str().unwrap_or("").to_string(),
            opportunities: value["opportunities"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(HorizonWorkflow {
                                title: v["title"].as_str()?.to_string(),
                                whats_possible: v["whats_possible"].as_str()?.to_string(),
                                how_to_try: v["how_to_try"].as_str().unwrap_or("").to_string(),
                                copyable_prompt: v["copyable_prompt"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub(crate) async fn generate_fun_ending(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        summaries: &str,
        lang_instruction: &str,
    ) -> NortHingResult<Option<FunEnding>> {
        let prompt = format!(
            "{}{}",
            FUN_ENDING_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{summaries}", summaries),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Fun Ending AI call failed: {}", e)))?;

        info!(
            "Fun Ending response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Fun Ending text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse fun ending JSON: {}", e)))?;

        Ok(Some(FunEnding {
            headline: value["headline"]
                .as_str()
                .or(value["title"].as_str())
                .unwrap_or("")
                .to_string(),
            detail: value["detail"]
                .as_str()
                .or(value["message"].as_str())
                .unwrap_or("")
                .to_string(),
        }))
    }
}
