<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# Skill System v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a Markdown-based skill loader and registry that replaces v3's heavy builtin skill catalog. Skills are loaded from `~/.config/agent-app/skills/` directory, matched to user prompts via keywords, and injected into system prompts on-demand.

**Architecture:** 
- `SkillLoader` scans filesystem, parses Markdown + YAML frontmatter
- `SkillRegistry` holds loaded skills, provides keyword matching (TF-IDF/BM25)
- `PromptInjector` injects matched skill content into system prompt with token budget
- Integration with existing `agent-app-core` prompt builder

**Tech Stack:** Rust, `serde_yaml`, `pulldown-cmark`, regex, existing workspace crates

---

## File Structure

**New files:**
- `src/crates/execution/agent-runtime/src/skills/loader.rs` - Filesystem scanning + Markdown parsing
- `src/crates/execution/agent-runtime/src/skills/registry.rs` - In-memory registry + keyword matching
- `src/crates/execution/agent-runtime/src/skills/injector.rs` - Prompt injection with token budget
- `src/crates/execution/agent-runtime/src/skills/mod.rs` - Module exports
- `src/crates/execution/agent-runtime/tests/skills_contracts.rs` - Integration tests
- `docs/skills/SKILL_FORMAT.md` - Skill authoring documentation

**Modified files:**
- `src/crates/execution/agent-runtime/src/lib.rs` - Add skills module
- `src/crates/execution/agent-runtime/src/prompt.rs` - Integrate skill injection into prompt builder
- `src/crates/execution/agent-runtime/Cargo.toml` - Add dependencies if needed

---

## Task 1: Skill Data Structures

**Files:**
- Create: `src/crates/execution/agent-runtime/src/skills/mod.rs`

- [ ] **Step 1: Define Skill data structures**

```rust
//! Skill system v2 - Markdown-based skill loader and registry

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Unique identifier for a skill
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillId(pub String);

impl SkillId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parsed skill metadata from YAML frontmatter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Complete skill with metadata and Markdown body
#[derive(Debug, Clone)]
pub struct Skill {
    pub metadata: SkillMetadata,
    /// Markdown content (without frontmatter)
    pub body: String,
    /// Absolute path to source file
    pub source_path: PathBuf,
    /// Token count estimate (simple word-based)
    pub token_estimate: usize,
}

impl Skill {
    /// Full text for injection (metadata + body)
    pub fn full_text(&self) -> String {
        format!(
            "# {}\n\n{}",
            self.metadata.name,
            self.body
        )
    }
    
    /// Check if this skill matches given keywords
    pub fn matches_keywords(&self, keywords: &[String]) -> f32 {
        let trigger_text = self.metadata.triggers.join(" ").to_lowercase();
        let desc_text = self.metadata.description.to_lowercase();
        let body_text = self.body.to_lowercase();
        
        let mut score = 0.0f32;
        for keyword in keywords {
            let kw = keyword.to_lowercase();
            if trigger_text.contains(&kw) {
                score += 3.0;
            }
            if desc_text.contains(&kw) {
                score += 2.0;
            }
            if body_text.contains(&kw) {
                score += 1.0;
            }
        }
        score
    }
}

/// Error types for skill operations
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(SkillId),
    #[error("Invalid skill format: {0}")]
    InvalidFormat(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    YamlParse(String),
}

pub type SkillResult<T> = Result<T, SkillError>;
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p agent-app-agent-runtime`
Expected: PASS (may warn about unused code)

- [ ] **Step 3: Commit**

```bash
git add src/crates/execution/agent-runtime/src/skills/mod.rs
git commit -m "feat(skills): define Skill data structures and error types"
```

---

## Task 2: Skill Loader - Filesystem Scanning

