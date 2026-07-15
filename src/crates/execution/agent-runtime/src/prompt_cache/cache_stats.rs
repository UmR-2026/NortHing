//! Prompt-cache observability counters and one-shot snapshot types.
//!
//! Pure data shapes that the [`super::cache_store::SessionPromptCacheStore`]
//! mutates on every lookup and exposes for monitoring / serialization.
//! This file owns the "observability" sub-domain; the store-side counter
//! mutation lives in [`super::cache_query`] (for lookup-time hits / misses
//! / expired events) and [`super::cache_store`] (for `clear_stats`).
//!
//! Sub-domain layout:
//! - `cache_types.rs`   — cached entries, identities, scope enum.
//! - `cache_stats.rs`   — `PromptCacheStats` + `CacheEffectivenessReport`
//!                        (this file).
//! - `cache_store.rs`   — store lifecycle and write paths.
//! - `cache_query.rs`   — read-path lookup counter mutation.
//!
//! All public items are re-exported from the facade (`crate::prompt_cache`)
//! so existing import paths keep working unchanged.

use serde::{Deserialize, Serialize};

/// Atomic counters incremented on every [`super::cache_store::SessionPromptCacheStore`]
/// lookup. `hits` count identity- and TTL-valid reuses, `misses` count
/// identity mismatches, and `expired` count TTL-only mismatches.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PromptCacheStats {
    pub system_prompt_hits: u64,
    pub system_prompt_misses: u64,
    pub system_prompt_expired: u64,
    pub user_context_hits: u64,
    pub user_context_misses: u64,
    pub user_context_expired: u64,
}

impl PromptCacheStats {
    /// Total number of system_prompt lookups (hits + misses + expired).
    pub fn system_prompt_total(&self) -> u64 {
        self.system_prompt_hits + self.system_prompt_misses + self.system_prompt_expired
    }

    /// Total number of user_context lookups (hits + misses + expired).
    pub fn user_context_total(&self) -> u64 {
        self.user_context_hits + self.user_context_misses + self.user_context_expired
    }

    /// System-prompt hit rate in [0.0, 1.0]. Returns 0.0 when no lookups happened.
    pub fn system_prompt_hit_rate(&self) -> f64 {
        let total = self.system_prompt_total();
        if total == 0 {
            0.0
        } else {
            self.system_prompt_hits as f64 / total as f64
        }
    }

    /// User-context hit rate in [0.0, 1.0]. Returns 0.0 when no lookups happened.
    pub fn user_context_hit_rate(&self) -> f64 {
        let total = self.user_context_total();
        if total == 0 {
            0.0
        } else {
            self.user_context_hits as f64 / total as f64
        }
    }

    /// Returns combined total of all lookups across both caches.
    pub fn combined_total(&self) -> u64 {
        self.system_prompt_total() + self.user_context_total()
    }

    /// Returns combined hit rate across both caches in [0.0, 1.0].
    /// Returns 0.0 when no lookups have happened.
    pub fn combined_hit_rate(&self) -> f64 {
        let total = self.combined_total();
        if total == 0 {
            0.0
        } else {
            (self.system_prompt_hits + self.user_context_hits) as f64 / total as f64
        }
    }
}

/// A one-shot snapshot of all cache effectiveness metrics at a point in time.
/// Serializable — suitable for sending to monitoring, logging, or persisting to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEffectivenessReport {
    /// Raw counters (hits / misses / expired).
    pub stats: PromptCacheStats,
    /// System-prompt hit rate in [0.0, 1.0].
    pub system_prompt_hit_rate: f64,
    /// User-context hit rate in [0.0, 1.0].
    pub user_context_hit_rate: f64,
    /// Combined hit rate across both caches in [0.0, 1.0].
    pub combined_hit_rate: f64,
    /// Epoch milliseconds when this snapshot was captured.
    pub captured_at_ms: u64,
}
