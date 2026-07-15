//! Deep Review run manifest gate — active-vs-skipped subagent accounting.
//!
//! `DeepReviewRunManifestGate` reads the launch manifest once and answers
//! the policy question: "is this subagent active for this review target?"
//! It collects active subagent ids from `workPackets`, `coreReviewers`,
//! `enabledExtraReviewers`, and `qualityGateReviewer`, and remembers the
//! skip reason for every reviewer listed in `skippedReviewers`.
//!
//! `ensure_active` is the only call site outside this module: any
//! review step that requires a subagent must succeed against this gate
//! before it can proceed, and a missing reviewer surfaces a typed
//! `DeepReviewPolicyViolation` rather than a generic 404.

use super::execution_policy::DeepReviewPolicyViolation;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewRunManifestGate {
    active_subagent_ids: HashSet<String>,
    skipped_subagent_reasons: HashMap<String, String>,
}

impl DeepReviewRunManifestGate {
    pub fn from_value(raw: &Value) -> Option<Self> {
        let manifest = raw.as_object()?;
        if manifest.get("reviewMode").and_then(Value::as_str) != Some("deep") {
            return None;
        }

        let mut active_subagent_ids = HashSet::new();
        collect_manifest_members(manifest.get("workPackets"), &mut active_subagent_ids);
        collect_manifest_members(manifest.get("coreReviewers"), &mut active_subagent_ids);
        collect_manifest_members(manifest.get("enabledExtraReviewers"), &mut active_subagent_ids);
        if let Some(id) = manifest
            .get("qualityGateReviewer")
            .and_then(manifest_member_subagent_id)
        {
            active_subagent_ids.insert(id);
        }

        if active_subagent_ids.is_empty() {
            return None;
        }

        let mut skipped_subagent_reasons = HashMap::new();
        if let Some(skipped) = manifest.get("skippedReviewers").and_then(Value::as_array) {
            for member in skipped {
                let Some(id) = manifest_member_subagent_id(member) else {
                    continue;
                };
                let reason = member.get("reason").and_then(Value::as_str).unwrap_or("skipped").trim();
                skipped_subagent_reasons.insert(
                    id,
                    if reason.is_empty() {
                        "skipped".to_string()
                    } else {
                        reason.to_string()
                    },
                );
            }
        }

        Some(Self {
            active_subagent_ids,
            skipped_subagent_reasons,
        })
    }

    pub fn ensure_active(&self, subagent_type: &str) -> Result<(), DeepReviewPolicyViolation> {
        if self.active_subagent_ids.contains(subagent_type) {
            return Ok(());
        }

        let reason = self
            .skipped_subagent_reasons
            .get(subagent_type)
            .map(String::as_str)
            .unwrap_or("missing_from_manifest");

        Err(DeepReviewPolicyViolation::new(
            "deep_review_subagent_not_active_for_target",
            format!(
                "DeepReview subagent '{}' is not active for this review target (reason: {})",
                subagent_type, reason
            ),
        ))
    }
}

fn collect_manifest_members(raw: Option<&Value>, output: &mut HashSet<String>) {
    let Some(values) = raw.and_then(Value::as_array) else {
        return;
    };

    for member in values {
        if let Some(id) = manifest_member_subagent_id(member) {
            output.insert(id);
        }
    }
}

fn manifest_member_subagent_id(value: &Value) -> Option<String> {
    let id = value
        .get("subagentId")
        .or_else(|| value.get("subagent_id"))
        .and_then(Value::as_str)?
        .trim();
    (!id.is_empty()).then(|| id.to_string())
}
