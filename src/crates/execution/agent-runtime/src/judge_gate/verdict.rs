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
    // Must have exactly one BEGIN and exactly one END marker
    if text.matches(VERDICT_JSON_BEGIN).count() != 1 {
        return Err(VerdictMalformed::NoVerdictBlock);
    }
    if text.matches(VERDICT_JSON_END).count() != 1 {
        return Err(VerdictMalformed::NoVerdictBlock);
    }

    let begin_pos = text.find(VERDICT_JSON_BEGIN).unwrap();
    let end_pos = text.find(VERDICT_JSON_END).unwrap();

    if begin_pos >= end_pos {
        return Err(VerdictMalformed::NoVerdictBlock); // END before or at BEGIN
    }

    let block_content = text[begin_pos + VERDICT_JSON_BEGIN.len()..end_pos].trim();
    if block_content.is_empty() {
        return Err(VerdictMalformed::BlockContentNotJson { cause: "empty block".to_string() });
    }

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(block_content)
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

    // Length check above already guarantees exactly 4 rules, so extra check is unnecessary.

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

    // Test 2: reject with all pass
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
        assert!(matches!(result.unwrap_err(), VerdictMalformed::NoVerdictBlock));
    }

    // Test: two END markers (BEGIN..END\nEND)
    #[test]
    fn parse_two_end_markers_rejected() {
        let text = r#"VERDICT_JSON_BEGIN
{"verdict": "approve", "rule_checks": [], "evidence_assessment": "x", "rationale": "y"}
VERDICT_JSON_END
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::NoVerdictBlock));
    }

    // Test: two BEGIN markers
    #[test]
    fn parse_two_begin_markers_rejected() {
        let text = r#"VERDICT_JSON_BEGIN
VERDICT_JSON_BEGIN
{"verdict": "approve", "rule_checks": [], "evidence_assessment": "x", "rationale": "y"}
VERDICT_JSON_END"#;
        let result = parse_verdict(text, &evidence_ids());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerdictMalformed::NoVerdictBlock));
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
