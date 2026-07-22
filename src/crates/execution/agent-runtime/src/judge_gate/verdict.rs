//! Verdict parser - parses judge output according to §3 protocol.

use super::redlines::REDLINE_TABLE;
use super::types::{ParsedVerdict, RuleCheck, RuleStatus, VerdictKind, VerdictMalformed};

const VERDICT_JSON_BEGIN: &str = "VERDICT_JSON_BEGIN";
const VERDICT_JSON_END: &str = "VERDICT_JSON_END";

/// Parses the judge verdict text and validates it against the protocol rules.
///
/// Returns Ok(ParsedVerdict) if the text is valid, Err(VerdictMalformed) otherwise.
///
/// The parsing rules from §3:
/// 1. Exactly one verdict block; block contains valid JSON; verdict ∈ {approve, reject}
/// 2. rule_checks length exactly 4; rule IDs match REDLINE_TABLE one-to-one (no missing,
///    no duplicates, no unknown, no extra); status ∈ {pass, violation}
/// 3. evidence_assessment non-empty and references at least one valid evidence ID
/// 4. rationale non-empty
pub fn parse_verdict(text: &str, valid_evidence_ids: &[String]) -> Result<ParsedVerdict, VerdictMalformed> {
    // Find verdict blocks - must have exactly one BEGIN and one END with content between
    let begin_pos = text.find(VERDICT_JSON_BEGIN);
    let end_pos = text.find(VERDICT_JSON_END);

    let block = match (begin_pos, end_pos) {
        (None, None) => return Err(VerdictMalformed::NoVerdictBlock),
        (None, Some(_)) => return Err(VerdictMalformed::NoVerdictBlock), // END without BEGIN
        (Some(_), None) => return Err(VerdictMalformed::NoVerdictBlock), // BEGIN without END
        (Some(begin_idx), Some(end_idx)) => {
            if begin_idx >= end_idx {
                return Err(VerdictMalformed::NoVerdictBlock); // END before BEGIN
            }
            // Check there's no second BEGIN or END
            let second_begin = text[end_idx + VERDICT_JSON_END.len()..].find(VERDICT_JSON_BEGIN);
            let second_end = text[begin_idx + VERDICT_JSON_BEGIN.len()..end_idx].find(VERDICT_JSON_END);
            if second_begin.is_some() || second_end.is_some() {
                return Err(VerdictMalformed::MultipleVerdictBlocks);
            }

            let block_content = text[begin_idx + VERDICT_JSON_BEGIN.len()..end_idx].trim();
            if block_content.is_empty() {
                return Err(VerdictMalformed::BlockContentNotJson { cause: "empty block".to_string() });
            }
            block_content
        }
    };

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(block)
        .map_err(|e| VerdictMalformed::BlockContentNotJson {
            cause: e.to_string(),
        })?;

    // Extract and validate verdict field
    let verdict_str = json
        .get("verdict")
        .and_then(|v| v.as_str())
        .ok_or(VerdictMalformed::VerdictFieldMissing)?;

    let verdict = match verdict_str {
        "approve" => VerdictKind::Approve,
        "reject" => VerdictKind::Reject,
        _ => {
            return Err(VerdictMalformed::VerdictFieldInvalid {
                value: verdict_str.to_string(),
            })
        }
    };

    // Extract and validate rule_checks
    let rule_checks_arr = json
        .get("rule_checks")
        .and_then(|v| v.as_array())
        .ok_or(VerdictMalformed::RuleChecksMissing)?;

    if rule_checks_arr.len() != 4 {
        return Err(VerdictMalformed::RuleChecksWrongCount {
            expected: 4,
            actual: rule_checks_arr.len(),
        });
    }

    // Build expected rule IDs set
    let expected_rule_ids: Vec<&str> = REDLINE_TABLE.iter().map(|r| r.id).collect();
    let mut seen_rule_ids: Vec<&str> = Vec::new();
    let mut rule_checks: Vec<RuleCheck> = Vec::new();

    for item in rule_checks_arr {
        let rule_id = item
            .get("rule")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VerdictMalformed::RuleCheckMissing {
                rule_id: String::new(),
            })?;

        let status_str = item
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VerdictMalformed::RuleStatusInvalid {
                rule_id: rule_id.to_string(),
                value: String::new(),
            })?;

        let status = match status_str {
            "pass" => RuleStatus::Pass,
            "violation" => RuleStatus::Violation,
            _ => {
                return Err(VerdictMalformed::RuleStatusInvalid {
                    rule_id: rule_id.to_string(),
                    value: status_str.to_string(),
                })
            }
        };

        // Check for missing rule (rule ID not in REDLINE_TABLE)
        if !expected_rule_ids.contains(&rule_id) {
            return Err(VerdictMalformed::RuleCheckUnknown {
                rule_id: rule_id.to_string(),
            });
        }

        // Check for duplicate rule
        if seen_rule_ids.contains(&rule_id) {
            return Err(VerdictMalformed::RuleCheckDuplicate {
                rule_id: rule_id.to_string(),
            });
        }

        seen_rule_ids.push(rule_id);
        rule_checks.push(RuleCheck {
            rule: rule_id.to_string(),
            status,
        });
    }

    // Check for missing rules (any expected rule not in seen_rule_ids)
    for expected_id in &expected_rule_ids {
        if !seen_rule_ids.contains(expected_id) {
            return Err(VerdictMalformed::RuleCheckMissing {
                rule_id: expected_id.to_string(),
            });
        }
    }

    // Check for extra rules (seen_rule_ids has more than expected - shouldn't happen given length check)
    if seen_rule_ids.len() != 4 {
        return Err(VerdictMalformed::RuleCheckExtra);
    }

    // Validate evidence_assessment
    let evidence_assessment = json
        .get("evidence_assessment")
        .and_then(|v| v.as_str())
        .ok_or(VerdictMalformed::EvidenceAssessmentEmpty)?;

    if evidence_assessment.is_empty() {
        return Err(VerdictMalformed::EvidenceAssessmentEmpty);
    }

    // Check that evidence_assessment references at least one valid evidence ID
    let has_valid_reference = valid_evidence_ids.iter().any(|id| {
        // Match T#, F#, S1, H# patterns
        evidence_assessment.contains(id)
    });

    if !has_valid_reference {
        return Err(VerdictMalformed::EvidenceAssessmentNoReference);
    }

    // Validate rationale
    let rationale = json
        .get("rationale")
        .and_then(|v| v.as_str())
        .ok_or(VerdictMalformed::RationaleEmpty)?;

    if rationale.is_empty() {
        return Err(VerdictMalformed::RationaleEmpty);
    }

    Ok(ParsedVerdict {
        verdict,
        rule_checks,
        evidence_assessment: evidence_assessment.to_string(),
        rationale: rationale.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::RuleStatus;

    const VALID_EVIDENCE_IDS: [&str; 4] = ["T1", "F1", "S1", "H1"];

    fn make_valid_verdict(verdict_str: &str, all_pass: bool) -> String {
        let status1 = if all_pass { "pass" } else { "violation" };
        format!(
            r#"VERDICT_JSON_BEGIN
{{
  "verdict": "{}",
  "rule_checks": [
    {{"rule": "I-NEG-1", "status": "{}"}},
    {{"rule": "I-NEG-2", "status": "pass"}},
    {{"rule": "I-NEG-3", "status": "pass"}},
    {{"rule": "I-NEG-4", "status": "pass"}}
  ],
  "evidence_assessment": "Based on T1 and F1 evidence, the candidate meets all requirements.",
  "rationale": "All redlines pass."
}}
VERDICT_JSON_END"#,
            verdict_str, status1
        )
    }

    fn evidence_ids() -> Vec<String> {
        VALID_EVIDENCE_IDS.iter().map(|s| s.to_string()).collect()
    }

    // Test 1: approve with all pass -> Ok(approve)
    #[test]
    fn parse_approve_all_pass_ok() {
        let text = make_valid_verdict("approve", true);
        let result = parse_verdict(&text, &evidence_ids());
        assert!(result.is_ok());
        let pv = result.unwrap();
        assert_eq!(pv.verdict, VerdictKind::Approve);
        assert_eq!(pv.rule_checks.len(), 4);
        for rc in &pv.rule_checks {
            assert_eq!(rc.status, RuleStatus::Pass);
        }
    }

    // Test 2: any violation -> Ok but approve not valid (semantic check in test)
    #[test]
    fn parse_approve_with_violation_still_parses() {
        let text = make_valid_verdict("approve", false);
        let result = parse_verdict(&text, &evidence_ids());
        // Parsing succeeds but verdict field says approve while there's a violation
        assert!(result.is_ok());
        let pv = result.unwrap();
        assert_eq!(pv.verdict, VerdictKind::Approve);
        // One rule has violation
        assert!(pv.rule_checks.iter().any(|rc| rc.status == RuleStatus::Violation));
    }

    // Test 3: reject with all pass
    #[test]
    fn parse_reject_with_all_pass() {
        let text = make_valid_verdict("reject", true);
        let result = parse_verdict(&text, &evidence_ids());
        assert!(result.is_ok());
        let pv = result.unwrap();
        assert_eq!(pv.verdict, VerdictKind::Reject);
    }

    // Test: missing rule
    #[test]
    fn parse_missing_rule() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "Three rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleChecksWrongCount { .. }));
    }

    // Test: duplicate rule
    #[test]
    fn parse_duplicate_rule() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "All rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleCheckDuplicate { .. }));
    }

    // Test: unknown rule
    #[test]
    fn parse_unknown_rule() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-99", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "All rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleCheckUnknown { .. }));
    }

    // Test: extra rule
    #[test]
    fn parse_extra_rule() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"},
    {"rule": "I-NEG-X", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "All rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleChecksWrongCount { .. }));
    }

    // Test: status=not_evaluated
    #[test]
    fn parse_status_not_evaluated() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "not_evaluated"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "All rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleStatusInvalid { .. }));
    }

    // Test: status other invalid value
    #[test]
    fn parse_status_invalid_value() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "maybe"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": "All rules pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RuleStatusInvalid { .. }));
    }

    // Test: zero blocks
    #[test]
    fn parse_zero_blocks() {
        let text = "This is not a verdict.";
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::NoVerdictBlock));
    }

    // Test: two blocks
    #[test]
    fn parse_two_blocks() {
        let text = r#"VERDICT_JSON_BEGIN
{"verdict": "approve", "rule_checks": [], "evidence_assessment": "x", "rationale": "y"}
VERDICT_JSON_END
Some text in between
VERDICT_JSON_BEGIN
{"verdict": "reject", "rule_checks": [], "evidence_assessment": "x", "rationale": "y"}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::MultipleVerdictBlocks));
    }

    // Test: block content not JSON
    #[test]
    fn parse_block_not_json() {
        let text = r#"VERDICT_JSON_BEGIN
This is not JSON
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::BlockContentNotJson { .. }));
    }

    // Test: verdict field invalid value
    #[test]
    fn parse_verdict_invalid_value() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "maybe",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T1",
  "rationale": "All pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::VerdictFieldInvalid { .. }));
    }

    // Test: evidence_assessment empty
    #[test]
    fn parse_evidence_assessment_empty() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "",
  "rationale": "All pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::EvidenceAssessmentEmpty));
    }

    // Test: evidence_assessment references non-existent ID
    #[test]
    fn parse_evidence_assessment_nonexistent_reference() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T9 shows good results",
  "rationale": "All pass."
}
VERDICT_JSON_END"#;
        let result = parse_verdict(&text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::EvidenceAssessmentNoReference));
    }

    // Test: rationale empty
    #[test]
    fn parse_rationale_empty() {
        let text = r#"VERDICT_JSON_BEGIN
{
  "verdict": "approve",
  "rule_checks": [
    {"rule": "I-NEG-1", "status": "pass"},
    {"rule": "I-NEG-2", "status": "pass"},
    {"rule": "I-NEG-3", "status": "pass"},
    {"rule": "I-NEG-4", "status": "pass"}
  ],
  "evidence_assessment": "T1 shows good results",
  "rationale": ""
}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::RationaleEmpty));
    }

    // Test: approve but four not all pass - semantic check
    #[test]
    fn parse_approve_but_not_all_pass_semantic_check() {
        // This test verifies that even if parsing succeeds, the caller must check
        // that approve only holds when all 4 rules are Pass
        let text = make_valid_verdict("approve", false); // status1 = "violation"
        let result = parse_verdict(&text, &evidence_ids());
        assert!(result.is_ok()); // Parsing succeeds
        let pv = result.unwrap();
        // But the semantic check (all pass) fails - caller must detect this
        assert!(pv.rule_checks.iter().any(|rc| rc.status == RuleStatus::Violation));
        // The approve verdict is present but semantically invalid
        assert_eq!(pv.verdict, VerdictKind::Approve);
    }
}
