use crate::stream::types::unified::UnifiedTokenUsage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSSEData {
    #[serde(default)]
    pub candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    pub usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(default)]
    pub prompt_feedback: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCandidate {
    #[serde(default)]
    pub content: Option<GeminiContent>,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub grounding_metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub safety_ratings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiContent {
    #[serde(default)]
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiPart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub thought: Option<bool>,
    #[serde(default)]
    pub thought_signature: Option<String>,
    #[serde(default)]
    pub function_call: Option<GeminiFunctionCall>,
    #[serde(default)]
    pub executable_code: Option<GeminiExecutableCode>,
    #[serde(default)]
    pub code_execution_result: Option<GeminiCodeExecutionResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiFunctionCall {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiExecutableCode {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCodeExecutionResult {
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    #[serde(default)]
    pub prompt_token_count: u32,
    #[serde(default)]
    pub candidates_token_count: u32,
    #[serde(default)]
    pub total_token_count: u32,
    #[serde(default)]
    pub thoughts_token_count: Option<u32>,
    #[serde(default)]
    pub cached_content_token_count: Option<u32>,
}

impl From<GeminiUsageMetadata> for UnifiedTokenUsage {
    fn from(usage: GeminiUsageMetadata) -> Self {
        let reasoning_token_count = usage.thoughts_token_count;
        let candidates_token_count = usage
            .candidates_token_count
            .saturating_add(reasoning_token_count.unwrap_or(0));
        Self {
            prompt_token_count: usage.prompt_token_count,
            candidates_token_count,
            total_token_count: usage.total_token_count,
            reasoning_token_count,
            cached_content_token_count: usage.cached_content_token_count,
            cache_creation_token_count: None,
        }
    }
}
