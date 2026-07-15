//! Prompt-cache data types, identities, value objects, and scope enum.
//!
//! Pure data shapes and their inherent impls that the prompt-cache owner
//! uses to describe cached prompt entries, per-session buckets, and
//! invalidation scopes. This file owns the "shape" of the prompt-cache
//! API surface; lifecycle, query, and stats behaviour live in the sibling
//! modules.
//!
//! Sub-domain layout:
//! - `cache_types.rs`   — constants, identities, cached-text value objects,
//!                        per-session cache struct, scope enum, and the
//!                        shared `current_time_ms` helper (this file).
//! - `cache_stats.rs`   — `PromptCacheStats` counters + `CacheEffectivenessReport`
//!                        one-shot snapshot.
//! - `cache_store.rs`   — `SessionPromptCacheStore` struct, lifecycle / write
//!                        methods, `PromptCacheLookup` enum, and the
//!                        `invalidate` operation.
//! - `cache_query.rs`   — read-path `lookup_system_prompt` and
//!                        `lookup_user_context` impls on `SessionPromptCacheStore`.
//!
//! All public items are re-exported from the facade (`crate::prompt_cache`)
//! so existing `northhing_agent_runtime::prompt_cache::Item` import paths
//! keep working unchanged for downstream consumers (assembly/core re-export,
//! `tests/prompt_cache_contracts.rs`, etc.).

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Current prompt-cache schema version. Bumped when serialized shapes change.
pub const PROMPT_CACHE_SCHEMA_VERSION: u32 = 1;

/// Default persistence TTL for cached prompt entries (24h).
pub const DEFAULT_PROMPT_CACHE_PERSISTENCE_TTL: Duration = Duration::from_secs(60 * 60 * 24);

/// Policy controlling cache TTL and persistence TTL for cached prompt
/// entries. `cache_ttl = None` means no in-memory expiry; `persistence_ttl
/// = None` means cached entries never expire on load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCachePolicy {
    pub cache_ttl: Option<Duration>,
    pub persistence_ttl: Option<Duration>,
}

impl Default for PromptCachePolicy {
    fn default() -> Self {
        Self {
            cache_ttl: None,
            persistence_ttl: Some(DEFAULT_PROMPT_CACHE_PERSISTENCE_TTL),
        }
    }
}

/// Identity under which a system prompt is cached. Two cached system
/// prompts with the same `scope_key` are considered equivalent and can
/// satisfy each other's lookups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptCacheIdentity {
    pub scope_key: String,
}

impl SystemPromptCacheIdentity {
    pub fn new(scope_key: impl Into<String>) -> Self {
        Self {
            scope_key: scope_key.into(),
        }
    }
}

/// Identity under which a user-context reminder is cached. Same scoping
/// semantics as [`SystemPromptCacheIdentity`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserContextCacheIdentity {
    pub scope_key: String,
}

impl UserContextCacheIdentity {
    pub fn new(scope_key: impl Into<String>) -> Self {
        Self {
            scope_key: scope_key.into(),
        }
    }
}

/// Build the combined scope key used by callers that want to key an
/// external cache by both system-prompt identity and user-context identity.
pub fn prompt_cache_scope_key(
    system_prompt: &SystemPromptCacheIdentity,
    user_context: &UserContextCacheIdentity,
) -> String {
    format!("{}||{}", system_prompt.scope_key, user_context.scope_key)
}

/// Cached prompt body plus the timestamp at which it was captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedPromptText {
    pub content: String,
    pub created_at_ms: u64,
}

impl CachedPromptText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            created_at_ms: current_time_ms(),
        }
    }

    pub fn is_expired(&self, ttl: Option<Duration>, now_ms: u64) -> bool {
        ttl.is_some_and(|ttl| {
            let ttl_ms = ttl.as_millis().try_into().unwrap_or(u64::MAX);
            now_ms.saturating_sub(self.created_at_ms) >= ttl_ms
        })
    }
}

/// Cached system prompt: the rendered text plus the identity under which
/// it was cached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedSystemPrompt {
    #[serde(flatten)]
    pub text: CachedPromptText,
    pub identity: SystemPromptCacheIdentity,
}

impl CachedSystemPrompt {
    pub fn new(identity: SystemPromptCacheIdentity, content: impl Into<String>) -> Self {
        Self {
            text: CachedPromptText::new(content),
            identity,
        }
    }

    pub fn is_usable(&self, identity: &SystemPromptCacheIdentity, ttl: Option<Duration>, now_ms: u64) -> bool {
        self.identity == *identity && !self.text.is_expired(ttl, now_ms)
    }
}

/// Cached user-context reminder: the rendered text plus the identity under
/// which it was cached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedUserContext {
    #[serde(flatten)]
    pub text: CachedPromptText,
    pub identity: UserContextCacheIdentity,
}

impl CachedUserContext {
    pub fn new(identity: UserContextCacheIdentity, content: impl Into<String>) -> Self {
        Self {
            text: CachedPromptText::new(content),
            identity,
        }
    }

    pub fn is_usable(&self, identity: &UserContextCacheIdentity, ttl: Option<Duration>, now_ms: u64) -> bool {
        self.identity == *identity && !self.text.is_expired(ttl, now_ms)
    }
}

/// Per-session cache bucket holding the optional system-prompt and
/// user-context entries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPromptCache {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<CachedSystemPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_context: Option<CachedUserContext>,
}

impl SessionPromptCache {
    /// Drop any cached entry whose `created_at_ms` exceeds the supplied TTL
    /// relative to "now". Returns `true` if any entry was evicted.
    pub fn apply_persistence_ttl(&mut self, ttl: Option<Duration>) -> bool {
        let now_ms = current_time_ms();
        let mut changed = false;

        if self
            .system_prompt
            .as_ref()
            .is_some_and(|entry| entry.text.is_expired(ttl, now_ms))
        {
            self.system_prompt = None;
            changed = true;
        }

        if self
            .user_context
            .as_ref()
            .is_some_and(|entry| entry.text.is_expired(ttl, now_ms))
        {
            self.user_context = None;
            changed = true;
        }

        changed
    }

    pub fn is_empty(&self) -> bool {
        self.system_prompt.is_none() && self.user_context.is_none()
    }
}

/// Tag identifying which sub-caches an [`super::cache_store::SessionPromptCacheStore::invalidate`]
/// call should clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptCacheScope {
    SystemPrompt,
    UserContext,
    All,
}

impl PromptCacheScope {
    /// Whether this scope clears the system-prompt slot. Exposed at
    /// `pub(super)` so the store impl can read it without widening the
    /// public surface.
    pub(super) fn clears_system_prompt(self) -> bool {
        matches!(self, Self::SystemPrompt | Self::All)
    }

    /// Whether this scope clears the user-context slot. Exposed at
    /// `pub(super)` so the store impl can read it without widening the
    /// public surface.
    pub(super) fn clears_user_context(self) -> bool {
        matches!(self, Self::UserContext | Self::All)
    }
}

/// Wall-clock millisecond timestamp used for cache expiry decisions.
/// Visible to sibling modules only (`pub(super)`); external callers use
/// the higher-level [`CachedPromptText::is_expired`] / [`SessionPromptCache::apply_persistence_ttl`]
/// APIs instead.
pub(super) fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
