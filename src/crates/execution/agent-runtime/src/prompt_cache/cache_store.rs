//! Prompt-cache store: per-session lifecycle, write paths, and stats snapshot.
//!
//! Owns the [`SessionPromptCacheStore`] struct (DashMap-backed session
//! cache + `Mutex`-backed stats counter), the [`PromptCacheLookup`] result
//! enum, and the lifecycle / write / invalidation methods. Read-path
//! lookups live in [`super::cache_query`] so this file stays focused on
//! "mutate the cache" operations.
//!
//! Sub-domain layout:
//! - `cache_types.rs`   — cached entries, identities, scope enum, expiry helper.
//! - `cache_stats.rs`   — `PromptCacheStats` + `CacheEffectivenessReport`.
//! - `cache_store.rs`   — store struct, write paths, invalidation, stats
//!                        snapshot (this file).
//! - `cache_query.rs`   — read-path `lookup_system_prompt` /
//!                        `lookup_user_context` impls.
//!
//! All public items are re-exported from the facade (`crate::prompt_cache`)
//! so existing import paths keep working unchanged.
//!
//! Cross-sibling visibility: the store's two `Arc`-wrapped fields are
//! `pub(super)` so [`super::cache_query`] can mutate them directly on
//! every lookup without widening the public surface for outside callers.

use super::cache_stats::{CacheEffectivenessReport, PromptCacheStats};
use super::cache_types::{
    current_time_ms, CachedSystemPrompt, CachedUserContext, PromptCacheScope, SessionPromptCache,
};
use dashmap::DashMap;
use std::sync::{Arc, Mutex};

/// Per-session prompt cache store, sharing the same underlying state
/// across clones (Arc-backed maps + stats counter).
pub struct SessionPromptCacheStore {
    /// Per-session cache buckets keyed by `session_id`. Visible to sibling
    /// [`super::cache_query`] only — outside callers must use the store's
    /// methods.
    pub(super) session_caches: Arc<DashMap<String, SessionPromptCache>>,
    /// Shared stats counter incremented on every lookup. Visible to sibling
    /// [`super::cache_query`] only.
    pub(super) stats: Arc<Mutex<PromptCacheStats>>,
}

/// Outcome of a [`SessionPromptCacheStore::lookup_system_prompt`] /
/// [`SessionPromptCacheStore::lookup_user_context`] call.
pub enum PromptCacheLookup {
    /// Cache hit — the supplied content is reusable for this lookup.
    Hit(String),
    /// Cache miss — the identity did not match (or no entry was cached).
    Miss,
    /// Cache hit on the identity, but the entry was past its TTL and has
    /// been evicted as a side effect.
    Expired,
}

impl Default for SessionPromptCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionPromptCacheStore {
    pub fn new() -> Self {
        Self {
            session_caches: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(PromptCacheStats::default())),
        }
    }

    /// Snapshot the current stats counters. Locks briefly; safe to call
    /// from any thread.
    pub fn stats(&self) -> PromptCacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Capture a one-shot snapshot of all cache effectiveness metrics.
    /// Thread-safe (shares the same lock as `get_stats`).
    pub fn effectiveness_report(&self) -> CacheEffectivenessReport {
        let stats = self.stats();
        CacheEffectivenessReport {
            system_prompt_hit_rate: stats.system_prompt_hit_rate(),
            user_context_hit_rate: stats.user_context_hit_rate(),
            combined_hit_rate: stats.combined_hit_rate(),
            stats,
            captured_at_ms: current_time_ms(),
        }
    }

    /// Reset all stats counters to zero.
    pub fn clear_stats(&self) {
        *self.stats.lock().unwrap() = PromptCacheStats::default();
    }

    /// Ensure a session bucket exists, creating an empty one if missing.
    pub fn create_session(&self, session_id: &str) {
        self.session_caches.entry(session_id.to_string()).or_default();
    }

    pub fn has_session(&self, session_id: &str) -> bool {
        self.session_caches.contains_key(session_id)
    }

    /// Overwrite the cache bucket for `session_id` with `cache`.
    pub fn replace_cache(&self, session_id: &str, cache: SessionPromptCache) {
        self.session_caches.insert(session_id.to_string(), cache);
    }

    /// Clone the cache bucket for `session_id`, or return `None` if missing.
    pub fn get_cache(&self, session_id: &str) -> Option<SessionPromptCache> {
        self.session_caches.get(session_id).map(|cache| cache.clone())
    }

    pub fn set_system_prompt(&self, session_id: &str, entry: CachedSystemPrompt) {
        if let Some(mut cache) = self.session_caches.get_mut(session_id) {
            cache.system_prompt = Some(entry);
        } else {
            self.session_caches.insert(
                session_id.to_string(),
                SessionPromptCache {
                    system_prompt: Some(entry),
                    user_context: None,
                },
            );
        }
    }

    pub fn set_user_context(&self, session_id: &str, entry: CachedUserContext) {
        if let Some(mut cache) = self.session_caches.get_mut(session_id) {
            cache.user_context = Some(entry);
        } else {
            self.session_caches.insert(
                session_id.to_string(),
                SessionPromptCache {
                    system_prompt: None,
                    user_context: Some(entry),
                },
            );
        }
    }

    /// Clear the requested slots for `session_id` according to `scope`.
    /// Returns `true` if any slot was actually populated and got cleared.
    pub fn invalidate(&self, session_id: &str, scope: PromptCacheScope) -> bool {
        let Some(mut cache) = self.session_caches.get_mut(session_id) else {
            return false;
        };

        let mut changed = false;
        if scope.clears_system_prompt() && cache.system_prompt.take().is_some() {
            changed = true;
        }
        if scope.clears_user_context() && cache.user_context.take().is_some() {
            changed = true;
        }
        changed
    }

    pub fn delete_session(&self, session_id: &str) {
        self.session_caches.remove(session_id);
    }
}
