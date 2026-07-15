// REFERENCE — copied from src/crates/assembly/core/src/agentic/tools/implementations/skills/resolver_v2.rs
// Last synced: 2813b36 (v3-restructure)
// Mirror only — NOT compiled. Original file lives in src/.
// If you change the source, re-run: node scripts/copy_reference.js

//! Skill resolver v2 — on-demand skill selection.
//!
//! Replaces the v3 "render all skills" approach (`render_full_skill_listing_body`)
//! with a keyword-matching resolver that picks the top-K most relevant skills
//! for a given prompt. This cuts the per-turn skill listing from ~12-15K tokens
//! (24 skills) to ~2-5K tokens (top 3-5 skills).
//!
//! Implementation is intentionally simple: bag-of-words keyword overlap between
//! the prompt and each skill's `name` + `description`. No embeddings, no model
//! dependency, sub-millisecond latency.
//!
//! Rollback: set `USE_SKILL_REGISTRY` in `skill_agent_snapshot.rs` to `false`
//! to fall back to the full listing.

use crate::agentic::tools::implementations::skills::SkillInfo;

/// Maximum number of skills returned by `resolve_for_prompt`.
pub const RESOLVED_SKILLS_MAX: usize = 5;

/// Minimum relevance score (0.0-1.0) for a skill to be included in resolved set.
/// Skills below this threshold are filtered out to avoid noise.
pub const MIN_RELEVANCE_SCORE: f64 = 0.05;

/// A skill reference with a computed relevance score.
#[derive(Debug, Clone)]
pub struct ResolvedSkill {
    pub skill: SkillInfo,
    pub score: f64,
}

/// Resolve which skills are most relevant to a given prompt.
///
/// Returns up to [`RESOLVED_SKILLS_MAX`] skills sorted by descending relevance
/// score. Skills scoring below [`MIN_RELEVANCE_SCORE`] are excluded.
///
/// # Algorithm
/// Tokenizes the prompt and each skill's `name` + `description` into lowercase
/// word sets. The score is the Jaccard-like overlap:
/// `|prompt_keywords ∩ skill_keywords| / |skill_keywords|`.
///
/// This is deliberately simple (per the A4 plan: "Don't over-engineer with
/// embeddings"). It runs in O(skills × avg_keywords) — fast enough to run
/// every turn.
pub fn resolve_for_prompt(prompt: &str, skills: &[SkillInfo]) -> Vec<ResolvedSkill> {
    resolve_for_prompt_with_max(prompt, skills, RESOLVED_SKILLS_MAX)
}

