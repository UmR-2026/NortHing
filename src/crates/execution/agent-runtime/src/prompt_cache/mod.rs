//! Prompt-cache owner decisions.
//!
//! Facade for the prompt-cache owner. The original 873-line
//! `prompt_cache.rs` has been split into four sibling modules that each
//! own a focused sub-domain:
//!
//! - [`cache_types`]   — pure data types: constants, identities, cached
//!                       prompt value objects, per-session bucket struct,
//!                       invalidation scope enum, and the shared
//!                       `current_time_ms` helper.
//! - [`cache_stats`]   — observability counters ([`PromptCacheStats`])
//!                       and the one-shot [`CacheEffectivenessReport`]
//!                       snapshot type.
//! - [`cache_store`]   — [`SessionPromptCacheStore`] struct, the
//!                       [`PromptCacheLookup`] result enum, and the
//!                       lifecycle / write / invalidation methods.
//! - [`cache_query`]   — read-path [`SessionPromptCacheStore::lookup_system_prompt`]
//!                       and [`SessionPromptCacheStore::lookup_user_context`]
//!                       inherent impls (kept separate from the write path
//!                       so the store file stays focused on lifecycle).
//!
//! All public items from the four siblings are re-exported below via
//! wildcard `pub use` so the existing
//! `northhing_agent_runtime::prompt_cache::Item` import paths keep
//! working unchanged for downstream consumers (assembly/core
//! re-export at `src/crates/assembly/core/src/agentic/session/prompt_cache.rs`,
//! `tests/prompt_cache_contracts.rs`, etc.).
//!
//! Behaviour, public type surface, and module path are unchanged; only
//! the file layout was reorganised.

mod cache_query;
mod cache_stats;
mod cache_store;
mod cache_types;

pub use cache_stats::*;
pub use cache_store::*;
pub use cache_types::*;

#[cfg(test)]
mod tests {
    use super::{
        CachedSystemPrompt, CachedUserContext, PromptCacheLookup, PromptCachePolicy, PromptCacheScope,
        PromptCacheStats, SessionPromptCacheStore, SystemPromptCacheIdentity, UserContextCacheIdentity,
        DEFAULT_PROMPT_CACHE_PERSISTENCE_TTL,
    };
    use std::time::Duration;

    #[test]
    fn default_prompt_cache_policy_uses_one_day_persistence_ttl() {
        let policy = PromptCachePolicy::default();

        assert_eq!(policy.cache_ttl, None);
        assert_eq!(policy.persistence_ttl, Some(DEFAULT_PROMPT_CACHE_PERSISTENCE_TTL));
    }

