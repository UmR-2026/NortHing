//! LLM-based fact distillation for agent memory.
//!
//! Replaces the keyword-only `distill_facts_from_user_message` with an LLM-powered
//! channel. Falls back to the keyword path on any failure (cost gate, config off,
//! timeout, parse error) — never panics, never propagates errors to the caller.

use crate::infrastructure::ai::{get_global_ai_client_factory, AIClient, AIClientFactory};
use crate::service::config::{get_global_config_service, GlobalConfig};
use crate::service::agent_memory::facts::{
    distill_facts_from_user_message, Fact, FactConfidence, FactProvenance, FactScope, FactType,
};
use crate::util::types::Message;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Minimum user input length (chars) to trigger LLM distillation.
/// Shorter inputs fall back to keyword distillation.
const MIN_USER_INPUT_CHARS: usize = 20;
/// Maximum number of facts to distill per turn.
const MAX_DISTILL_FACTS: usize = 3;
/// Maximum chars per fact text.
const MAX_FACT_TEXT_CHARS: usize = 300;
/// Maximum chars of assistant reply to include as context.
const MAX_ASSISTANT_TEXT_CHARS: usize = 500;
/// LLM call timeout in seconds.
const DISTILL_TIMEOUT_SECS: u64 = 15;

/// Distill candidate facts from user input using an LLM, with keyword fallback.
///
/// On any failure (cost gate, config disabled, model resolution failure, timeout,
/// empty/error response, parse failure), falls back to `distill_facts_from_user_message`.
/// Never panics and never propagates errors to the caller.
pub(crate) async fn distill_facts_with_llm(
    user_input: &str,
    last_assistant_text: Option<&str>,
    session_id: &str,
    turn_id: &str,
) -> Vec<Fact> {
    // a) Cost gate: short inputs skip the LLM.
    if user_input.chars().count() < MIN_USER_INPUT_CHARS {
        return distill_facts_from_user_message(user_input, session_id, turn_id);
    }

    // b) Read config.
    let config = match read_distiller_config().await {
        Some(c) => c,
        None => return distill_facts_from_user_message(user_input, session_id, turn_id),
    };

    if !config.memory.distiller_enabled {
        return distill_facts_from_user_message(user_input, session_id, turn_id);
    }

    // c) Resolve the AI client (design M5).
    let client = match resolve_distiller_client(&config).await {
        Some(c) => c,
        None => return distill_facts_from_user_message(user_input, session_id, turn_id),
    };

    // d) Build the distillation prompt.
    let messages = build_distillation_messages(user_input, last_assistant_text);

    // e) Call the LLM with a timeout.
    let response = match tokio::time::timeout(
        Duration::from_secs(DISTILL_TIMEOUT_SECS),
        client.send_message(messages, None),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            warn!("Distiller: AI call failed: {}", e);
            return distill_facts_from_user_message(user_input, session_id, turn_id);
        }
        Err(_) => {
            warn!("Distiller: AI call timed out after {}s", DISTILL_TIMEOUT_SECS);
            return distill_facts_from_user_message(user_input, session_id, turn_id);
        }
    };

    // f) Extract text from the response.
    let text = response.text;
    if text.trim().is_empty() {
        return distill_facts_from_user_message(user_input, session_id, turn_id);
    }

    // g) Parse the JSON response.
    let facts = parse_distilled_facts(&text, session_id, turn_id);
    if facts.is_empty() {
        return distill_facts_from_user_message(user_input, session_id, turn_id);
    }

    facts
}

/// Read the global config, logging warnings on failure.
async fn read_distiller_config() -> Option<GlobalConfig> {
    let service = match get_global_config_service().await {
        Ok(s) => s,
        Err(e) => {
            warn!("Distiller: failed to get config service: {}", e);
            return None;
        }
    };
    match service.config(None).await {
        Ok(c) => Some(c),
        Err(e) => {
            warn!("Distiller: failed to read config: {}", e);
            None
        }
    }
}

