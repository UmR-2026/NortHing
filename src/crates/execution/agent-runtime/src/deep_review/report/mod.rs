//! Provider-neutral Deep Review report enrichment.
//!
//! This module owns JSON report facts that do not require product session IO,
//! global coordinators, event emitters, or host-specific tool context.

pub mod report_findings;
pub mod report_render;
pub mod report_score;
pub mod report_summary;

pub use report_findings::fill_deep_review_packet_metadata;
pub use report_score::{
    fill_deep_review_cache_update_signals, fill_deep_review_reliability_signals,
    fill_deep_review_runtime_tracker_signal,
};
pub use report_summary::deep_review_cache_from_completed_reviewers;
pub use report_summary::DeepReviewCacheUpdate;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deep_review::incremental_cache::DeepReviewIncrementalCache;
    use serde_json::json;

    #[test]
    fn runtime_tracker_signal_adds_concurrency_limited_warning_once() {
        let mut input = json!({
            "summary": {
                "overall_assessment": "Review completed"
            }
        });

        fill_deep_review_runtime_tracker_signal(&mut input, 3);
        fill_deep_review_runtime_tracker_signal(&mut input, 3);

        assert_eq!(
            input["reliability_signals"],
            json!([{
                "kind": "concurrency_limited",
                "severity": "warning",
                "count": 3,
                "source": "runtime"
            }])
        );
    }

    #[test]
    fn runtime_tracker_signal_ignores_zero_count() {
        let mut input = json!({});

        fill_deep_review_runtime_tracker_signal(&mut input, 0);

        assert!(input.get("reliability_signals").is_none());
    }

    #[test]
    fn cache_update_signal_shaping_stays_runtime_owned() {
        let mut input = json!({});
        let cache_update = DeepReviewCacheUpdate {
            value: json!({ "fingerprint": "fp-test", "packets": {} }),
            hit_count: 2,
            miss_count: 1,
        };

        fill_deep_review_cache_update_signals(&mut input, &cache_update);

        assert_eq!(
            input["reliability_signals"],
            json!([
                {
                    "kind": "cache_hit",
                    "severity": "info",
                    "count": 2,
                    "source": "runtime"
                },
                {
                    "kind": "cache_miss",
                    "severity": "info",
                    "count": 1,
                    "source": "runtime"
                }
            ])
        );
    }

    #[test]
    fn incremental_cache_stores_completed_reviewers_by_packet_id() {
        let manifest = json!({
            "incrementalReviewCache": {
                "fingerprint": "fp-review-v2"
            },
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "displayName": "Security Reviewer"
                },
                {
                    "packetId": "reviewer:ReviewPerformance:group-1-of-1",
                    "phase": "reviewer",
                    "subagentId": "ReviewPerformance",
                    "displayName": "Performance Reviewer"
                }
            ]
        });
        let mut input = json!({
            "summary": {
                "overall_assessment": "Review completed",
                "risk_level": "medium",
                "recommended_action": "request_changes"
            },
            "issues": [],
            "positive_points": [],
            "reviewers": [
                {
                    "name": "Security Reviewer",
                    "specialty": "security",
                    "status": "completed",
                    "summary": "Found one high-risk issue."
                },
                {
                    "name": "Performance Reviewer",
                    "specialty": "performance",
                    "status": "partial_timeout",
                    "summary": "Timed out before completion.",
                    "partial_output": "Large render path was still being checked."
                }
            ]
        });
        fill_deep_review_packet_metadata(&mut input, Some(&manifest));

        let cache_update = deep_review_cache_from_completed_reviewers(&input, Some(&manifest), None)
            .expect("completed reviewer should produce cache value");
        let cache = DeepReviewIncrementalCache::from_value(&cache_update.value);

        assert_eq!(cache.fingerprint(), "fp-review-v2");
        assert_eq!(cache_update.hit_count, 0);
        assert_eq!(cache_update.miss_count, 1);
        assert!(cache
            .get_packet("reviewer:ReviewSecurity:group-1-of-1")
            .is_some_and(|output| output.contains("Found one high-risk issue.")));
        assert_eq!(cache.get_packet("reviewer:ReviewPerformance:group-1-of-1"), None);
    }

    #[test]
    fn incremental_cache_replaces_stale_existing_cache() {
        let manifest = json!({
            "incrementalReviewCache": {
                "fingerprint": "fp-new"
            },
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "displayName": "Security Reviewer"
                }
            ]
        });
        let mut stale_cache = DeepReviewIncrementalCache::new("fp-old");
        stale_cache.store_packet("reviewer:ReviewSecurity", "stale output");
        let mut input = json!({
            "summary": {
                "overall_assessment": "Review completed",
                "risk_level": "low",
                "recommended_action": "approve"
            },
            "issues": [],
            "positive_points": [],
            "reviewers": [
                {
                    "name": "Security Reviewer",
                    "specialty": "security",
                    "status": "completed",
                    "summary": "Fresh security output."
                }
            ]
        });
        fill_deep_review_packet_metadata(&mut input, Some(&manifest));

        let cache_update =
            deep_review_cache_from_completed_reviewers(&input, Some(&manifest), Some(&stale_cache.to_value()))
                .expect("completed reviewer should replace stale cache");
        let cache = DeepReviewIncrementalCache::from_value(&cache_update.value);

        assert_eq!(cache.fingerprint(), "fp-new");
        assert_eq!(cache_update.hit_count, 0);
        assert_eq!(cache_update.miss_count, 1);
        assert!(cache
            .get_packet("reviewer:ReviewSecurity")
            .is_some_and(|output| output.contains("Fresh security output.")));
        assert!(!cache
            .get_packet("reviewer:ReviewSecurity")
            .is_some_and(|output| output.contains("stale output")));
    }

    #[test]
    fn incremental_cache_counts_existing_packet_hits() {
        let manifest = json!({
            "incrementalReviewCache": {
                "fingerprint": "fp-existing"
            },
            "workPackets": [
                {
                    "packetId": "reviewer:ReviewSecurity",
                    "phase": "reviewer",
                    "subagentId": "ReviewSecurity",
                    "displayName": "Security Reviewer"
                },
                {
                    "packetId": "reviewer:ReviewPerformance",
                    "phase": "reviewer",
                    "subagentId": "ReviewPerformance",
                    "displayName": "Performance Reviewer"
                }
            ]
        });
        let mut existing_cache = DeepReviewIncrementalCache::new("fp-existing");
        existing_cache.store_packet("reviewer:ReviewSecurity", "cached security output");
        let mut input = json!({
            "summary": {
                "overall_assessment": "Review completed",
                "risk_level": "medium",
                "recommended_action": "request_changes"
            },
            "issues": [],
            "positive_points": [],
            "reviewers": [
                {
                    "name": "Security Reviewer",
                    "specialty": "security",
                    "status": "completed",
                    "summary": "Reused security output."
                },
                {
                    "name": "Performance Reviewer",
                    "specialty": "performance",
                    "status": "completed",
                    "summary": "Fresh performance output."
                }
            ]
        });
        fill_deep_review_packet_metadata(&mut input, Some(&manifest));

        let cache_update =
            deep_review_cache_from_completed_reviewers(&input, Some(&manifest), Some(&existing_cache.to_value()))
                .expect("completed reviewers should update cache");

        assert_eq!(cache_update.hit_count, 1);
        assert_eq!(cache_update.miss_count, 1);
    }
}
