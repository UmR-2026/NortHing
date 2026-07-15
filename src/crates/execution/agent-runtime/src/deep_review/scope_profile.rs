//! Deep Review scope profile — typed view of `manifest.scopeProfile`.
//!
//! `DeepReviewScopeProfile::from_manifest` parses the optional
//! `scopeProfile` / `scope_profile` object on the launch manifest. The
//! frontend builds this section, but Rust owns defensive parsing and the
//! final trust boundary: spellings are accepted in both camelCase and
//! snake_case for backward compatibility with older manifests, and
//! non-`deep` review modes or unrecognised review depths silently fall
//! back to `None`.
//!
//! Reduced-depth review (`review_depth != "full_depth"`) is exposed via
//! `is_reduced_depth` so the launch path can require extra policy
//! confirmation before proceeding.

use super::manifest_helpers::{normalized_non_empty_string, scope_dependency_hops_to_string, string_for_any_key};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewScopeProfile {
    review_depth: String,
    risk_focus_tags: Vec<String>,
    max_dependency_hops: Option<String>,
    optional_reviewer_policy: Option<String>,
    allow_broad_tool_exploration: bool,
    coverage_expectation: Option<String>,
}

impl DeepReviewScopeProfile {
    pub fn from_manifest(raw: &Value) -> Option<Self> {
        let manifest = raw.as_object()?;
        let review_mode = string_for_any_key(raw, &["reviewMode", "review_mode"])?;
        if review_mode != "deep" {
            return None;
        }

        let profile = manifest
            .get("scopeProfile")
            .or_else(|| manifest.get("scope_profile"))?
            .as_object()?;
        let review_depth = profile
            .get("reviewDepth")
            .or_else(|| profile.get("review_depth"))
            .and_then(normalized_non_empty_string)?;
        if !matches!(review_depth.as_str(), "high_risk_only" | "risk_expanded" | "full_depth") {
            return None;
        }

        let risk_focus_tags = profile
            .get("riskFocusTags")
            .or_else(|| profile.get("risk_focus_tags"))
            .and_then(Value::as_array)
            .map(|tags| {
                tags.iter()
                    .filter_map(|tag| tag.as_str().map(str::trim))
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Some(Self {
            review_depth,
            risk_focus_tags,
            max_dependency_hops: profile
                .get("maxDependencyHops")
                .or_else(|| profile.get("max_dependency_hops"))
                .and_then(scope_dependency_hops_to_string),
            optional_reviewer_policy: profile
                .get("optionalReviewerPolicy")
                .or_else(|| profile.get("optional_reviewer_policy"))
                .and_then(normalized_non_empty_string),
            allow_broad_tool_exploration: profile
                .get("allowBroadToolExploration")
                .or_else(|| profile.get("allow_broad_tool_exploration"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            coverage_expectation: profile
                .get("coverageExpectation")
                .or_else(|| profile.get("coverage_expectation"))
                .and_then(normalized_non_empty_string),
        })
    }

    pub fn coverage_expectation(&self) -> Option<&str> {
        self.coverage_expectation.as_deref()
    }

    pub fn is_reduced_depth(&self) -> bool {
        self.review_depth != "full_depth"
    }
}

#[cfg(test)]
impl DeepReviewScopeProfile {
    pub fn review_depth(&self) -> &str {
        &self.review_depth
    }

    pub fn risk_focus_tags(&self) -> &[String] {
        &self.risk_focus_tags
    }

    pub fn max_dependency_hops(&self) -> Option<&str> {
        self.max_dependency_hops.as_deref()
    }

    pub fn optional_reviewer_policy(&self) -> Option<&str> {
        self.optional_reviewer_policy.as_deref()
    }

    pub fn allow_broad_tool_exploration(&self) -> bool {
        self.allow_broad_tool_exploration
    }
}

#[cfg(test)]
mod tests {
    use super::DeepReviewScopeProfile;
    use serde_json::json;

    #[test]
    fn scope_profile_parses_camel_case_manifest() {
        let manifest = json!({
            "reviewMode": "deep",
            "scopeProfile": {
                "reviewDepth": "high_risk_only",
                "riskFocusTags": ["security", "cross_boundary_api_contracts"],
                "maxDependencyHops": 0,
                "optionalReviewerPolicy": "risk_matched_only",
                "allowBroadToolExploration": false,
                "coverageExpectation": "High-risk-only pass."
            }
        });

        let profile = DeepReviewScopeProfile::from_manifest(&manifest).expect("scope profile should parse");

        assert_eq!(profile.review_depth(), "high_risk_only");
        assert_eq!(
            profile.risk_focus_tags().iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["security", "cross_boundary_api_contracts"]
        );
        assert_eq!(profile.max_dependency_hops(), Some("0"));
        assert_eq!(profile.optional_reviewer_policy(), Some("risk_matched_only"));
        assert!(!profile.allow_broad_tool_exploration());
        assert_eq!(profile.coverage_expectation(), Some("High-risk-only pass."));
        assert!(profile.is_reduced_depth());
    }

    #[test]
    fn scope_profile_parses_snake_case_manifest() {
        let manifest = json!({
            "review_mode": "deep",
            "scope_profile": {
                "review_depth": "full_depth",
                "risk_focus_tags": ["security"],
                "max_dependency_hops": "policy_limited",
                "optional_reviewer_policy": "full",
                "allow_broad_tool_exploration": true,
                "coverage_expectation": "Full-depth pass."
            }
        });

        let profile = DeepReviewScopeProfile::from_manifest(&manifest).expect("scope profile should parse");

        assert_eq!(profile.review_depth(), "full_depth");
        assert_eq!(profile.max_dependency_hops(), Some("policy_limited"));
        assert!(profile.allow_broad_tool_exploration());
        assert!(!profile.is_reduced_depth());
    }

    #[test]
    fn scope_profile_missing_stays_compatible_with_legacy_manifest() {
        let manifest = json!({
            "reviewMode": "deep",
            "workPackets": []
        });

        assert!(DeepReviewScopeProfile::from_manifest(&manifest).is_none());
    }
}