/// Like [`resolve_for_prompt`] but with a configurable max result count.
pub fn resolve_for_prompt_with_max(
    prompt: &str,
    skills: &[SkillInfo],
    max_results: usize,
) -> Vec<ResolvedSkill> {
    let prompt_keywords = tokenize(prompt);
    if prompt_keywords.is_empty() || skills.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<ResolvedSkill> = skills
        .iter()
        .filter_map(|skill| {
            let score = score_skill(&prompt_keywords, skill);
            if score >= MIN_RELEVANCE_SCORE {
                Some(ResolvedSkill {
                    skill: skill.clone(),
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending, then by name ascending for stable ordering.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.skill.name.cmp(&b.skill.name))
    });

    scored.truncate(max_results);
    scored
}

/// Score a single skill against a set of prompt keywords.
///
/// Returns a value in [0.0, 1.0] representing the fraction of the skill's
/// keywords that appear in the prompt. Skill keywords are drawn from both
/// `name` and `description`, with name tokens weighted 2x.
fn score_skill(prompt_keywords: &std::collections::HashSet<String>, skill: &SkillInfo) -> f64 {
    let name_keywords = tokenize(&skill.name);
    let desc_keywords = tokenize(&skill.description);

    // Union of skill keywords (name + description).
    let skill_keywords: std::collections::HashSet<&str> = name_keywords
        .iter()
        .chain(desc_keywords.iter())
        .map(String::as_str)
        .collect();

    if skill_keywords.is_empty() {
        return 0.0;
    }

    // Weighted overlap: name matches count double.
    let name_weight = 2.0;
    let desc_weight = 1.0;
    let mut weighted_hits = 0.0;
    let mut weighted_total = 0.0;

    for kw in &name_keywords {
        weighted_total += name_weight;
        if prompt_keywords.contains(kw) {
            weighted_hits += name_weight;
        }
    }
    for kw in &desc_keywords {
        // Don't double-count keywords that are in both name and description.
        if !name_keywords.contains(kw) {
            weighted_total += desc_weight;
            if prompt_keywords.contains(kw) {
                weighted_hits += desc_weight;
            }
        }
    }

    if weighted_total == 0.0 {
        0.0
    } else {
        weighted_hits / weighted_total
    }
}

/// Tokenize a string into lowercase keyword tokens.
///
/// Splits on non-alphanumeric characters and filters out:
/// - Empty tokens
/// - Single-character tokens (too generic)
/// - Common English stop words
fn tokenize(text: &str) -> std::collections::HashSet<String> {
    static STOP_WORDS: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "for", "with", "to", "of", "in", "on", "at", "is",
        "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did",
        "will", "would", "should", "could", "may", "might", "must", "shall", "can", "need", "this",
        "that", "these", "those", "i", "you", "he", "she", "it", "we", "they", "me", "my", "your",
        "his", "her", "its", "our", "their", "what", "which", "who", "when", "where", "why", "how",
        "all", "each", "every", "both", "few", "more", "most", "other", "some", "such", "no", "not",
        "only", "own", "same", "so", "than", "too", "very", "just", "about", "into", "from", "up",
        "out", "if", "then", "there", "here", "now", "please", "help", "want", "get", "make", "use",
        "using", "used", "via", "per", "etc", "via",
    ];

    text.split(|c: char| !c.is_alphanumeric())
        .filter_map(|token| {
            let lower = token.to_lowercase();
            if lower.len() < 2 {
                return None;
            }
            if STOP_WORDS.contains(&lower.as_str()) {
                return None;
            }
            Some(lower)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::tools::implementations::skills::SkillLocation;

    fn make_skill(name: &str, description: &str) -> SkillInfo {
        SkillInfo {
            key: format!("test::{name}"),
            name: name.to_string(),
            description: description.to_string(),
            path: format!("/skills/{name}"),
            level: SkillLocation::User,
            source_slot: "test".to_string(),
            dir_name: name.to_string(),
            is_builtin: false,
            group_key: None,
            is_shadowed: false,
            shadowed_by_key: None,
        }
    }

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Debug my Rust code please");
        assert!(tokens.contains("debug"));
        assert!(tokens.contains("rust"));
        assert!(tokens.contains("code"));
        assert!(!tokens.contains("my")); // stop word
        assert!(!tokens.contains("please")); // stop word
    }

    #[test]
    fn test_resolve_returns_relevant_skill() {
        let skills = vec![
            make_skill("systematic-debugging", "Systematically debug Rust code errors and crashes"),
            make_skill("ppt-design", "Create PowerPoint slide decks and presentations"),
            make_skill("memory", "Long-term memory storage and retrieval"),
        ];

        let resolved = resolve_for_prompt("debug my Rust code crash", &skills);
        assert!(!resolved.is_empty());
        assert_eq!(resolved[0].skill.name, "systematic-debugging");
        // Score is the fraction of skill keywords hit by the prompt. The debug
        // skill description has ~6 unique keywords; "debug", "rust", "code",
        // "crash" all match, so score should be well above zero.
        assert!(resolved[0].score > 0.1, "expected debug skill score > 0.1, got {}", resolved[0].score);
    }

    #[test]
    fn test_resolve_excludes_irrelevant() {
        let skills = vec![
            make_skill("ppt-design", "Create PowerPoint slide decks and presentations"),
            make_skill("xlsx", "Excel spreadsheet generation"),
        ];

        // A prompt about debugging should match neither office skill.
        let resolved = resolve_for_prompt("debug my Rust compilation error", &skills);
        assert!(resolved.is_empty(), "No office skills should match a debug prompt");
    }

    #[test]
    fn test_resolve_ppt_prompt_returns_empty_after_drop() {
        // Per A4 acceptance: PPT skill dropped → "make me a slide deck" returns [].
        // (Simulates the post-A7 state where ppt-design is removed from builtins.)
        let skills = vec![
            make_skill("systematic-debugging", "Debug code"),
            make_skill("memory", "Memory storage"),
        ];

        let resolved = resolve_for_prompt("make me a slide deck", &skills);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_resolve_respects_max_results() {
        let skills: Vec<SkillInfo> = (0..10)
            .map(|i| make_skill(&format!("skill-{i}"), "rust code debug error"))
            .collect();

        let resolved = resolve_for_prompt("rust code debug error", &skills);
        assert!(resolved.len() <= RESOLVED_SKILLS_MAX);
    }

    #[test]
    fn test_resolve_sorted_by_score_descending() {
        let skills = vec![
            make_skill("low-match", "something about rust"),
            make_skill("high-match", "rust rust rust code code debug"),
            make_skill("medium-match", "rust code development"),
        ];

        let resolved = resolve_for_prompt("rust code debug", &skills);
        if resolved.len() >= 2 {
            assert!(resolved[0].score >= resolved[1].score);
        }
    }

    #[test]
    fn test_empty_prompt_returns_empty() {
        let skills = vec![make_skill("test", "test description")];
        let resolved = resolve_for_prompt("", &skills);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_empty_skills_returns_empty() {
        let resolved = resolve_for_prompt("debug code", &[]);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_only_stopwords_prompt_returns_empty() {
        let skills = vec![make_skill("test", "test description")];
        let resolved = resolve_for_prompt("the a an and or", &skills);
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_score_skill_name_weighted_higher() {
        let prompt_keywords: std::collections::HashSet<String> =
            ["debug"].iter().map(|s| s.to_string()).collect();
        let skill_name_match = make_skill("debug", "unrelated words here");
        let skill_desc_match = make_skill("unrelated", "debug unrelated words here");

        let name_score = score_skill(&prompt_keywords, &skill_name_match);
        let desc_score = score_skill(&prompt_keywords, &skill_desc_match);

        assert!(name_score > desc_score, "Name match should score higher than description match");
    }

    #[test]
    fn test_resolve_memory_skill() {
        let skills = vec![
            make_skill("memory", "Store and retrieve long-term memory facts and context"),
            make_skill("ppt-design", "PowerPoint presentations"),
        ];

        let resolved = resolve_for_prompt("remember this fact in memory", &skills);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].skill.name, "memory");
    }
}
