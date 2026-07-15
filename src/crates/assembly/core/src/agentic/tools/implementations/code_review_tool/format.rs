//! Result formatting for `CodeReviewTool`.
//!
//! Owns the fallback/default review result JSON construction used when
//! retries exhaust all attempts.

use serde_json::{json, Value};

impl super::CodeReviewTool {
    /// Generate a review result using all default values.
    ///
    /// Used when retries fail multiple times.
    pub fn create_default_result() -> Value {
        json!({
            "schema_version": 1,
            "summary": {
                "overall_assessment": "None",
                "risk_level": "low",
                "recommended_action": "approve",
                "confidence_note": "AI review failed, using default result"
            },
            "issues": [],
            "positive_points": ["None"],
            "review_mode": "standard",
            "reviewers": [],
            "remediation_plan": [],
            "schema_version": 1
        })
    }
}
