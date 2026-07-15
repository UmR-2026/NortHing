//! Deep Review findings/issues enrichment.
//!
//! Owns packet metadata inference and `reviewers` array shaping for the
//! findings section of a Deep Review report.

use super::super::manifest::DeepReviewScopeProfile;
use crate::deep_review::manifest::{DeepReviewEvidencePack, DeepReviewRunManifestGate};
use serde_json::{json, Value};

pub fn fill_deep_review_packet_metadata(input: &mut Value, run_manifest: Option<&Value>) {
    let Some(reviewers) = input.get_mut("reviewers").and_then(Value::as_array_mut) else {
        return;
    };

    for reviewer in reviewers {
        let packet_id = normalized_non_empty_string(reviewer.get("packet_id"));
        let packet_status_source = normalized_non_empty_string(reviewer.get("packet_status_source"));
        let inferred_packet_id = if packet_id.is_none() {
            infer_unique_packet_id_for_reviewer(reviewer, run_manifest)
        } else {
            None
        };

        let Some(object) = reviewer.as_object_mut() else {
            continue;
        };

        if packet_id.is_some() {
            if packet_status_source.is_none() {
                object.insert("packet_status_source".to_string(), json!("reported"));
            }
        } else if let Some(inferred_packet_id) = inferred_packet_id {
            object.insert("packet_id".to_string(), json!(inferred_packet_id));
            object.insert("packet_status_source".to_string(), json!("inferred"));
        } else if packet_status_source.is_none() {
            object.insert("packet_status_source".to_string(), json!("missing"));
        }
    }
}

fn normalized_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn packet_string_field<'a>(packet: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| packet.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn reviewer_match_tokens(reviewer: &Value) -> Vec<String> {
    ["name", "specialty"]
        .iter()
        .filter_map(|key| normalized_non_empty_string(reviewer.get(*key)))
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn packet_match_tokens(packet: &Value) -> Vec<String> {
    [
        &["subagentId", "subagent_id", "subagent_type"][..],
        &["displayName", "display_name"][..],
        &["roleName", "role"][..],
    ]
    .iter()
    .filter_map(|keys| packet_string_field(packet, keys))
    .map(|value| value.to_ascii_lowercase())
    .collect()
}

fn infer_unique_packet_id_for_reviewer(reviewer: &Value, run_manifest: Option<&Value>) -> Option<String> {
    let reviewer_tokens = reviewer_match_tokens(reviewer);
    if reviewer_tokens.is_empty() {
        return None;
    }

    let manifest = run_manifest?;
    let packets = manifest
        .get("workPackets")
        .or_else(|| manifest.get("work_packets"))?
        .as_array()?;
    let mut matches = packets.iter().filter_map(|packet| {
        let packet_id = packet_string_field(packet, &["packetId", "packet_id"])?;
        let packet_tokens = packet_match_tokens(packet);
        let matched = packet_tokens
            .iter()
            .any(|packet_token| reviewer_tokens.iter().any(|token| token == packet_token));
        matched.then(|| packet_id.to_string())
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(first)
    }
}
