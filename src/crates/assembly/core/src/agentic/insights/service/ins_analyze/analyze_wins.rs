//! Wins and friction derivation for InsightsService.

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
    pub(crate) async fn analyze_wins(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        summaries: &str,
        lang_instruction: &str,
    ) -> NortHingResult<WinsResult> {
        let prompt = format!(
            "{}{}",
            WINS_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{summaries}", summaries),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Wins AI call failed: {}", e)))?;

        info!(
            "Wins response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Wins text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse wins JSON: {}", e)))?;

        Ok(WinsResult {
            intro: value["intro"].as_str().unwrap_or("").to_string(),
            big_wins: value["impressive_workflows"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(BigWin {
                                title: v["title"].as_str()?.to_string(),
                                description: v["description"].as_str()?.to_string(),
                                impact: v["impact"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub(crate) async fn analyze_friction(
        ai_client: &Arc<AIClient>,
        aggregate_json: &str,
        summaries: &str,
        friction_details: &str,
        lang_instruction: &str,
    ) -> NortHingResult<FrictionResult> {
        let prompt = format!(
            "{}{}",
            FRICTION_PROMPT_TEMPLATE
                .replace("{aggregate_json}", aggregate_json)
                .replace("{summaries}", summaries)
                .replace("{friction_details}", friction_details),
            lang_instruction
        );

        let messages = vec![Message::user(prompt)];
        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("Friction AI call failed: {}", e)))?;

        info!(
            "Friction response: len={}, finish={:?}",
            response.text.len(),
            response.finish_reason
        );
        debug!("Friction text: {}", safe_truncate(&response.text, 300));

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse friction JSON: {}", e)))?;

        Ok(FrictionResult {
            intro: value["intro"].as_str().unwrap_or("").to_string(),
            friction_categories: value["friction_categories"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(FrictionCategory {
                                category: v["category"].as_str()?.to_string(),
                                count: v["count"].as_u64().unwrap_or(0) as u32,
                                description: v["description"].as_str()?.to_string(),
                                examples: v["examples"]
                                    .as_array()
                                    .map(|a| a.iter().filter_map(|e| e.as_str().map(String::from)).collect())
                                    .unwrap_or_default(),
                                suggestion: v["suggestion"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}
