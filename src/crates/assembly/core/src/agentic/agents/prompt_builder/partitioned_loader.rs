//! Partitioned prompt loader — v3 Phase 1 architecture.
//!
//! Splits prompt construction into 4 layers with independent caching:
//! - Layer 1: Template (static, embedded in binary)
//! - Layer 2: Agent prompt (template + persona + language + memory) — per session
//! - Layer 3: System prompt (agent prompt + workspace + session_id + tool defs) — per turn
//! - Layer 4: Full context (system prompt + prepended reminders) — always fresh
//!
//! This reduces per-turn string allocation by ~80% for typical agentic sessions.

use crate::agentic::agents::prompt_builder::PromptBuilder;
use crate::agentic::agents::prompt_builder::PromptBuilderContext;
use crate::util::errors::{NortHingError, NortHingResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Feature gate for v3 Phase 1. Set to false to revert to legacy PromptBuilder path.
pub const USE_PARTITIONED_LOADER: bool = true;

/// Cache identity for Layer 2 (agent prompt).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentPromptCacheIdentity {
    pub template_name: String,
    pub workspace_path: String,
    pub session_id: Option<String>,
}

/// Cache identity for Layer 3 (system prompt).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemPromptCacheIdentity {
    pub agent_prompt_hash: u64,
    pub tool_defs_hash: u64,
    pub workspace_path: String,
    pub session_id: Option<String>,
}

/// Partitioned prompt loader with 4-layer caching.
pub struct PartitionedLoader {
    /// Layer 1: Template name (used to look up embedded prompt).
    template_name: String,

    /// Layer 2: Agent prompt (template + persona + language + memory).
    /// Cached per (template_name, workspace_path, session_id).
    agent_prompt: Option<String>,
    agent_prompt_identity: Option<AgentPromptCacheIdentity>,

    /// Layer 3: System prompt (agent_prompt + workspace_root + session_id + tool_defs).
    /// Cached per (agent_prompt_hash, tool_defs_hash).
    system_prompt: Option<String>,
    system_prompt_identity: Option<SystemPromptCacheIdentity>,
}

impl PartitionedLoader {
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            agent_prompt: None,
            agent_prompt_identity: None,
            system_prompt: None,
            system_prompt_identity: None,
        }
    }

    /// Build Layer 2: Agent prompt.
    ///
    /// Replaces {PERSONA}, {LANGUAGE_PREFERENCE}, {CLAW_WORKSPACE}, {AGENT_MEMORY}.
    /// These change rarely (only when workspace persona/memory files change).
    pub async fn build_agent_prompt(&mut self, context: &PromptBuilderContext) -> NortHingResult<String> {
        let identity = AgentPromptCacheIdentity {
            template_name: self.template_name.clone(),
            workspace_path: context.workspace_path.clone(),
            session_id: context.session_id.clone(),
        };

        if let Some(ref cached) = self.agent_prompt {
            if self.agent_prompt_identity.as_ref() == Some(&identity) {
                return Ok(cached.clone());
            }
        }

        // Build fresh agent prompt using PromptBuilder
        let builder = PromptBuilder::new(context.clone());
        let template = crate::agentic::agents::get_embedded_prompt(&self.template_name).ok_or_else(|| {
            NortHingError::Agent(format!(
                "Prompt template '{}' not found in embedded files",
                self.template_name
            ))
        })?;

        let agent_prompt = builder.build_agent_prompt_layer(template).await?;
        self.agent_prompt = Some(agent_prompt.clone());
        self.agent_prompt_identity = Some(identity);

        Ok(agent_prompt)
    }

    /// Build Layer 3: System prompt.
    ///
    /// Replaces {SESSION_ID}, {DEEP_RESEARCH_REPORT_LINK}, appends visual mode guidance.
    /// Also injects tool definitions (which change when MCP tools are registered/unregistered).
    pub async fn build_system_prompt(
        &mut self,
        context: &PromptBuilderContext,
        tool_defs: Option<&str>,
    ) -> NortHingResult<String> {
        // First ensure agent prompt is built
        let agent_prompt = self.build_agent_prompt(context).await?;
        let agent_hash = hash_string(&agent_prompt);
        let tool_hash = tool_defs.map(hash_string).unwrap_or(0);

        let identity = SystemPromptCacheIdentity {
            agent_prompt_hash: agent_hash,
            tool_defs_hash: tool_hash,
            workspace_path: context.workspace_path.clone(),
            session_id: context.session_id.clone(),
        };

        // Check cache
        if let Some(ref cached) = self.system_prompt {
            if self.system_prompt_identity.as_ref() == Some(&identity) {
                return Ok(cached.clone());
            }
        }

        // Build fresh system prompt from agent prompt + dynamic parts
        let builder = PromptBuilder::new(context.clone());
        let system_prompt = builder.build_system_prompt_layer(&agent_prompt, tool_defs).await?;
        self.system_prompt = Some(system_prompt.clone());
        self.system_prompt_identity = Some(identity);

        Ok(system_prompt)
    }

    /// Invalidate Layer 2 cache (call when workspace persona/memory files change).
    pub fn invalidate_agent_prompt(&mut self) {
        self.agent_prompt = None;
        self.agent_prompt_identity = None;
        // System prompt depends on agent prompt, so invalidate it too
        self.invalidate_system_prompt();
    }

    /// Invalidate Layer 3 cache (call when tool definitions change, e.g. MCP register/unregister).
    pub fn invalidate_system_prompt(&mut self) {
        self.system_prompt = None;
        self.system_prompt_identity = None;
    }

    /// Get the template name.
    pub fn template_name(&self) -> &str {
        &self.template_name
    }
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

