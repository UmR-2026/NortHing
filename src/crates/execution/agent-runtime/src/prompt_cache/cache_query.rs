//! Prompt-cache read path: `lookup_system_prompt` and `lookup_user_context`.
//!
//! Both methods are inherent impls on [`super::cache_store::SessionPromptCacheStore`]
//! defined here, in their own sibling module, so that the read path
//! (identity match + TTL check + stats mutation + opportunistic eviction)
//! stays decoupled from the write / lifecycle path in
//! [`super::cache_store`].
//!
//! Sub-domain layout:
//! - `cache_types.rs`   — cached entries, identities, scope enum, expiry helper.
//! - `cache_stats.rs`   — `PromptCacheStats` + `CacheEffectivenessReport`.
//! - `cache_store.rs`   — store struct, write paths, invalidation, stats
//!                        snapshot.
//! - `cache_query.rs`   — read-path lookups (this file).
//!
//! All public items are re-exported from the facade (`crate::prompt_cache`)
//! so existing import paths keep working unchanged.
//!
//! Cross-sibling visibility: this file reaches into `pub(super)` fields
//! of [`super::cache_store::SessionPromptCacheStore`] (`session_caches`
//! and `stats`) to mutate them directly, mirroring the pre-split behaviour
//! without widening the public surface.

use super::cache_store::{PromptCacheLookup, SessionPromptCacheStore};
use super::cache_types::{current_time_ms, PromptCacheScope, SystemPromptCacheIdentity, UserContextCacheIdentity};
use std::time::Duration;

impl SessionPromptCacheStore {
    pub fn lookup_system_prompt(
        &self,
        session_id: &str,
        identity: &SystemPromptCacheIdentity,
        ttl: Option<Duration>,
    ) -> PromptCacheLookup {
        let now_ms = current_time_ms();
        let cached_entry = self
            .session_caches
            .get(session_id)
            .and_then(|cache| cache.system_prompt.clone());

        match cached_entry {
            Some(entry) if entry.is_usable(identity, ttl, now_ms) => {
                self.stats.lock().unwrap().system_prompt_hits += 1;
                PromptCacheLookup::Hit(entry.text.content)
            }
            Some(entry) if entry.text.is_expired(ttl, now_ms) => {
                self.stats.lock().unwrap().system_prompt_expired += 1;
                self.invalidate(session_id, PromptCacheScope::SystemPrompt);
                PromptCacheLookup::Expired
            }
            _ => {
                self.stats.lock().unwrap().system_prompt_misses += 1;
                PromptCacheLookup::Miss
            }
        }
    }

    pub fn lookup_user_context(
        &self,
        session_id: &str,
        identity: &UserContextCacheIdentity,
        ttl: Option<Duration>,
    ) -> PromptCacheLookup {
        let now_ms = current_time_ms();
        let cached_entry = self
            .session_caches
            .get(session_id)
            .and_then(|cache| cache.user_context.clone());

        match cached_entry {
            Some(entry) if entry.is_usable(identity, ttl, now_ms) => {
                self.stats.lock().unwrap().user_context_hits += 1;
                PromptCacheLookup::Hit(entry.text.content)
            }
            Some(entry) if entry.text.is_expired(ttl, now_ms) => {
                self.stats.lock().unwrap().user_context_expired += 1;
                self.invalidate(session_id, PromptCacheScope::UserContext);
                PromptCacheLookup::Expired
            }
            _ => {
                self.stats.lock().unwrap().user_context_misses += 1;
                PromptCacheLookup::Miss
            }
        }
    }
}