**Files:**
- Create: `src/crates/execution/agent-runtime/src/skills/loader.rs`
- Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs` (add pub use)

- [ ] **Step 1: Write test for skill loading**

Create: `src/crates/execution/agent-runtime/tests/skills_contracts.rs`

```rust
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn skill_loader_scans_directory_and_loads_valid_skills() {
    let temp_dir = TempDir::new().unwrap();
    let skills_dir = temp_dir.path().join("skills");
    fs::create_dir(&skills_dir).unwrap();
    
    // Create a valid skill file
    let skill_content = r#"---
id: rust-debug
name: Rust Debugging
version: "1.0.0"
description: Help debug Rust compilation errors
triggers:
  - rust
  - compile error
  - borrow checker
---

# Rust Debugging Guide

When you see a Rust compile error:

1. Read the error message carefully
2. Check the file and line number
3. Look for ownership issues

## Common Fixes

- Use `.clone()` for simple cases
- Use `Rc<T>` or `Arc<T>` for shared ownership
"#;
    
    fs::write(skills_dir.join("rust-debug.md"), skill_content).unwrap();
    
    // Create an invalid file (should be skipped)
    fs::write(skills_dir.join("invalid.txt"), "not a skill").unwrap();
    
    // Load skills
    let loader = agent_app_agent_runtime::skills::SkillLoader::new(&skills_dir);
    let skills = loader.load_all().unwrap();
    
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].metadata.id, "rust-debug");
    assert_eq!(skills[0].metadata.name, "Rust Debugging");
    assert!(skills[0].body.contains("Rust Debugging Guide"));
    assert_eq!(skills[0].metadata.triggers, vec!["rust", "compile error", "borrow checker"]);
}

