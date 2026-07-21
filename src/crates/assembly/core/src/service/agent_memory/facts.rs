//! Structured facts storage for agent memory.
//! Facts are append-only, stored in `facts.jsonl` alongside the workspace memory directory.

use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

const FACTS_FILE_NAME: &str = "facts.jsonl";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Fact {
    pub schema_version: u32,
    pub id: String,
    pub text: String,
    pub provenance: FactProvenance,
    pub confidence: FactConfidence,
    pub scope: FactScope,
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FactProvenance {
    pub session_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactConfidence {
    High,
    Med,
    Low,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactScope {
    Workspace,
    Global,
}

impl Fact {
    /// Estimate tokens for the fact text using ceiling division (chars+3)/4.
    fn estimated_tokens(&self) -> usize {
        (self.text.chars().count() + 3) / 4
    }
}

/// Append facts to the facts.jsonl file in the memory directory.
/// Append-only; does not interfere with memory.md.
pub(crate) async fn append_facts(memory_dir: &Path, facts: &[Fact]) -> NortHingResult<()> {
    let facts_path = memory_dir.join(FACTS_FILE_NAME);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&facts_path)
        .await
        .map_err(|e| {
            NortHingError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open facts file {}: {}", facts_path.display(), e),
            ))
        })?;

    for fact in facts {
        let line = serde_json::to_string(fact).map_err(|e| {
            NortHingError::Serialization(e)
        })?;
        file.write_all(line.as_bytes()).await.map_err(|e| {
            NortHingError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write fact: {}", e),
            ))
        })?;
        file.write_all(b"\n").await.map_err(|e| {
            NortHingError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write newline: {}", e),
            ))
        })?;
    }

    debug!(
        "Appended {} facts to {}",
        facts.len(),
        facts_path.display()
    );

    Ok(())
}

/// Append candidate facts, skipping exact-text duplicates (history + batch).
/// Returns the number of facts actually appended. IO errors are logged as
/// warnings and result in no append (never propagated to the caller).
pub(crate) async fn append_facts_dedup(memory_dir: &Path, candidates: Vec<Fact>) -> usize {
    if candidates.is_empty() {
        return 0;
    }

    let existing = match read_facts(memory_dir).await {
        Ok(facts) => facts,
        Err(e) => {
            warn!(
                "Facts: failed to read existing facts for deduplication, skipping append: {}",
                e
            );
            return 0;
        }
    };

    let mut seen: std::collections::HashSet<String> = existing.iter().map(|f| f.text.clone()).collect();
    let new_facts: Vec<Fact> = candidates.into_iter().filter(|c| seen.insert(c.text.clone())).collect();

    if new_facts.is_empty() {
        return 0;
    }

    let appended = new_facts.len();
    if let Err(e) = append_facts(memory_dir, &new_facts).await {
        warn!("Facts: failed to append facts: {}", e);
        return 0;
    }
    appended
}

/// Read all facts from the facts.jsonl file.
/// Skips damaged lines with a warning.
pub(crate) async fn read_facts(memory_dir: &Path) -> NortHingResult<Vec<Fact>> {    let facts_path = memory_dir.join(FACTS_FILE_NAME);

    if !facts_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&facts_path).await.map_err(|e| {
        NortHingError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read facts file {}: {}", facts_path.display(), e),
        ))
    })?;

    let mut facts = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Fact>(line) {
            Ok(fact) => facts.push(fact),
            Err(e) => {
                warn!("Skipping damaged fact line: {}", e);
            }
        }
    }

    debug!("Read {} facts from {}", facts.len(), facts_path.display());

    Ok(facts)
}