/// Resolve the AI client for distillation per design M5.
///
/// If `distiller_model` is set (format "provider/model"), find a matching entry
/// in `config.ai.models` and use its id. Otherwise (or on any resolution
/// failure) fall back to "fast".
async fn resolve_distiller_client(
    config: &GlobalConfig,
) -> Option<Arc<AIClient>> {
    let factory = match get_global_ai_client_factory().await {
        Ok(f) => f,
        Err(e) => {
            warn!("Distiller: failed to get AI client factory: {}", e);
            return None;
        }
    };

    let model_ref = config.memory.distiller_model.as_deref();

    let client = match model_ref {
        Some(model_str) => {
            let parts: Vec<&str> = model_str.splitn(2, '/').collect();
            if parts.len() != 2 {
                warn!(
                    "Distiller: invalid distiller_model '{}', expected 'provider/model'. Falling back to fast.",
                    model_str
                );
                factory.get_client_resolved("fast").await
            } else {
                let (provider, model) = (parts[0], parts[1]);
                let matched_id = config
                    .ai
                    .models
                    .iter()
                    .find(|m| {
                        m.provider == provider
                            && (m.model_name == model || m.name == model || m.id == model)
                    })
                    .map(|m| m.id.clone());
                match matched_id {
                    Some(id) => factory.get_client_resolved(&id).await,
                    None => {
                        warn!(
                            "Distiller: no model found for provider='{}', model='{}'. Falling back to fast.",
                            provider, model
                        );
                        factory.get_client_resolved("fast").await
                    }
                }
            }
        }
        None => factory.get_client_resolved("fast").await,
    };

    match client {
        Ok(c) => Some(c),
        Err(e) => {
            warn!("Distiller: failed to get AI client: {}", e);
            None
        }
    }
}

/// Resolve the memory LLM client for background tasks (dream, etc.).
///
/// Returns None if distiller is disabled, config is missing, or client
/// resolution fails.
pub(crate) async fn resolve_memory_llm_client() -> Option<Arc<AIClient>> {
    let config = match read_distiller_config().await {
        Some(c) => c,
        None => return None,
    };

    if !config.memory.distiller_enabled {
        return None;
    }

    resolve_distiller_client(&config).await
}

/// Build the distillation prompt messages.
///
/// System message contains the extraction instructions. User message wraps the
/// user input in `<user_message>` tags and optionally appends the last assistant
/// reply (truncated) in `<assistant_reply>` tags.
fn build_distillation_messages(user_input: &str, last_assistant_text: Option<&str>) -> Vec<Message> {
    let system_prompt = r#"You are a memory extraction assistant. Extract facts worth remembering across sessions from the user's message.

Only record:
- User profile/preferences (role, goals, knowledge level, tool preferences)
- Collaboration feedback (corrections AND confirmations, with reasons)
- Project motivation/background (goals, deadlines, context behind work)
- External resource pointers (links, dashboards, tracking systems)

Do NOT record:
- Code patterns, conventions, architecture, file paths, or project structure
- Git history, recent changes, or who-changed-what
- Debugging solutions or fix recipes
- Ephemeral task details, in-progress work, temporary state

Output a strict JSON array, max 3 items. Each item:
{"text": "...", "fact_type": "user|feedback|project|reference", "confidence": "high|med|low", "scope": "workspace|global"}

Rules:
- text must be <=300 characters, self-contained, and understandable without the original message
- fact_type: user (profile/preferences), feedback (collaboration guidance), project (motivation/context), reference (external resource pointer)
- confidence: high (explicit, certain), med (implied, likely), low (uncertain, speculative)
- scope: workspace (specific to this project), global (applies across projects)
- If nothing worth remembering, output: []

Respond with ONLY the JSON array, no explanation, no markdown fences."#;

    let mut user_content = format!("<user_message>{}</user_message>", user_input);
    if let Some(assistant_text) = last_assistant_text {
        let truncated: String = assistant_text.chars().take(MAX_ASSISTANT_TEXT_CHARS).collect();
        user_content.push_str(&format!("\n\n<assistant_reply>{}</assistant_reply>", truncated));
    }

    vec![
        Message::system(system_prompt.to_string()),
        Message::user(user_content),
    ]
}