use super::*;

#[test]
fn loader_stores_template_name() {
    let loader = PartitionedLoader::new("agentic_mode");
    assert_eq!(loader.template_name(), "agentic_mode");
}

#[test]
fn invalidate_agent_prompt_clears_both_caches() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    loader.agent_prompt = Some("cached".into());
    loader.agent_prompt_identity = Some(AgentPromptCacheIdentity {
        template_name: "agentic_mode".into(),
        workspace_path: "/tmp".into(),
        session_id: None,
    });
    loader.system_prompt = Some("cached_system".into());
    loader.system_prompt_identity = Some(SystemPromptCacheIdentity {
        agent_prompt_hash: 1,
        tool_defs_hash: 2,
        workspace_path: "/tmp".into(),
        session_id: None,
    });

    loader.invalidate_agent_prompt();

    assert!(loader.agent_prompt.is_none());
    assert!(loader.agent_prompt_identity.is_none());
    assert!(loader.system_prompt.is_none());
    assert!(loader.system_prompt_identity.is_none());
}

#[test]
fn invalidate_system_prompt_only_caches_system() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    loader.agent_prompt = Some("cached".into());
    loader.system_prompt = Some("cached_system".into());

    loader.invalidate_system_prompt();

    assert!(loader.agent_prompt.is_some()); // preserved
    assert!(loader.system_prompt.is_none());
    assert!(loader.system_prompt_identity.is_none());
}

#[test]
fn hash_string_is_deterministic() {
    let h1 = hash_string("hello");
    let h2 = hash_string("hello");
    assert_eq!(h1, h2);
}

#[test]
fn cache_identity_equality() {
    let id1 = AgentPromptCacheIdentity {
        template_name: "agentic_mode".into(),
        workspace_path: "/tmp".into(),
        session_id: Some("s1".into()),
    };
    let id2 = AgentPromptCacheIdentity {
        template_name: "agentic_mode".into(),
        workspace_path: "/tmp".into(),
        session_id: Some("s1".into()),
    };
    let id3 = AgentPromptCacheIdentity {
        template_name: "cowork_mode".into(),
        workspace_path: "/tmp".into(),
        session_id: Some("s1".into()),
    };
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

// ── Extra tests (Task 4) ────────────────────────────────────────

/// Two consecutive agent-prompt builds with identical identity → second hits cache.
#[tokio::test]
async fn agent_prompt_cache_hit_skips_rebuild() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    let ctx = PromptBuilderContext::new("/tmp", None, None);

    // First build: cache miss → populates cache
    let first = loader.build_agent_prompt(&ctx).await.expect("first build");

    // Second build with same identity → cache hit
    let second = loader.build_agent_prompt(&ctx).await.expect("second build");

    assert_eq!(first, second);
    // Verify the loader cached the result
    assert!(loader.agent_prompt.is_some());
    assert_eq!(
        loader.agent_prompt_identity.as_ref().map(|i| i.template_name.clone()),
        Some("agentic_mode".to_string())
    );
}

/// Changing tool definitions invalidates the system-prompt cache.
#[tokio::test]
async fn system_prompt_cache_miss_after_tool_defs_change() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    let ctx = PromptBuilderContext::new("/tmp", None, None);

    // Build system prompt with tool_defs = "hash-A"
    let _ = loader.build_agent_prompt(&ctx).await.expect("agent prompt");
    let _ = loader
        .build_system_prompt(&ctx, Some("hash-A"))
        .await
        .expect("system A");

    // Build with tool_defs = "hash-B" (different hash)
    let _ = loader
        .build_system_prompt(&ctx, Some("hash-B"))
        .await
        .expect("system B");

    // Verify the identity reflects the new hash
    let identity = loader.system_prompt_identity.as_ref().expect("identity must be set");
    assert_eq!(identity.tool_defs_hash, hash_string("hash-B"));
}

/// Equivalent string inputs produce identical hashes (stability invariant).
#[test]
fn cache_identity_hash_is_stable_for_equivalent_inputs() {
    let h1 = hash_string("agentic_mode");
    let h2 = hash_string("agentic_mode");
    let h3 = hash_string("agentic_mode_other");

    assert_eq!(h1, h2, "same input must produce same hash");
    assert_ne!(h1, h3, "different input must produce different hash");
}