/// Select facts for prompt injection.
/// Sorts by: scope Global > Workspace, confidence High > Med > Low, created_at new > old.
/// Truncates to fit within token budget.
pub fn select_facts_for_prompt(facts: &[Fact], token_budget: usize) -> Vec<Fact> {
    let mut sorted: Vec<Fact> = facts.to_vec();

    // Sort: Global first, then by confidence High>Med>Low, then by created_at new>old
    // Scope: Global (0) before Workspace (1) => lower ordinal = higher priority
    // Confidence: High (0) > Med (1) > Low (2) => lower ordinal = higher priority
    sorted.sort_by(|a, b| {
        let scope_order = |scope: &FactScope| match scope {
            FactScope::Global => 0,
            FactScope::Workspace => 1,
        };
        let scope_cmp = scope_order(&a.scope).cmp(&scope_order(&b.scope));
        if scope_cmp != std::cmp::Ordering::Equal {
            return scope_cmp;
        }

        let confidence_order = |conf: &FactConfidence| match conf {
            FactConfidence::High => 0,
            FactConfidence::Med => 1,
            FactConfidence::Low => 2,
        };
        let conf_cmp = confidence_order(&a.confidence).cmp(&confidence_order(&b.confidence));
        if conf_cmp != std::cmp::Ordering::Equal {
            return conf_cmp;
        }

        // Created_at: new > old (higher timestamp = higher priority)
        b.created_at.cmp(&a.created_at)
    });

    // Truncate to fit budget
    let mut selected = Vec::new();
    let mut used_tokens = 0;

    for fact in sorted {
        let fact_tokens = fact.estimated_tokens();
        if used_tokens + fact_tokens <= token_budget {
            selected.push(fact);
            used_tokens += fact_tokens;
        } else {
            break;
        }
    }

    debug!(
        "Selected {}/{} facts for prompt ({} tokens / {} budget)",
        selected.len(),
        facts.len(),
        used_tokens,
        token_budget
    );

    selected
}