/// Parse the LLM's JSON response into Facts.
///
/// Tolerates ```json fence wrapping. All fields are optional in the intermediate
/// struct. Unknown enum values cause the entry to be skipped. Max 3 facts.
/// Total parse failure returns an empty Vec (caller falls back to keywords).
fn parse_distilled_facts(json: &str, session_id: &str, turn_id: &str) -> Vec<Fact> {
    let cleaned = strip_json_fence(json);
    let raw_facts: Vec<RawDistilledFact> = match serde_json::from_str(&cleaned) {
        Ok(facts) => facts,
        Err(e) => {
            warn!("Distiller: failed to parse distilled facts JSON: {}", e);
            return Vec::new();
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let mut facts = Vec::new();
    for raw in raw_facts {
        if facts.len() >= MAX_DISTILL_FACTS {
            break;
        }
        let text = match raw.text {
            Some(t) => {
                let t = t.trim().to_string();
                if t.is_empty() {
                    continue;
                }
                t.chars().take(MAX_FACT_TEXT_CHARS).collect()
            }
            None => continue,
        };
        let fact_type = match raw.fact_type.as_deref() {
            Some("user") => FactType::User,
            Some("feedback") => FactType::Feedback,
            Some("project") => FactType::Project,
            Some("reference") => FactType::Reference,
            _ => continue,
        };
        let confidence = match raw.confidence.as_deref() {
            Some("high") => FactConfidence::High,
            Some("med") => FactConfidence::Med,
            Some("low") => FactConfidence::Low,
            _ => continue,
        };
        let scope = match raw.scope.as_deref() {
            Some("workspace") => FactScope::Workspace,
            Some("global") => FactScope::Global,
            _ => continue,
        };
        facts.push(Fact {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            text,
            provenance: FactProvenance {
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
            },
            confidence,
            scope,
            fact_type,
            created_at: now,
        });
    }
    facts
}

/// Strip ```json fence wrapping from a JSON string.
fn strip_json_fence(json: &str) -> String {
    let mut s = json.trim();
    if s.starts_with("```") {
        s = &s[3..];
        if s.starts_with("json") {
            s = &s[4..];
        }
        s = s.trim_start();
    }
    if s.ends_with("```") {
        s = &s[..s.len() - 3];
    }
    s.trim().to_string()
}

/// Intermediate struct for deserializing distilled facts (all fields optional).
#[derive(Debug, serde::Deserialize)]
struct RawDistilledFact {
    text: Option<String>,
    fact_type: Option<String>,
    confidence: Option<String>,
    scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json_array_maps_fields() {
        let json = r#"[{"text":"User prefers pnpm","fact_type":"user","confidence":"high","scope":"workspace"}]"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].text, "User prefers pnpm");
        assert_eq!(facts[0].fact_type, FactType::User);
        assert_eq!(facts[0].confidence, FactConfidence::High);
        assert_eq!(facts[0].scope, FactScope::Workspace);
        assert_eq!(facts[0].provenance.session_id, "s1");
        assert_eq!(facts[0].provenance.turn_id, "t1");
    }

    #[test]
    fn parse_json_fence_wrap() {
        let json = r#"```json
[{"text":"User prefers pnpm","fact_type":"user","confidence":"high","scope":"workspace"}]
```"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].text, "User prefers pnpm");
    }

    #[test]
    fn parse_bad_json_returns_empty() {
        let json = r#"not json at all"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert!(facts.is_empty());
    }

    #[test]
    fn parse_four_items_truncates_to_three() {
        let json = r#"[
            {"text":"Fact one","fact_type":"user","confidence":"high","scope":"workspace"},
            {"text":"Fact two","fact_type":"feedback","confidence":"med","scope":"global"},
            {"text":"Fact three","fact_type":"project","confidence":"low","scope":"workspace"},
            {"text":"Fact four","fact_type":"reference","confidence":"high","scope":"global"}
        ]"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert_eq!(facts.len(), 3);
    }

    #[test]
    fn parse_unknown_fact_type_skipped_valid_kept() {
        let json = r#"[
            {"text":"Valid fact","fact_type":"user","confidence":"high","scope":"workspace"},
            {"text":"Bad type","fact_type":"unknown","confidence":"med","scope":"workspace"}
        ]"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].text, "Valid fact");
    }

    #[test]
    fn parse_text_over_300_chars_truncated() {
        let long_text = "a".repeat(400);
        let json = format!(
            r#"[{{"text":"{}","fact_type":"user","confidence":"high","scope":"workspace"}}]"#,
            long_text
        );
        let facts = parse_distilled_facts(&json, "s1", "t1");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].text.chars().count(), 300);
    }

    #[test]
    fn parse_empty_array_returns_empty() {
        let json = r#"[]"#;
        let facts = parse_distilled_facts(json, "s1", "t1");
        assert!(facts.is_empty());
    }
}
