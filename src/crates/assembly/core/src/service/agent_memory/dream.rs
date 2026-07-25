//! Dream sweep: periodic cleanup of stale facts.
//!
//! Known limitations:
//! - JSONL side does not write superseded markers (DB is authoritative, avoids
//!   read_facts parse warnings).
//! - keep does not touch the fact (relies on 7-day review exemption to prevent
//!   re-sending the same fact for review).

use crate::infrastructure::ai::AIClient;
use crate::service::agent_memory::distiller::resolve_memory_llm_client;
use crate::service::agent_memory::facts::Fact;
use crate::service::agent_memory::judge_memory::{get_judge_state, set_judge_state};
use crate::service::agent_memory::memory_db::{default_memory_db_path, FactReview, MemoryDb};
use crate::util::types::Message;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

const DREAM_SWEEP_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000; // 24h
const STALE_THRESHOLD_MS: u64 = 30 * 86_400_000; // 30 days
const MAX_STALE_FACTS: usize = 20;
const DREAM_KEEP_EXEMPTION_DAYS: u64 = 7;
const DREAM_LLM_TIMEOUT_SECS: u64 = 15;
const MAX_REASON_CHARS: usize = 200;

/// Run a dream sweep for the given workspace.
///
/// This is fully warn-only: failures are logged and never propagated.
pub(crate) async fn run_dream_sweep(workspace_root: &std::path::Path) {
    // a) Resolve LLM client; None means disabled or unavailable.
    let client = match resolve_memory_llm_client().await {
        Some(c) => c,
        None => return,
    };

    // b) Open DB and check last sweep time.
    let db_path = default_memory_db_path();
    let db = match MemoryDb::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            warn!("Dream: failed to open memory db: {}", e);
            return;
        }
    };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let last_sweep = match get_judge_state(&db, "dream_last_sweep_at") {
        Ok(Some(v)) => match v.parse::<u64>() {
            Ok(ts) => ts,
            Err(_) => 0,
        },
        _ => 0,
    };

    if now_ms.saturating_sub(last_sweep) < DREAM_SWEEP_INTERVAL_MS {
        return;
    }

    // c) Get stale facts.
    let ws_key = workspace_root.to_string_lossy().to_string();
    let stale_facts = match db.get_stale_facts(
        Some(&ws_key),
        now_ms.saturating_sub(STALE_THRESHOLD_MS),
        MAX_STALE_FACTS,
    ) {
        Ok(facts) => facts,
        Err(e) => {
            warn!("Dream: failed to get stale facts: {}", e);
            return;
        }
    };

    if stale_facts.is_empty() {
        let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
        return;
    }

    // d) 7-day keep exemption.
    let mut candidates: Vec<&Fact> = Vec::new();
    for fact in &stale_facts {
        let reviews = match db.reviews_for_fact(&fact.id) {
            Ok(r) => r,
            Err(e) => {
                warn!("Dream: failed to get reviews for fact {}: {}", fact.id, e);
                continue;
            }
        };
        let has_recent_keep = reviews.iter().any(|r| {
            r.reviewer == "dream"
                && r.action == "keep"
                && now_ms.saturating_sub(r.created_at) < DREAM_KEEP_EXEMPTION_DAYS * 86_400_000
        });
        if !has_recent_keep {
            candidates.push(fact);
        }
    }

    if candidates.is_empty() {
        let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
        return;
    }

    // e) LLM batch judgment.
    let messages = build_dream_messages(&candidates);
    let response = match tokio::time::timeout(
        Duration::from_secs(DREAM_LLM_TIMEOUT_SECS),
        client.send_message(messages, None),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            warn!("Dream: AI call failed: {}", e);
            let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
            return;
        }
        Err(_) => {
            warn!(
                "Dream: AI call timed out after {}s",
                DREAM_LLM_TIMEOUT_SECS
            );
            let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
            return;
        }
    };

    let text = response.text;
    if text.trim().is_empty() {
        let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
        return;
    }

    // f) Parse verdicts.
    let verdicts = parse_dream_verdicts(&text, candidates.len());

    // g) Apply verdicts.
    let mut scanned = 0;
    let mut superseded = 0;
    let mut kept = 0;
    let mut skipped = 0;

    for (idx, action, reason) in verdicts {
        scanned += 1;
        if idx >= candidates.len() {
            skipped += 1;
            continue;
        }
        let fact = candidates[idx];
        match action.as_str() {
            "supersede" => {
                if let Err(e) = db.supersede_fact(&fact.id, None, now_ms) {
                    warn!("Dream: failed to supersede fact {}: {}", fact.id, e);
                } else {
                    superseded += 1;
                }
                let review = FactReview {
                    id: Uuid::new_v4().to_string(),
                    fact_id: fact.id.clone(),
                    reviewer: "dream".to_string(),
                    action: "supersede".to_string(),
                    reason,
                    created_at: now_ms,
                };
                if let Err(e) = db.record_fact_review(&review) {
                    warn!(
                        "Dream: failed to record supersede review for fact {}: {}",
                        fact.id, e
                    );
                }
            }
            "keep" => {
                kept += 1;
                let review = FactReview {
                    id: Uuid::new_v4().to_string(),
                    fact_id: fact.id.clone(),
                    reviewer: "dream".to_string(),
                    action: "keep".to_string(),
                    reason,
                    created_at: now_ms,
                };
                if let Err(e) = db.record_fact_review(&review) {
                    warn!(
                        "Dream: failed to record keep review for fact {}: {}",
                        fact.id, e
                    );
                }
            }
            _ => {
                skipped += 1;
            }
        }
    }

    // h) Write last sweep time and summary.
    let _ = set_judge_state(&db, "dream_last_sweep_at", &now_ms.to_string(), now_ms);
    info!(
        "Dream sweep: scanned={}, superseded={}, kept={}, skipped={}",
        scanned, superseded, kept, skipped
    );
}

