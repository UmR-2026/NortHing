//! Judge brief builder - constructs the prompt sent to the judge gate.

use super::redlines::REDLINE_TABLE;
use super::types::{subject_digest, GateRequest};

const VERDICT_JSON_BEGIN: &str = "VERDICT_JSON_BEGIN";
const VERDICT_JSON_END: &str = "VERDICT_JSON_END";
const BRIEF_BUDGET_LIMIT: usize = 12_000;

/// Builds the judge brief for a gate request.
/// The brief includes:
/// - Role instruction (redline judge, no weight definition authority)
/// - 4 redline rules verbatim
/// - Evidence ID list with content (by ID reference)
/// - "candidate self-report weight = 0" instruction
/// - §3 verdict protocol output format
/// - subject_digest
pub fn build_judge_brief(request: &GateRequest) -> String {
    let subject_digest_str = subject_digest(&request.subject);
    let evidence_ids = request.evidence.evidence_ids();
    let evidence_ids_list = evidence_ids.join(", ");

    let mut brief = String::new();

    // Role instruction
    brief.push_str("You are the GateJudge — a redline enforcement judge with no authority to define weights or override invariant rules.\n\n");

    // The 4 redline rules verbatim
    brief.push_str("## Inviolable Redline Rules (I-NEG-1 through I-NEG-4)\n\n");
    for rule in &REDLINE_TABLE {
        brief.push_str(&format!("### {}\n", rule.id));
        brief.push_str(rule.statement);
        brief.push_str("\n\n");
    }

    // Evidence ID list with content
    brief.push_str("## Evidence IDs and Content\n");
    brief.push_str(&format!("Evidence IDs in this request: {}\n\n", evidence_ids_list));

    // Traces
    for (i, trace) in request.evidence.traces.iter().enumerate() {
        brief.push_str(&format!("T{}: turn_id={}, tool={}, error_excerpt=\"{}\"",
            i + 1, trace.turn_id, trace.tool, trace.error_excerpt));
        if let Some(ref repair) = trace.repair_excerpt {
            brief.push_str(&format!(", repair_excerpt=\"{}\"", repair));
        }
        brief.push_str("\n");
    }

    // FS diffs
    for (i, diff) in request.evidence.fs_diffs.iter().enumerate() {
        brief.push_str(&format!("F{}: path={}, added={}, removed={}\n",
            i + 1, diff.path, diff.added, diff.removed));
    }

    // Success rate
    brief.push_str("S1: success_rate comparison\n");

    // Human feedback
    if let super::types::HumanFeedbackSlot::Present(ref feedbacks) = request.evidence.human_feedback {
        for (i, fb) in feedbacks.iter().enumerate() {
            brief.push_str(&format!("H{}: origin={}, excerpt=\"{}\"\n", i + 1, fb.origin, fb.excerpt));
        }
    }

    brief.push_str("\n");

    // Candidate self-report weight = 0
    brief.push_str("## Critical Constraint\n");
    brief.push_str("The candidate's self-reported assessments have ZERO weight in your evaluation.\n");
    brief.push_str("You must base your verdict ONLY on the objective evidence IDs listed above.\n\n");

    // Verdict protocol output format
    brief.push_str("## Verdict Protocol (output format is NON-NEGOTIABLE)\n\n");
    brief.push_str(&format!("Your response MUST contain exactly ONE block delimited by:\n"));
    brief.push_str(&format!("{}\n", VERDICT_JSON_BEGIN));
    brief.push_str(&format!("{}\n\n", VERDICT_JSON_END));
    brief.push_str("Inside the block, output valid JSON with these exact fields:\n");
    brief.push_str("- \"verdict\": MUST be either \"approve\" or \"reject\"\n");
    brief.push_str("- \"rule_checks\": array of exactly 4 objects, one per redline rule I-NEG-1 through I-NEG-4\n");
    brief.push_str("  - each object: {\"rule\": \"<rule_id>\", \"status\": \"pass\" or \"violation\"}\n");
    brief.push_str("- \"evidence_assessment\": your evaluation of the evidence, MUST reference at least one evidence ID (T#, F#, S1, or H#)\n");
    brief.push_str("- \"rationale\": your reasoning for the verdict\n\n");
    brief.push_str("## Approve Sufficient and Necessary Conditions\n");
    brief.push_str("APPROVE is valid ONLY when ALL FOUR redline rules I-NEG-1 through I-NEG-4 are \"pass\".\n");
    brief.push_str("If ANY rule shows \"violation\", the verdict MUST be \"reject\".\n\n");

    // Subject digest
    brief.push_str(&format!("## Subject Digest (for receipt binding)\n"));
    brief.push_str(&format!("Subject digest: {}\n", subject_digest_str));

    // Debug assert budget
    debug_assert!(
        brief.len() <= BRIEF_BUDGET_LIMIT + evidence_ids_list.len() + 100,
        "Brief exceeds budget limit"
    );

    brief
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{
        ActionKind, AbsentReason, EvidencePack, HumanFeedbackSlot, RateSample,
        SuccessRateComparison, ToolTraceEvidence,
    };

    fn make_request() -> GateRequest {
        GateRequest {
            action_kind: ActionKind::PromoteSkillCandidate,
            subject: b"test subject content".to_vec(),
            evidence: EvidencePack {
                traces: vec![ToolTraceEvidence {
                    turn_id: "turn-1".to_string(),
                    tool: "tool-a".to_string(),
                    error_excerpt: "error".to_string(),
                    repair_excerpt: None,
                }],
                fs_diffs: vec![],
                success_rate: SuccessRateComparison {
                    baseline: RateSample::NoBaselineYet,
                    candidate: RateSample::Present {
                        successes: 5,
                        attempts: 10,
                    },
                },
                human_feedback: HumanFeedbackSlot::Absent(AbsentReason::NoHumanExposureYet),
            },
        }
    }

    #[test]
    fn brief_contains_four_redline_ids_and_text() {
        let request = make_request();
        let brief = build_judge_brief(&request);

        // Check all 4 rule IDs are present
        assert!(brief.contains("I-NEG-1"));
        assert!(brief.contains("I-NEG-2"));
        assert!(brief.contains("I-NEG-3"));
        assert!(brief.contains("I-NEG-4"));

        // Check rule texts are present (the full Chinese statements)
        assert!(brief.contains("用户数据文件"));
        assert!(brief.contains("未过门的固化产物不得出现"));
        assert!(brief.contains("红线表与门禁执行代码不被固化动作自身修改"));
        assert!(brief.contains("审计日志只可追加"));
    }

    #[test]
    fn brief_contains_evidence_ids() {
        let request = make_request();
        let brief = build_judge_brief(&request);

        // Check evidence IDs
        assert!(brief.contains("T1"));
        assert!(brief.contains("S1"));
        // No F1 (fs_diffs empty), no H1 (human_feedback absent)
        assert!(brief.contains("Evidence IDs"));
    }

    #[test]
    fn brief_contains_weight_zero_instruction() {
        let request = make_request();
        let brief = build_judge_brief(&request);

        assert!(brief.contains("ZERO weight"));
        assert!(brief.contains("candidate's self-reported assessments"));
    }

    #[test]
    fn brief_contains_verdict_markers() {
        let request = make_request();
        let brief = build_judge_brief(&request);

        assert!(brief.contains("VERDICT_JSON_BEGIN"));
        assert!(brief.contains("VERDICT_JSON_END"));
    }

    #[test]
    fn brief_contains_subject_digest() {
        let request = make_request();
        let brief = build_judge_brief(&request);

        assert!(brief.contains("sha256:v1:"));
        // Verify it's a 64-char hex string after the prefix
        let sd = subject_digest(&request.subject);
        assert_eq!(&sd[..10], "sha256:v1:");
        assert_eq!(sd.len(), 10 + 64);
    }
}