    #[test]
    fn system_prompt_cache_requires_matching_identity() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );

        assert_eq!(
            match store.lookup_system_prompt(
                "session-1",
                &SystemPromptCacheIdentity::new("template:agentic_mode"),
                None,
            ) {
                PromptCacheLookup::Hit(value) => Some(value),
                _ => None,
            },
            Some("prompt-a".to_string())
        );
        assert!(matches!(
            store.lookup_system_prompt(
                "session-1",
                &SystemPromptCacheIdentity::new("template:debug_mode"),
                None,
            ),
            PromptCacheLookup::Miss
        ));
    }

    #[test]
    fn expired_user_context_is_evicted_on_read() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_user_context(
            "session-1",
            CachedUserContext::new(
                UserContextCacheIdentity::new("workspace_context|workspace_instructions"),
                "stale context",
            ),
        );

        assert!(matches!(
            store.lookup_user_context(
                "session-1",
                &UserContextCacheIdentity::new("workspace_context|workspace_instructions"),
                Some(Duration::from_millis(0)),
            ),
            PromptCacheLookup::Expired
        ));
        assert!(store
            .get_cache("session-1")
            .expect("session cache")
            .user_context
            .is_none());
    }

    #[test]
    fn invalidate_scope_can_clear_all_cached_prompt_parts() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );
        store.set_user_context(
            "session-1",
            CachedUserContext::new(UserContextCacheIdentity::new("workspace_context"), "context"),
        );

        assert!(store.invalidate("session-1", PromptCacheScope::All));

        let cache = store.get_cache("session-1").expect("session cache");
        assert!(cache.system_prompt.is_none());
        assert!(cache.user_context.is_none());
    }

    #[test]
    fn stats_default_all_zero() {
        let stats = PromptCacheStats::default();
        assert_eq!(stats.system_prompt_hits, 0);
        assert_eq!(stats.system_prompt_misses, 0);
        assert_eq!(stats.system_prompt_expired, 0);
        assert_eq!(stats.user_context_hits, 0);
        assert_eq!(stats.user_context_misses, 0);
        assert_eq!(stats.user_context_expired, 0);
    }

    #[test]
    fn system_prompt_lookup_hit_increments_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );

        assert!(matches!(
            store.lookup_system_prompt(
                "session-1",
                &SystemPromptCacheIdentity::new("template:agentic_mode"),
                None,
            ),
            PromptCacheLookup::Hit(_)
        ));

        let stats = store.stats();
        assert_eq!(stats.system_prompt_hits, 1);
        assert_eq!(stats.system_prompt_misses, 0);
        assert_eq!(stats.system_prompt_expired, 0);
    }

    #[test]
    fn system_prompt_lookup_miss_increments_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");

        assert!(matches!(
            store.lookup_system_prompt(
                "session-1",
                &SystemPromptCacheIdentity::new("template:agentic_mode"),
                None,
            ),
            PromptCacheLookup::Miss
        ));

        let stats = store.stats();
        assert_eq!(stats.system_prompt_hits, 0);
        assert_eq!(stats.system_prompt_misses, 1);
        assert_eq!(stats.system_prompt_expired, 0);
    }

    #[test]
    fn system_prompt_lookup_expired_increments_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );

        assert!(matches!(
            store.lookup_system_prompt(
                "session-1",
                &SystemPromptCacheIdentity::new("template:agentic_mode"),
                Some(Duration::from_millis(0)),
            ),
            PromptCacheLookup::Expired
        ));

        let stats = store.stats();
        assert_eq!(stats.system_prompt_hits, 0);
        assert_eq!(stats.system_prompt_misses, 0);
        assert_eq!(stats.system_prompt_expired, 1);
    }

    #[test]
    fn user_context_lookup_hit_increments_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_user_context(
            "session-1",
            CachedUserContext::new(UserContextCacheIdentity::new("workspace_context"), "context-content"),
        );

        assert!(matches!(
            store.lookup_user_context("session-1", &UserContextCacheIdentity::new("workspace_context"), None,),
            PromptCacheLookup::Hit(_)
        ));

        let stats = store.stats();
        assert_eq!(stats.user_context_hits, 1);
        assert_eq!(stats.user_context_misses, 0);
        assert_eq!(stats.user_context_expired, 0);
    }

    #[test]
    fn user_context_lookup_miss_increments_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");

        assert!(matches!(
            store.lookup_user_context("session-1", &UserContextCacheIdentity::new("workspace_context"), None,),
            PromptCacheLookup::Miss
        ));

        let stats = store.stats();
        assert_eq!(stats.user_context_hits, 0);
        assert_eq!(stats.user_context_misses, 1);
        assert_eq!(stats.user_context_expired, 0);
    }

    #[test]
    fn hit_rate_is_zero_when_no_lookups() {
        let stats = PromptCacheStats::default();
        assert_eq!(stats.system_prompt_total(), 0);
        assert_eq!(stats.user_context_total(), 0);
        assert_eq!(stats.system_prompt_hit_rate(), 0.0);
        assert_eq!(stats.user_context_hit_rate(), 0.0);
    }

    #[test]
    fn hit_rate_reflects_hits_over_total_lookups() {
        let stats = PromptCacheStats {
            system_prompt_hits: 3,
            system_prompt_misses: 1,
            system_prompt_expired: 1,
            user_context_hits: 2,
            user_context_misses: 0,
            user_context_expired: 2,
        };

        assert_eq!(stats.system_prompt_total(), 5);
        assert_eq!(stats.user_context_total(), 4);
        assert!((stats.system_prompt_hit_rate() - 0.6).abs() < 1e-9);
        assert!((stats.user_context_hit_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn prompt_cache_stats_serializes_to_json() {
        use serde_json;

        let stats = PromptCacheStats {
            system_prompt_hits: 3,
            system_prompt_misses: 1,
            system_prompt_expired: 1,
            user_context_hits: 2,
            user_context_misses: 0,
            user_context_expired: 2,
        };

        let json = serde_json::to_string(&stats).expect("serialization must succeed");
        let deserialized: PromptCacheStats = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(deserialized, stats);
    }

    #[test]
    fn combined_hit_rate_is_zero_when_no_lookups() {
        let stats = PromptCacheStats::default();
        assert_eq!(stats.combined_total(), 0);
        assert_eq!(stats.combined_hit_rate(), 0.0);
    }

    #[test]
    fn combined_hit_rate_averages_two_caches() {
        // system: 2 hits / 3 total; user: 1 hit / 1 total
        let stats = PromptCacheStats {
            system_prompt_hits: 2,
            system_prompt_misses: 1,
            system_prompt_expired: 0,
            user_context_hits: 1,
            user_context_misses: 0,
            user_context_expired: 0,
        };
        assert_eq!(stats.combined_total(), 4);
        // (2 + 1) / (3 + 1) = 3/4 = 0.75
        assert!((stats.combined_hit_rate() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn effectiveness_report_reflects_current_stats() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );
        store.set_user_context(
            "session-1",
            CachedUserContext::new(UserContextCacheIdentity::new("workspace_context"), "ctx"),
        );

        // Generate one hit and one miss in each cache.
        let _ = store.lookup_system_prompt(
            "session-1",
            &SystemPromptCacheIdentity::new("template:agentic_mode"),
            None,
        );
        let _ = store.lookup_system_prompt("session-1", &SystemPromptCacheIdentity::new("template:other"), None);
        let _ = store.lookup_user_context("session-1", &UserContextCacheIdentity::new("workspace_context"), None);
        let _ = store.lookup_user_context("session-1", &UserContextCacheIdentity::new("workspace_other"), None);

        let report = store.effectiveness_report();

        // Verify stats match what get_stats() would return
        let direct_stats = store.stats();
        assert_eq!(report.stats, direct_stats);

        // Verify hit rates are computed correctly
        assert!((report.system_prompt_hit_rate - 0.5).abs() < 1e-9); // 1 hit / 2 total
        assert!((report.user_context_hit_rate - 0.5).abs() < 1e-9); // 1 hit / 2 total
        assert!((report.combined_hit_rate - 0.5).abs() < 1e-9); // 2 hits / 4 total

        // captured_at_ms should be a positive value
        assert!(report.captured_at_ms > 0);
    }

    #[test]
    fn effectiveness_report_serializes_to_json() {
        use super::CacheEffectivenessReport;

        let store = SessionPromptCacheStore::new();
        let report = store.effectiveness_report();

        let json = serde_json::to_string(&report).expect("serialization must succeed");
        assert!(json.contains("\"system_prompt_hit_rate\""));
        assert!(json.contains("\"user_context_hit_rate\""));
        assert!(json.contains("\"combined_hit_rate\""));
        assert!(json.contains("\"captured_at_ms\""));

        let deserialized: CacheEffectivenessReport = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(deserialized, report);
    }

    #[test]
    fn effectiveness_report_zero_state() {
        let store = SessionPromptCacheStore::new();
        let report = store.effectiveness_report();

        assert_eq!(report.stats, PromptCacheStats::default());
        assert_eq!(report.system_prompt_hit_rate, 0.0);
        assert_eq!(report.user_context_hit_rate, 0.0);
        assert_eq!(report.combined_hit_rate, 0.0);
        assert!(report.captured_at_ms > 0);
    }

    #[test]
    fn clear_stats_resets_all_counters_to_zero() {
        let store = SessionPromptCacheStore::new();
        store.create_session("session-1");
        store.set_system_prompt(
            "session-1",
            CachedSystemPrompt::new(SystemPromptCacheIdentity::new("template:agentic_mode"), "prompt-a"),
        );
        store.set_user_context(
            "session-1",
            CachedUserContext::new(UserContextCacheIdentity::new("workspace_context"), "ctx"),
        );

        // Generate some hits and misses.
        let _ = store.lookup_system_prompt(
            "session-1",
            &SystemPromptCacheIdentity::new("template:agentic_mode"),
            None,
        );
        let _ = store.lookup_user_context("session-1", &UserContextCacheIdentity::new("workspace_context"), None);
        let _ = store.lookup_system_prompt("session-1", &SystemPromptCacheIdentity::new("template:other"), None);

        let before = store.stats();
        assert_eq!(before.system_prompt_hits, 1);
        assert_eq!(before.system_prompt_misses, 1);
        assert_eq!(before.user_context_hits, 1);

        store.clear_stats();

        let after = store.stats();
        assert_eq!(after, PromptCacheStats::default());
        assert_eq!(after.system_prompt_hit_rate(), 0.0);
        assert_eq!(after.user_context_hit_rate(), 0.0);
    }
}
