//! Deep Review summary aggregation and cache shaping.
//!
//! Owns the incremental cache update and summary-level facts for the summary
//! section of a Deep Review report.

use super::super::incremental_cache::DeepReviewIncrementalCache;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewCacheUpdate {
    pub value: Value,
    pub hit_count: usize,
    pub miss_count: usize,
}

pub fn deep_review_cache_fingerprint(run_manifest: Option<&Value>) -> Option<String> {
    let manifest = run_manifest?;
    let cache_config = value_for_any_key(manifest, &["incrementalReviewCache", "incremental_review_cache"])?;
    packet_string_field(cache_config, &["fingerprint"]).map(str::to_string)
}

pub fn deep_review_cache_from_completed_reviewers(
    input: &Value,
    run_manifest: Option<&Value>,
    existing_cache: Option<&Value>,
) -> Option<DeepReviewCacheUpdate> {
    let fingerprint = deep_review_cache_fingerprint(run_manifest)?;
    let matching_existing_cache = existing_cache
        .map(DeepReviewIncrementalCache::from_value)
        .filter(|cache| cache.fingerprint() == fingerprint);
    let mut cache = matching_existing_cache
        .clone()
        .unwrap_or_else(|| DeepReviewIncrementalCache::new(&fingerprint));
    let mut stored_count = 0usize;
    let mut hit_count = 0usize;
    let mut miss_count = 0usize;

    if let Some(reviewers) = input.get("reviewers").and_then(Value::as_array) {
        for reviewer in reviewers {
            let is_completed = reviewer
                .get("status")
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|status| status == "completed");
            if !is_completed {
                continue;
            }
            let Some(packet_id) = normalized_non_empty_string(reviewer.get("packet_id")) else {
                continue;
            };
            if matching_existing_cache
                .as_ref()
                .and_then(|cache| cache.get_packet(&packet_id))
                .is_some()
            {
                hit_count += 1;
            } else {
                miss_count += 1;
            }
            let output = serde_json::to_string(reviewer).unwrap_or_else(|_| reviewer.to_string());
            cache.store_packet(&packet_id, &output);
            stored_count += 1;
        }
    }

    (stored_count > 0).then(|| DeepReviewCacheUpdate {
        value: cache.to_value(),
        hit_count,
        miss_count,
    })
}

fn value_for_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn packet_string_field<'a>(packet: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| packet.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn normalized_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
