//! Free helpers for `RoundExecutor` that produce structured JSON values
//! derived from per-round model usage records.
//!
//! Sibling module to `round_executor/mod.rs` (Round 47c split). Currently only holds
//! `token_details_from_usage`, used by `RoundExecutor::emit_token_usage_update`
//! (defined in `rexec_run.rs`) to serialize token-stats cache/reasoning fields
//! into the `TokenUsageUpdated` event payload.

/// Build the optional `tokenDetails` JSON object for a `TokenUsageUpdated`
/// event.
///
/// Emits the present keys among
/// `reasoningTokenCount` / `cachedContentTokenCount` /
/// `cacheCreationTokenCount`. Returns `None` when the usage record carries no
/// cache or reasoning info.
pub(in crate::agentic::execution) fn token_details_from_usage(
    usage: &crate::util::types::ai::GeminiUsage,
) -> Option<serde_json::Value> {
    let mut details = serde_json::Map::new();
    if let Some(reasoning_tokens) = usage.reasoning_token_count {
        details.insert("reasoningTokenCount".to_string(), serde_json::json!(reasoning_tokens));
    }
    if let Some(cached_tokens) = usage.cached_content_token_count {
        details.insert("cachedContentTokenCount".to_string(), serde_json::json!(cached_tokens));
    }
    // Cache writes (Anthropic only at the moment). Disjoint from reads.
    if let Some(creation_tokens) = usage.cache_creation_token_count {
        details.insert(
            "cacheCreationTokenCount".to_string(),
            serde_json::json!(creation_tokens),
        );
    }

    (!details.is_empty()).then_some(serde_json::Value::Object(details))
}