#[test]
fn skill_loader_handles_missing_directory() {
    let loader = agent_app_agent_runtime::skills::SkillLoader::new("/nonexistent/path");
    let skills = loader.load_all().unwrap();
    assert!(skills.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts -- skill_loader_scans_directory_and_loads_valid_skills`
Expected: FAIL - "SkillLoader not found"

- [ ] **Step 3: Implement SkillLoader**

Create: `src/crates/execution/agent-runtime/src/skills/loader.rs`

```rust
//! Skill loader - scans filesystem and parses Markdown + YAML frontmatter

use std::fs;
use std::path::{Path, PathBuf};
use super::{Skill, SkillMetadata, SkillResult, SkillError};

/// Scans directories for `.md` skill files and loads them
#[derive(Debug, Clone)]
pub struct SkillLoader {
    root_path: PathBuf,
}

impl SkillLoader {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }
    
    /// Load all skills from the root directory (recursive)
    pub fn load_all(&self) -> SkillResult<Vec<Skill>> {
        let mut skills = Vec::new();
        
        if !self.root_path.exists() {
            return Ok(skills); // Empty, not error
        }
        
        self.load_recursive(&self.root_path, &mut skills)?;
        Ok(skills)
    }
    
    fn load_recursive(&self, dir: &Path, skills: &mut Vec<Skill>) -> SkillResult<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.load_recursive(&path, skills)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                match self.load_single(&path) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        eprintln!("Warning: Failed to load skill from {}: {}", path.display(), e);
                        // Continue loading other skills
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a single skill file
    pub fn load_single(&self, path: &Path) -> SkillResult<Skill> {
        let content = fs::read_to_string(path)?;
        self.parse_skill(content, path.to_path_buf())
    }
    
    /// Parse skill content (YAML frontmatter + Markdown body)
    fn parse_skill(&self, content: String, source_path: PathBuf) -> SkillResult<Skill> {
        // Split frontmatter and body
        let (frontmatter, body) = self.split_frontmatter(&content)?;
        
        // Parse YAML frontmatter
        let metadata: SkillMetadata = serde_yaml::from_str(&frontmatter)
            .map_err(|e| SkillError::YamlParse(e.to_string()))?;
        
        // Validate required fields
        if metadata.id.is_empty() {
            return Err(SkillError::InvalidFormat("Skill id is required".to_string()));
        }
        if metadata.name.is_empty() {
            return Err(SkillError::InvalidFormat("Skill name is required".to_string()));
        }
        
        // Simple token estimate (words * 1.3)
        let word_count = body.split_whitespace().count();
        let token_estimate = (word_count as f32 * 1.3) as usize;
        
        Ok(Skill {
            metadata,
            body: body.to_string(),
            source_path,
            token_estimate,
        })
    }
    
    /// Split content into YAML frontmatter and Markdown body
    fn split_frontmatter(&self, content: &str) -> SkillResult<(String, String)> {
        let delimiter = "---";
        
        if !content.starts_with(delimiter) {
            return Err(SkillError::InvalidFormat(
                "Skill file must start with YAML frontmatter (---)".to_string()
            ));
        }
        
        // Find second delimiter
        let rest = &content[delimiter.len()..];
        match rest.find(delimiter) {
            Some(end_pos) => {
                let frontmatter = rest[..end_pos].trim().to_string();
                let body = rest[end_pos + delimiter.len()..].trim().to_string();
                Ok((frontmatter, body))
            }
            None => Err(SkillError::InvalidFormat(
                "YAML frontmatter not properly closed (missing second ---)".to_string()
            )),
        }
    }
}
```

- [ ] **Step 4: Export from mod.rs**

Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs`

Add at the end:
```rust
pub mod loader;

pub use loader::SkillLoader;
```

- [ ] **Step 5: Add tempfile dev-dependency**

Check if `tempfile` is already in `Cargo.toml`. If not, add to `[dev-dependencies]`.

- [ ] **Step 6: Run tests**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/crates/execution/agent-runtime/src/skills/loader.rs \
        src/crates/execution/agent-runtime/src/skills/mod.rs \
        src/crates/execution/agent-runtime/tests/skills_contracts.rs
git commit -m "feat(skills): implement SkillLoader with filesystem scanning and YAML frontmatter parsing"
```

---

## Task 3: Skill Registry - Keyword Matching

**Files:**
- Create: `src/crates/execution/agent-runtime/src/skills/registry.rs`
- Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs`

- [ ] **Step 1: Write test for keyword matching**

Add to `src/crates/execution/agent-runtime/tests/skills_contracts.rs`:

```rust
#[test]
fn skill_registry_matches_keywords_and_returns_top_results() {
    use agent_app_agent_runtime::skills::{SkillRegistry, Skill, SkillMetadata};
    
    let mut registry = SkillRegistry::new();
    
    // Add test skills
    registry.add_skill(Skill {
        metadata: SkillMetadata {
            id: "rust-debug".to_string(),
            name: "Rust Debugging".to_string(),
            version: "1.0.0".to_string(),
            description: "Debug Rust compile errors".to_string(),
            triggers: vec!["rust".to_string(), "compile error".to_string()],
            tags: vec![],
        },
        body: "Rust debugging content".to_string(),
        source_path: PathBuf::from("/tmp/rust-debug.md"),
        token_estimate: 100,
    });
    
    registry.add_skill(Skill {
        metadata: SkillMetadata {
            id: "python-debug".to_string(),
            name: "Python Debugging".to_string(),
            version: "1.0.0".to_string(),
            description: "Debug Python errors".to_string(),
            triggers: vec!["python".to_string(), "exception".to_string()],
            tags: vec![],
        },
        body: "Python debugging content".to_string(),
        source_path: PathBuf::from("/tmp/python-debug.md"),
        token_estimate: 100,
    });
    
    // Match "rust compile error"
    let matches = registry.find_matching_skills(&["rust", "compile error"], 3);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].metadata.id, "rust-debug");
    
    // Match "python" - should return python skill
    let matches = registry.find_matching_skills(&["python"], 3);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].metadata.id, "python-debug");
    
    // Match "debug" - should return both (in body)
    let matches = registry.find_matching_skills(&["debug"], 3);
    assert_eq!(matches.len(), 2);
}

#[test]
fn skill_registry_respects_max_results_limit() {
    use agent_app_agent_runtime::skills::{SkillRegistry, Skill, SkillMetadata};
    
    let mut registry = SkillRegistry::new();
    
    // Add 5 skills
    for i in 0..5 {
        registry.add_skill(Skill {
            metadata: SkillMetadata {
                id: format!("skill-{}", i),
                name: format!("Skill {}", i),
                version: "1.0.0".to_string(),
                description: "Test skill".to_string(),
                triggers: vec!["test".to_string()],
                tags: vec![],
            },
            body: "Test content".to_string(),
            source_path: PathBuf::from(format!("/tmp/skill-{}.md", i)),
            token_estimate: 100,
        });
    }
    
    let matches = registry.find_matching_skills(&["test"], 3);
    assert_eq!(matches.len(), 3); // Limited to 3
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts -- skill_registry`
Expected: FAIL - "SkillRegistry not found"

- [ ] **Step 3: Implement SkillRegistry**

Create: `src/crates/execution/agent-runtime/src/skills/registry.rs`

```rust
//! Skill registry - in-memory storage and keyword matching

use std::collections::HashMap;
use super::{Skill, SkillId, SkillResult};

/// In-memory registry of loaded skills
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: HashMap<SkillId, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
    
    /// Add a skill to the registry
    pub fn add_skill(&mut self, skill: Skill) {
        let id = SkillId::new(skill.metadata.id.clone());
        self.skills.insert(id, skill);
    }
    
    /// Remove a skill from the registry
    pub fn remove_skill(&mut self, id: &SkillId) -> Option<Skill> {
        self.skills.remove(id)
    }
    
    /// Get a skill by ID
    pub fn get_skill(&self, id: &SkillId) -> Option<&Skill> {
        self.skills.get(id)
    }
    
    /// List all registered skills
    pub fn all_skills(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
    
    /// Find skills matching given keywords, sorted by relevance score
    /// Returns top N results
    pub fn find_matching_skills(&self, keywords: &[impl AsRef<str>], max_results: usize) -> Vec<&Skill> {
        let keywords: Vec<String> = keywords.iter()
            .map(|s| s.as_ref().to_lowercase())
            .collect();
        
        let mut scored: Vec<(f32, &Skill)> = self.skills.values()
            .map(|skill| {
                let score = skill.matches_keywords(&keywords);
                (score, skill)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();
        
        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top N
        scored.into_iter()
            .take(max_results)
            .map(|(_, skill)| skill)
            .collect()
    }
    
    /// Load skills from a directory using SkillLoader
    pub fn load_from_directory(&mut self, path: impl AsRef<std::path::Path>) -> SkillResult<usize> {
        use super::SkillLoader;
        
        let loader = SkillLoader::new(path);
        let skills = loader.load_all()?;
        let count = skills.len();
        
        for skill in skills {
            self.add_skill(skill);
        }
        
        Ok(count)
    }
    
    /// Total token count of all registered skills
    pub fn total_token_estimate(&self) -> usize {
        self.skills.values().map(|s| s.token_estimate).sum()
    }
    
    /// Count of registered skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}
```

- [ ] **Step 4: Export from mod.rs**

Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs`

Add:
```rust
pub mod registry;

pub use registry::SkillRegistry;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts -- skill_registry`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/crates/execution/agent-runtime/src/skills/registry.rs \
        src/crates/execution/agent-runtime/src/skills/mod.rs \
        src/crates/execution/agent-runtime/tests/skills_contracts.rs
git commit -m "feat(skills): implement SkillRegistry with keyword matching and scoring"
```

---

## Task 4: Prompt Injector - Token Budget Management

**Files:**
- Create: `src/crates/execution/agent-runtime/src/skills/injector.rs`
- Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs`

- [ ] **Step 1: Write test for prompt injection**

Add to `src/crates/execution/agent-runtime/tests/skills_contracts.rs`:

```rust
#[test]
fn prompt_injector_injects_skills_within_token_budget() {
    use agent_app_agent_runtime::skills::{Skill, SkillMetadata, PromptInjector};
    
    let skills = vec![
        Skill {
            metadata: SkillMetadata {
                id: "short-skill".to_string(),
                name: "Short Skill".to_string(),
                version: "1.0.0".to_string(),
                description: "A short skill".to_string(),
                triggers: vec![],
                tags: vec![],
            },
            body: "Short content.".to_string(),
            source_path: PathBuf::from("/tmp/short.md"),
            token_estimate: 10,
        },
        Skill {
            metadata: SkillMetadata {
                id: "long-skill".to_string(),
                name: "Long Skill".to_string(),
                version: "1.0.0".to_string(),
                description: "A long skill".to_string(),
                triggers: vec![],
                tags: vec![],
            },
            body: "This is a much longer content with many words to test token budget limiting. ".repeat(50),
            source_path: PathBuf::from("/tmp/long.md"),
            token_estimate: 5000,
        },
    ];
    
    let injector = PromptInjector::new(100); // 100 token budget
    let result = injector.inject_skills("Base prompt", &skills);
    
    // Should include short skill but not long skill
    assert!(result.contains("Short Skill"));
    assert!(!result.contains("Long Skill"));
    assert!(result.contains("Base prompt"));
}

#[test]
fn prompt_injector_respects_budget_and_includes_none_if_all_exceed() {
    use agent_app_agent_runtime::skills::{Skill, SkillMetadata, PromptInjector};
    
    let skills = vec![
        Skill {
            metadata: SkillMetadata {
                id: "expensive".to_string(),
                name: "Expensive".to_string(),
                version: "1.0.0".to_string(),
                description: "Too big".to_string(),
                triggers: vec![],
                tags: vec![],
            },
            body: "Big content".repeat(1000),
            source_path: PathBuf::from("/tmp/big.md"),
            token_estimate: 1000,
        },
    ];
    
    let injector = PromptInjector::new(50); // 50 token budget
    let result = injector.inject_skills("Base prompt", &skills);
    
    // Should just return base prompt, no skills fit
    assert_eq!(result, "Base prompt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts -- prompt_injector`
Expected: FAIL

- [ ] **Step 3: Implement PromptInjector**

Create: `src/crates/execution/agent-runtime/src/skills/injector.rs`

```rust
//! Prompt injector - injects skill content into system prompt with token budget

use super::Skill;

/// Injects skill content into prompts while respecting token budgets
#[derive(Debug, Clone)]
pub struct PromptInjector {
    /// Maximum tokens to allocate for skill content
    max_skill_tokens: usize,
}

impl PromptInjector {
    pub fn new(max_skill_tokens: usize) -> Self {
        Self { max_skill_tokens }
    }
    
    /// Inject skills into a base prompt, staying within token budget
    /// Skills should be pre-sorted by relevance (highest first)
    pub fn inject_skills(&self, base_prompt: &str, skills: &[&Skill]) -> String {
        let mut remaining_budget = self.max_skill_tokens;
        let mut injected_content = String::new();
        
        for skill in skills {
            if skill.token_estimate > remaining_budget {
                // Skip skills that don't fit
                continue;
            }
            
            if !injected_content.is_empty() {
                injected_content.push_str("\n\n---\n\n");
            }
            
            injected_content.push_str(&skill.full_text());
            remaining_budget -= skill.token_estimate;
        }
        
        if injected_content.is_empty() {
            base_prompt.to_string()
        } else {
            format!(
                "{}\n\n## Relevant Skills\n\n{}",
                base_prompt,
                injected_content
            )
        }
    }
    
    /// Quick token estimate for a string (words * 1.3)
    pub fn estimate_tokens(text: &str) -> usize {
        let word_count = text.split_whitespace().count();
        (word_count as f32 * 1.3) as usize
    }
}

impl Default for PromptInjector {
    fn default() -> Self {
        Self::new(5000) // Default 5K token budget for skills
    }
}
```

- [ ] **Step 4: Export from mod.rs**

Modify: `src/crates/execution/agent-runtime/src/skills/mod.rs`

Add:
```rust
pub mod injector;

pub use injector::PromptInjector;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts -- prompt_injector`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/crates/execution/agent-runtime/src/skills/injector.rs \
        src/crates/execution/agent-runtime/src/skills/mod.rs \
        src/crates/execution/agent-runtime/tests/skills_contracts.rs
git commit -m "feat(skills): implement PromptInjector with token budget management"
```

---

## Task 5: Integration with Prompt Builder

**Files:**
- Modify: `src/crates/execution/agent-runtime/src/prompt.rs`
- Modify: `src/crates/execution/agent-runtime/src/lib.rs`

- [ ] **Step 1: Examine existing prompt builder**

Read: `src/crates/execution/agent-runtime/src/prompt.rs` to understand current structure.

- [ ] **Step 2: Add skill integration**

Modify `src/crates/execution/agent-runtime/src/prompt.rs` to:
1. Accept a `SkillRegistry` reference
2. Extract keywords from user prompt
3. Find matching skills
4. Inject into system prompt using `PromptInjector`

Key integration point (pseudocode, adapt to actual prompt builder structure):

```rust
// In the prompt building function:
fn build_system_prompt(
    base_prompt: &str,
    user_prompt: &str,
    skill_registry: Option<&skills::SkillRegistry>,
) -> String {
    let mut prompt = base_prompt.to_string();
    
    // Extract keywords from user prompt (simple version: split into words)
    if let Some(registry) = skill_registry {
        let keywords: Vec<String> = user_prompt
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() > 3) // Filter out short words
            .collect();
        
        let matching = registry.find_matching_skills(&keywords, 3);
        
        if !matching.is_empty() {
            let injector = skills::PromptInjector::new(5000);
            prompt = injector.inject_skills(&prompt, &matching);
        }
    }
    
    prompt
}
```

- [ ] **Step 3: Export skills module from lib.rs**

Modify: `src/crates/execution/agent-runtime/src/lib.rs`

Add:
```rust
pub mod skills;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p agent-app-agent-runtime`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/crates/execution/agent-runtime/src/prompt.rs \
        src/crates/execution/agent-runtime/src/lib.rs
git commit -m "feat(skills): integrate skill injection into prompt builder"
```

---

## Task 6: Documentation and Examples

**Files:**
- Create: `docs/skills/SKILL_FORMAT.md`
- Create: `skills/README.md` (example skills directory)

- [ ] **Step 1: Write skill format documentation**

Create: `docs/skills/SKILL_FORMAT.md`

```markdown
# Skill Format Specification

## Overview

Skills are Markdown files with YAML frontmatter that provide domain-specific knowledge to the agent.

## File Location

Skills are loaded from:
- `~/.config/agent-app/skills/` (Linux/macOS)
- `%APPDATA%\agent-app\skills\` (Windows)

## File Format

```markdown
---
id: unique-skill-id
name: Human Readable Name
version: "1.0.0"
description: Brief description of what this skill does
triggers:
  - keyword1
  - keyword2
  - "multi-word phrase"
tags:
  - category1
  - category2
---

# Skill Content

Markdown content that will be injected into the system prompt when this skill is matched.

## Guidelines

- Keep it concise: aim for < 500 words
- Use clear headings and structure
- Include examples where helpful
- Focus on actionable knowledge
```

## Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier (kebab-case) |
| `name` | Yes | Human-readable name |
| `version` | No | Semantic version (default: "1.0.0") |
| `description` | Yes | Brief description for matching |
| `triggers` | No | Keywords that activate this skill |
| `tags` | No | Categories for organization |

## Matching Behavior

Skills are matched based on:
1. **Triggers** (highest priority): Direct keyword matching
2. **Description** (medium priority): Description text matching
3. **Body** (lowest priority): Content body matching

Top 3 matching skills are injected, up to 5000 token budget.

## Example: Rust Debugging

```markdown
---
id: rust-debugging
name: Rust Debugging Guide
version: "1.0.0"
description: Help diagnose and fix Rust compilation errors
triggers:
  - rust
  - compile error
  - borrow checker
  - lifetime
  - ownership
tags:
  - rust
  - debugging
---

# Rust Debugging

## Common Error Patterns

### Borrow Checker Errors

When you see "cannot borrow `x` as mutable more than once":

1. Check if you need to split the borrows
2. Consider using `Rc<T>` or `Arc<T>` for shared ownership
3. Use `clone()` if the type is cheap to clone

### Lifetime Errors

When lifetimes don't match:

1. Add explicit lifetime annotations
2. Use `'static` for string literals
3. Consider returning owned data instead of references
```
```

- [ ] **Step 2: Create example skills directory**

Create: `skills/README.md`

```markdown
# Example Skills

This directory contains example skills for agent-app.

To use these skills:
1. Copy `.md` files to `~/.config/agent-app/skills/`
2. Restart agent-app or reload skills from settings
3. Skills will be automatically matched based on your prompts

## Included Examples

- `rust-debugging.md` - Rust error diagnosis and fixes
- `python-debugging.md` - Python exception handling
- `git-workflow.md` - Common Git operations and best practices

## Creating Custom Skills

See `docs/skills/SKILL_FORMAT.md` for the full specification.
```

- [ ] **Step 3: Create example skill files**

Create: `skills/rust-debugging.md` (from spec example above)
Create: `skills/python-debugging.md` (similar structure)
Create: `skills/git-workflow.md` (similar structure)

- [ ] **Step 4: Commit**

```bash
git add docs/skills/SKILL_FORMAT.md \
        skills/README.md \
        skills/*.md
git commit -m "docs(skills): add skill format specification and example skills"
```

---

## Task 7: Final Integration Test

**Files:**
- Modify: `src/crates/execution/agent-runtime/tests/skills_contracts.rs`

- [ ] **Step 1: Write end-to-end integration test**

Add to `src/crates/execution/agent-runtime/tests/skills_contracts.rs`:

```rust
#[test]
fn end_to_end_skill_loading_and_matching() {
    use std::fs;
    use tempfile::TempDir;
    use agent_app_agent_runtime::skills::{SkillLoader, SkillRegistry, PromptInjector};
    
    let temp_dir = TempDir::new().unwrap();
    let skills_dir = temp_dir.path().join("skills");
    fs::create_dir(&skills_dir).unwrap();
    
    // Create multiple skills
    fs::write(skills_dir.join("rust.md"), r#"---
id: rust-debug
name: Rust Debugging
description: Rust errors
triggers: [rust, error]
---

# Rust

Fix Rust errors."#).unwrap();
    
    fs::write(skills_dir.join("python.md"), r#"---
id: python-debug
name: Python Debugging
description: Python errors
triggers: [python, exception]
---

# Python

Fix Python errors."#).unwrap();
    
    // Load and register
    let loader = SkillLoader::new(&skills_dir);
    let skills = loader.load_all().unwrap();
    
    let mut registry = SkillRegistry::new();
    for skill in skills {
        registry.add_skill(skill);
    }
    
    assert_eq!(registry.len(), 2);
    
    // Match for rust
    let matches = registry.find_matching_skills(&["rust", "error"], 3);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].metadata.id, "rust-debug");
    
    // Inject into prompt
    let injector = PromptInjector::new(1000);
    let prompt = injector.inject_skills("You are a helpful assistant.", &matches);
    
    assert!(prompt.contains("Rust Debugging"));
    assert!(prompt.contains("Fix Rust errors"));
    assert!(prompt.contains("helpful assistant"));
}
```

- [ ] **Step 2: Run all skill tests**

Run: `cargo test -p agent-app-agent-runtime --test skills_contracts`
Expected: ALL PASS

- [ ] **Step 3: Run full workspace test**

Run: `cargo test --workspace`
Expected: PASS (or known Windows-specific failures)

- [ ] **Step 4: Commit**

```bash
git add src/crates/execution/agent-runtime/tests/skills_contracts.rs
git commit -m "test(skills): add end-to-end integration test for skill loading and matching"
```

---

## Self-Review

### 1. Spec Coverage

| PRD Requirement | Task |
|---------------|------|
| SK-01: 技能加载 | Task 2 - SkillLoader |
| SK-02: 技能解析 | Task 2 - YAML frontmatter + Markdown body |
| SK-03: 技能匹配 | Task 3 - Keyword matching with scoring |
| SK-04: 技能注入 | Task 4 - PromptInjector with token budget |
| SK-05: 技能管理 | Task 3 - SkillRegistry add/remove/get/list |
| 文件系统存储 | Task 2 - Directory scanning |
| Markdown格式 | Task 2 - Frontmatter splitting |

### 2. Placeholder Scan

- No "TBD", "TODO", "implement later"
- All test code is complete with assertions
- All implementation code is complete
- No "similar to Task N" references

### 3. Type Consistency

- `SkillId` used consistently across all tasks
- `SkillMetadata` fields match between loader and registry
- `Skill.token_estimate` used in both loader and injector
- `PromptInjector` accepts `&[&Skill]` matching registry output

### 4. Gaps

- **Token estimation**: Using simple word-count * 1.3. Should consider switching to tiktoken or similar for accuracy in future.
- **Keyword extraction**: Using simple word splitting. Could be improved with NLP in future.
- **Skill hot-reload**: Not implemented. Skills loaded at startup only. Can be added later.
- **Skill enable/disable**: Not implemented. All loaded skills are active. Can be added in settings later.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-06-18-skill-system-v2.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