/// Distill candidate facts from a user message.
/// Returns a list of candidate facts based on keyword matching.
/// Each candidate has confidence Med and scope Workspace.
pub fn distill_facts_from_user_message(
    user_input: &str,
    session_id: &str,
    turn_id: &str,
) -> Vec<Fact> {
    // Bilingual keyword triggers from task spec
    let keywords = [
        "以后", "记住", "记得", "不要", "别", "总是", "一直", "优先", "别再",
        "prefer", "always", "never", "remember", "from now on",
    ];

    let has_keyword = keywords.iter().any(|kw| user_input.contains(kw));
    if !has_keyword {
        return Vec::new();
    }

    // Split into sentences
    let sentences: Vec<&str> = user_input
        .split(|c| c == '。' || c == '.' || c == '！' || c == '!' || c == '？' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut facts = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    for sentence in sentences {
        // Check if sentence contains any keyword
        let sentence_has_keyword = keywords.iter().any(|kw| sentence.contains(kw));
        if !sentence_has_keyword {
            continue;
        }

        // Truncate to 300 chars
        let text: String = sentence.chars().take(300).collect();

        facts.push(Fact {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            text,
            provenance: FactProvenance {
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Workspace,
            created_at: now,
        });
    }

    debug!(
        "Distilled {} candidate facts from user message ({} chars)",
        facts.len(),
        user_input.len()
    );

    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact_with(scope: FactScope, confidence: FactConfidence, created_at: u64) -> Fact {
        Fact {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            text: "test fact".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence,
            scope,
            created_at,
        }
    }

    #[test]
    fn select_facts_respects_scope_global_first() {
        let facts = vec![
            fact_with(FactScope::Workspace, FactConfidence::High, 1000),
            fact_with(FactScope::Global, FactConfidence::Low, 500),
            fact_with(FactScope::Workspace, FactConfidence::Med, 800),
            fact_with(FactScope::Global, FactConfidence::High, 300),
        ];

        let selected = select_facts_for_prompt(&facts, 10000);

        // Global should come before Workspace
        assert_eq!(selected[0].scope, FactScope::Global);
        assert_eq!(selected[1].scope, FactScope::Global);
        assert!(selected.iter().skip(2).all(|f| f.scope == FactScope::Workspace));
    }

    #[test]
    fn select_facts_respects_confidence_high_first() {
        let facts = vec![
            fact_with(FactScope::Workspace, FactConfidence::Low, 1000),
            fact_with(FactScope::Workspace, FactConfidence::High, 900),
            fact_with(FactScope::Workspace, FactConfidence::Med, 800),
        ];

        let selected = select_facts_for_prompt(&facts, 10000);

        assert_eq!(selected[0].confidence, FactConfidence::High);
        assert_eq!(selected[1].confidence, FactConfidence::Med);
        assert_eq!(selected[2].confidence, FactConfidence::Low);
    }

    #[test]
    fn select_facts_respects_newer_first_within_same_scope_and_confidence() {
        let facts = vec![
            fact_with(FactScope::Workspace, FactConfidence::High, 100),
            fact_with(FactScope::Workspace, FactConfidence::High, 1000),
            fact_with(FactScope::Workspace, FactConfidence::High, 500),
        ];

        let selected = select_facts_for_prompt(&facts, 10000);

        assert_eq!(selected[0].created_at, 1000);
        assert_eq!(selected[1].created_at, 500);
        assert_eq!(selected[2].created_at, 100);
    }

    #[test]
    fn select_facts_truncates_within_budget() {
        let facts = vec![
            Fact {
                schema_version: 1,
                id: "1".to_string(),
                text: "a".repeat(400), // ~100 tokens
                provenance: FactProvenance {
                    session_id: "s1".to_string(),
                    turn_id: "t1".to_string(),
                },
                confidence: FactConfidence::High,
                scope: FactScope::Workspace,
                created_at: 1000,
            },
            Fact {
                schema_version: 1,
                id: "2".to_string(),
                text: "b".repeat(400), // ~100 tokens
                provenance: FactProvenance {
                    session_id: "s1".to_string(),
                    turn_id: "t1".to_string(),
                },
                confidence: FactConfidence::High,
                scope: FactScope::Workspace,
                created_at: 900,
            },
            Fact {
                schema_version: 1,
                id: "3".to_string(),
                text: "c".repeat(400), // ~100 tokens
                provenance: FactProvenance {
                    session_id: "s1".to_string(),
                    turn_id: "t1".to_string(),
                },
                confidence: FactConfidence::High,
                scope: FactScope::Workspace,
                created_at: 800,
            },
        ];

        // Budget for ~250 tokens (1000 chars)
        let selected = select_facts_for_prompt(&facts, 250);

        // Should fit 2 facts, 3rd would exceed
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_facts_empty_input_returns_empty() {
        let facts: Vec<Fact> = vec![];
        let selected = select_facts_for_prompt(&facts, 1000);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_facts_zero_budget_returns_empty() {
        let facts = vec![fact_with(FactScope::Workspace, FactConfidence::High, 1000)];
        let selected = select_facts_for_prompt(&facts, 0);
        assert!(selected.is_empty());
    }

    #[tokio::test]
    async fn append_and_read_facts_round_trip() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let facts = vec![
            Fact {
                schema_version: 1,
                id: "id1".to_string(),
                text: "First fact".to_string(),
                provenance: FactProvenance {
                    session_id: "s1".to_string(),
                    turn_id: "t1".to_string(),
                },
                confidence: FactConfidence::High,
                scope: FactScope::Workspace,
                created_at: 1000,
            },
            Fact {
                schema_version: 1,
                id: "id2".to_string(),
                text: "Second fact".to_string(),
                provenance: FactProvenance {
                    session_id: "s1".to_string(),
                    turn_id: "t2".to_string(),
                },
                confidence: FactConfidence::Med,
                scope: FactScope::Global,
                created_at: 2000,
            },
        ];

        append_facts(&temp_dir, &facts).await.unwrap();

        let read_back = read_facts(&temp_dir).await.unwrap();
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].id, "id1");
        assert_eq!(read_back[1].id, "id2");

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn read_facts_skips_damaged_lines() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let facts_path = temp_dir.join(FACTS_FILE_NAME);
        tokio::fs::write(
            &facts_path,
            r#"{"schema_version":1,"id":"good1","text":"OK","provenance":{"session_id":"s1","turn_id":"t1"},"confidence":"high","scope":"workspace","created_at":1000}
DAMAGED LINE HERE
{"schema_version":1,"id":"good2","text":"Also OK","provenance":{"session_id":"s1","turn_id":"t2"},"confidence":"med","scope":"global","created_at":2000}"#,
        )
        .await
        .unwrap();

        let facts = read_facts(&temp_dir).await.unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].id, "good1");
        assert_eq!(facts[1].id, "good2");

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn append_facts_is_append_only() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let fact1 = vec![Fact {
            schema_version: 1,
            id: "id1".to_string(),
            text: "First".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::High,
            scope: FactScope::Workspace,
            created_at: 1000,
        }];

        append_facts(&temp_dir, &fact1).await.unwrap();

        let fact2 = vec![Fact {
            schema_version: 1,
            id: "id2".to_string(),
            text: "Second".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t2".to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Global,
            created_at: 2000,
        }];

        append_facts(&temp_dir, &fact2).await.unwrap();

        let facts = read_facts(&temp_dir).await.unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].id, "id1");
        assert_eq!(facts[1].id, "id2");

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn read_facts_nonexistent_file_returns_empty() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let facts = read_facts(&temp_dir).await.unwrap();
        assert!(facts.is_empty());

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[test]
    fn distill_facts_with_keyword_remember() {
        let user_input = "please remember that I prefer pnpm from now on";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert_eq!(facts.len(), 1);
        assert!(facts[0].text.contains("prefer pnpm"));
        assert_eq!(facts[0].confidence, FactConfidence::Med);
        assert_eq!(facts[0].scope, FactScope::Workspace);
    }

    #[test]
    fn distill_facts_with_keyword_always() {
        let user_input = "always use pnpm for this project";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert_eq!(facts.len(), 1);
        assert!(facts[0].text.contains("always use pnpm"));
    }

    #[test]
    fn distill_facts_with_keyword_chinese() {
        let user_input = "以后都用pnpm来管理依赖";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert_eq!(facts.len(), 1);
        assert!(facts[0].text.contains("以后都用pnpm"));
    }

    #[test]
    fn distill_facts_no_keyword_returns_empty() {
        let user_input = "Hello, how are you today?";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert!(facts.is_empty());
    }

    #[test]
    fn distill_facts_truncates_long_sentence() {
        // Create a single sentence exceeding 300 chars with keyword "remember"
        let user_input = "please remember that I prefer pnpm for all my projects and this is a very long sentence that should be truncated at around three hundred characters to fit within the limit for structured memory storage in the agent system and we need to make sure it actually exceeds three hundred characters before truncation happens";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].text.chars().count(), 300);
    }

    #[test]
    fn distill_facts_multiple_sentences_with_keyword() {
        let user_input = "Hello there. please remember I prefer pnpm. How is the weather? also always run tests before commit";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        // Should get 2 facts: one for "prefer pnpm" and one for "always run tests"
        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn distill_facts_with_cjk_period() {
        let user_input = "以后都用pnpm来管理依赖";
        let facts = distill_facts_from_user_message(user_input, "s1", "t1");

        assert_eq!(facts.len(), 1);
    }

    // --- Token estimation ceiling division tests ---

    fn fact_with_text(text: &str, created_at: u64) -> Fact {
        Fact {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            text: text.to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::High,
            scope: FactScope::Workspace,
            created_at,
        }
    }

    #[test]
    fn token_estimation_ceiling_division() {
        // (chars + 3) / 4 gives ceiling division
        // 0 chars -> 0 tokens
        assert_eq!(fact_with_text("", 1).estimated_tokens(), 0);
        // 1 char -> ceil(1/4) = 1
        assert_eq!(fact_with_text("a", 1).estimated_tokens(), 1);
        // 3 chars -> ceil(3/4) = 1
        assert_eq!(fact_with_text("abc", 1).estimated_tokens(), 1);
        // 4 chars -> ceil(4/4) = 1
        assert_eq!(fact_with_text("abcd", 1).estimated_tokens(), 1);
        // 5 chars -> ceil(5/4) = 2
        assert_eq!(fact_with_text("abcde", 1).estimated_tokens(), 2);
        // 8 chars -> ceil(8/4) = 2
        assert_eq!(fact_with_text("abcdefgh", 1).estimated_tokens(), 2);
        // 13 chars -> ceil(13/4) = 4
        assert_eq!(fact_with_text("abcdefghijklm", 1).estimated_tokens(), 4);
    }

    #[test]
    fn select_facts_budget_exactly_full() {
        // 4 chars = 1 token, budget 1 should include it
        let facts = vec![fact_with_text("abcd", 1000)];
        let selected = select_facts_for_prompt(&facts, 1);
        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn select_facts_budget_zero_excludes_all() {
        let facts = vec![fact_with_text("abcd", 1000)];
        let selected = select_facts_for_prompt(&facts, 0);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_facts_short_text_exact_budget() {
        // 1 char = 1 token, budget 1 should include it
        let facts = vec![fact_with_text("a", 1000)];
        let selected = select_facts_for_prompt(&facts, 1);
        assert_eq!(selected.len(), 1);
    }

    // --- Serde output contract tests ---

    #[test]
    fn serde_confidence_serializes_to_lowercase() {
        assert_eq!(serde_json::to_string(&FactConfidence::High).unwrap(), "\"high\"");
        assert_eq!(serde_json::to_string(&FactConfidence::Med).unwrap(), "\"med\"");
        assert_eq!(serde_json::to_string(&FactConfidence::Low).unwrap(), "\"low\"");
    }

    #[test]
    fn serde_scope_serializes_to_lowercase() {
        assert_eq!(serde_json::to_string(&FactScope::Global).unwrap(), "\"global\"");
        assert_eq!(serde_json::to_string(&FactScope::Workspace).unwrap(), "\"workspace\"");
    }

    #[test]
    fn serde_fact_round_trip() {
        let fact = Fact {
            schema_version: 1,
            id: "test-id".to_string(),
            text: "test fact".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Workspace,
            created_at: 1234567890,
        };
        let json = serde_json::to_string(&fact).unwrap();
        let parsed: Fact = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-id");
        assert_eq!(parsed.confidence, FactConfidence::Med);
        assert_eq!(parsed.scope, FactScope::Workspace);
    }

    // --- Deduplication tests ---

    #[tokio::test]
    async fn deduplication_existing_fact_prevents_append() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let existing = Fact {
            schema_version: 1,
            id: "id1".to_string(),
            text: "I prefer pnpm".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::High,
            scope: FactScope::Workspace,
            created_at: 1000,
        };

        // Write existing fact
        append_facts(&temp_dir, &[existing]).await.unwrap();
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 1);

        // Try to append duplicate
        let dup = Fact {
            schema_version: 1,
            id: "id2".to_string(),
            text: "I prefer pnpm".to_string(), // same text
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t2".to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Workspace,
            created_at: 2000,
        };

        // Dedupe at call site (distill) — simulate by filtering
        let existing_facts = read_facts(&temp_dir).await.unwrap();
        let existing_texts: std::collections::HashSet<String> =
            existing_facts.iter().map(|f| f.text.clone()).collect();
        let new_facts: Vec<_> = vec![dup].into_iter().filter(|c| !existing_texts.contains(&c.text)).collect();
        assert!(new_facts.is_empty());

        // Original file still has only 1 fact
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 1);

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn deduplication_batch_internal_prevents_duplicates() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        // Empty existing facts
        assert!(read_facts(&temp_dir).await.unwrap().is_empty());

        // Two facts with same text (simulating same sentence hit twice)
        let fact1 = Fact {
            schema_version: 1,
            id: "id1".to_string(),
            text: "I prefer pnpm".to_string(),
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Workspace,
            created_at: 1000,
        };
        let fact2 = Fact {
            schema_version: 1,
            id: "id2".to_string(),
            text: "I prefer pnpm".to_string(), // same text
            provenance: FactProvenance {
                session_id: "s1".to_string(),
                turn_id: "t1".to_string(),
            },
            confidence: FactConfidence::Med,
            scope: FactScope::Workspace,
            created_at: 1001,
        };

        // Dedupe using HashSet::insert (unified batch + history dedup)
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let unique: Vec<_> = vec![fact1, fact2]
            .into_iter()
            .filter(|f| seen.insert(f.text.clone()))
            .collect();

        // Only one should survive
        assert_eq!(unique.len(), 1);

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    // --- Read facts IO error test ---

    #[tokio::test]
    async fn read_facts_missing_file_returns_empty_not_error() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        // Don't create the directory - file doesn't exist
        let result = read_facts(&temp_dir).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // --- append_facts_dedup tests (real production path) ---

    #[tokio::test]
    async fn append_facts_dedup_skips_existing_identical_text() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let existing = distill_facts_from_user_message("以后都用 pnpm", "s1", "t1");
        assert_eq!(existing.len(), 1);
        let appended_first = append_facts_dedup(&temp_dir, existing).await;
        assert_eq!(appended_first, 1);
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 1);

        // Same text again — production dedup must skip it.
        let again = distill_facts_from_user_message("以后都用 pnpm", "s1", "t2");
        let appended_second = append_facts_dedup(&temp_dir, again).await;
        assert_eq!(appended_second, 0);
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 1);

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn append_facts_dedup_batch_internal_duplicates_write_once() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        // Two identical candidates in one batch → only one written.
        let batch = vec![
            distill_facts_from_user_message("以后都用 pnpm", "s1", "t1"),
            distill_facts_from_user_message("以后都用 pnpm", "s1", "t1"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        assert_eq!(batch.len(), 2);

        let appended = append_facts_dedup(&temp_dir, batch).await;
        assert_eq!(appended, 1);
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 1);

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn append_facts_dedup_appends_distinct_facts() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let batch = vec![
            distill_facts_from_user_message("以后都用 pnpm", "s1", "t1"),
            distill_facts_from_user_message("不要总是提交锁文件", "s1", "t1"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let appended = append_facts_dedup(&temp_dir, batch).await;
        assert_eq!(appended, 2);
        assert_eq!(read_facts(&temp_dir).await.unwrap().len(), 2);

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }
}
