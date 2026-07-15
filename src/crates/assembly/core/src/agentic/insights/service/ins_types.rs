//! Intermediate result types, constants, and helper functions for InsightsService.

use crate::agentic::insights::types::*;
use crate::util::errors::{NortHingError, NortHingResult};
use serde_json::Value;

// ============ Prompt templates ============

pub(crate) const FACET_PROMPT_TEMPLATE: &str = include_str!("../prompts/facet_extraction.md");
pub(crate) const SUGGESTIONS_PROMPT_TEMPLATE: &str = include_str!("../prompts/suggestions.md");
pub(crate) const AREAS_PROMPT_TEMPLATE: &str = include_str!("../prompts/areas.md");
pub(crate) const WINS_PROMPT_TEMPLATE: &str = include_str!("../prompts/wins.md");
pub(crate) const FRICTION_PROMPT_TEMPLATE: &str = include_str!("../prompts/friction.md");
pub(crate) const INTERACTION_STYLE_PROMPT_TEMPLATE: &str = include_str!("../prompts/interaction_style.md");
pub(crate) const AT_A_GLANCE_PROMPT_TEMPLATE: &str = include_str!("../prompts/at_a_glance.md");
pub(crate) const HORIZON_PROMPT_TEMPLATE: &str = include_str!("../prompts/horizon.md");
pub(crate) const FUN_ENDING_PROMPT_TEMPLATE: &str = include_str!("../prompts/fun_ending.md");

pub(crate) const MAX_CONCURRENT_FACET_EXTRACTIONS: usize = 5;

// ============ Intermediate result types ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WinsFrictionResult {
    #[serde(default)]
    pub(crate) wins_intro: String,
    pub(crate) big_wins: Vec<BigWin>,
    #[serde(default)]
    pub(crate) friction_intro: String,
    pub(crate) friction_categories: Vec<FrictionCategory>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WinsResult {
    pub(crate) intro: String,
    pub(crate) big_wins: Vec<BigWin>,
}

impl WinsResult {
    pub(crate) fn default() -> Self {
        Self {
            intro: String::new(),
            big_wins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct FrictionResult {
    pub(crate) intro: String,
    pub(crate) friction_categories: Vec<FrictionCategory>,
}

impl FrictionResult {
    pub(crate) fn default() -> Self {
        Self {
            intro: String::new(),
            friction_categories: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct InteractionStyleResult {
    pub(crate) narrative: String,
    pub(crate) key_patterns: Vec<String>,
}

impl InteractionStyleResult {
    pub(crate) fn default() -> Self {
        Self {
            narrative: String::new(),
            key_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HorizonResult {
    pub(crate) intro: String,
    pub(crate) opportunities: Vec<HorizonWorkflow>,
}

impl HorizonResult {
    pub(crate) fn default() -> Self {
        Self {
            intro: String::new(),
            opportunities: Vec::new(),
        }
    }
}

impl AtAGlance {
    pub(crate) fn default() -> Self {
        Self {
            whats_working: "Analysis in progress...".to_string(),
            whats_hindering: String::new(),
            quick_wins: String::new(),
            looking_ahead: String::new(),
        }
    }
}

// ============ Helper functions ============

pub(crate) fn is_rate_limit_error(e: &NortHingError) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") || msg.contains("rate_limit")
}

pub(crate) fn is_retryable_error(e: &NortHingError) -> bool {
    if is_rate_limit_error(e) {
        return true;
    }
    let msg = e.to_string().to_lowercase();
    msg.contains("cannot extract json")
        || msg.contains("sse stream closed")
        || msg.contains("stream closed before")
        || msg.contains("connection reset")
}

pub(crate) fn default_suggestions() -> InsightsSuggestions {
    InsightsSuggestions {
        northhing_md_additions: Vec::new(),
        features_to_try: Vec::new(),
        usage_patterns: Vec::new(),
    }
}

pub(crate) fn parse_string_u32_map(value: &Value) -> std::collections::HashMap<String, u32> {
    let mut map = std::collections::HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if let Some(n) = v.as_u64() {
                map.insert(k.clone(), n as u32);
            } else if let Some(n) = v.as_f64() {
                map.insert(k.clone(), n as u32);
            }
        }
    }
    map
}

pub(crate) fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub(crate) fn extract_json_from_response(response: &str) -> NortHingResult<String> {
    crate::util::extract_json_from_ai_response(response)
        .ok_or_else(|| NortHingError::service("Cannot extract JSON from AI response"))
}

/// Extract a string from a JSON value that may be a plain string or a nested object.
/// When the value is an object, concatenate all string values with spaces.
pub(crate) fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(map) => map
            .values()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}