fn build_dream_messages(facts: &[&Fact]) -> Vec<Message> {
    let system_prompt = r#"You are a memory curation assistant. Judge whether each fact is still valid, outdated, overturned, or has no long-term value.

Actions:
- "supersede": the fact is outdated, overturned, or has no long-term value.
- "keep": the fact is still valid or you are unsure.

Output a strict JSON array: [{"index": 0, "action": "keep"|"supersede", "reason": "..."}]

Rules:
- index must match the fact index in the input list (0-based).
- action must be exactly "keep" or "supersede".
- reason is optional, max 200 characters.
- If unsure, choose "keep"."#;

    let mut user_content = String::from("Facts to review:\n");
    for (i, fact) in facts.iter().enumerate() {
        user_content.push_str(&format!("{}. {}\n", i, fact.text));
    }

    vec![
        Message::system(system_prompt.to_string()),
        Message::user(user_content),
    ]
}

/// Parse the LLM's JSON verdict response.
///
/// Tolerates ```json fence wrapping. Bad JSON returns empty Vec. Index out of
/// bounds (>= fact_count) and unknown actions are skipped. Reason is truncated
/// to 200 characters.
fn parse_dream_verdicts(json: &str, fact_count: usize) -> Vec<(usize, String, Option<String>)> {
    let cleaned = strip_json_fence(json);
    let raw: Vec<RawDreamVerdict> = match serde_json::from_str(&cleaned) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for item in raw {
        let idx = item.index;
        if idx >= fact_count {
            continue;
        }
        let action = match item.action.as_deref() {
            Some("keep") | Some("supersede") => item.action.unwrap_or_default(),
            _ => continue,
        };
        let reason = item.reason.map(|r| {
            if r.chars().count() > MAX_REASON_CHARS {
                r.chars().take(MAX_REASON_CHARS).collect()
            } else {
                r
            }
        });
        results.push((idx, action, reason));
    }
    results
}

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

#[derive(Debug, serde::Deserialize)]
struct RawDreamVerdict {
    index: usize,
    action: Option<String>,
    reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json_array_maps_fields() {
        let json = r#"[{"index": 0, "action": "keep", "reason": "still valid"}]"#;
        let verdicts = parse_dream_verdicts(json, 1);
        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0], (0, "keep".to_string(), Some("still valid".to_string())));
    }

    #[test]
    fn parse_fence_tolerant() {
        let json = r#"```json
[{"index": 0, "action": "supersede", "reason": "outdated"}]
```"#;
        let verdicts = parse_dream_verdicts(json, 1);
        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0].1, "supersede");
    }

    #[test]
    fn parse_bad_json_returns_empty() {
        let json = r#"not json at all"#;
        let verdicts = parse_dream_verdicts(json, 1);
        assert!(verdicts.is_empty());
    }

    #[test]
    fn parse_index_out_of_bounds_skipped() {
        let json = r#"[{"index": 5, "action": "keep", "reason": "too high"}]"#;
        let verdicts = parse_dream_verdicts(json, 2);
        assert!(verdicts.is_empty());
    }

    #[test]
    fn parse_unknown_action_skipped() {
        let json = r#"[{"index": 0, "action": "maybe", "reason": "unclear"}]"#;
        let verdicts = parse_dream_verdicts(json, 1);
        assert!(verdicts.is_empty());
    }

    #[test]
    fn parse_reason_truncated() {
        let long_reason = "a".repeat(250);
        let json = format!(r#"[{{"index": 0, "action": "keep", "reason": "{}"}}]"#, long_reason);
        let verdicts = parse_dream_verdicts(&json, 1);
        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0].2.as_ref().unwrap().chars().count(), MAX_REASON_CHARS);
    }
}
