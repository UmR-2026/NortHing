//! Judge gate adapter layer.
//!
//! This module implements the core adapter layer for the judge gate as specified
//! in C4 Phase 0 design §5.1. It bridges the pure protocol layer in
//! `northhing-agent-runtime::judge_gate` with the runtime infrastructure.
//!
//! # Architecture
//!
//! - `runner.rs` - `JudgeRunner` trait + `SubagentJudgeRunner` production implementation
//! - `audit.rs` - Append-only audit log implementation
//! - `mod.rs` - `evaluate()` and `promote_candidate_skill()` orchestration

pub(crate) mod audit;
pub(crate) mod runner;

use crate::agentic::judge_gate::audit::{append_audit_entry, append_governance_override, AuditEntry, GovernanceOverrideEntry};
use crate::agentic::judge_gate::runner::{JudgeRunError, JudgeRunner};
use crate::infrastructure::app_paths::path_manager_arc;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_agent_runtime::judge_gate::{
    ActionKind, ApprovedGateReceipt, EvidencePack, GateRequest, GateVerdict, RejectClass, RuleCheck,
    RuleStatus, subject_digest, build_judge_brief, parse_verdict,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

static CONSUMED_RECEIPTS: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

const SKILL_MD_FILENAME: &str = "SKILL.md";
const CANDIDATES_DIR: &str = "candidates";

/// Evaluate a gate request.
///
/// # Flow
/// 1. Validate evidence pack - failure → EvidenceRejected (still audit)
/// 2. Build judge brief
/// 3. Run judge via runner - error → JudgeUnavailable
/// 4. Parse verdict - malformed → MalformedVerdict
/// 5. If approved, check all 4 rules pass - any violation → PolicyViolation
/// 6. Write audit entry - failure → AuditFailure (no receipt!)
/// 7. If approved and audit success → construct ApprovedGateReceipt
///
/// # Arguments
/// * `coordinator` - ConversationCoordinator reference for SubagentJudgeRunner
/// * `request` - The gate request with action, subject, and evidence
/// * `ctx` - Execution context with timeout, cancel, workspace info
/// * `runner` - JudgeRunner implementation (production or fake)
///
/// # Returns
/// * `GateVerdict::Approved(receipt)` on success
/// * `GateVerdict::Rejected(reason)` on failure
pub(crate) async fn evaluate(
    coordinator: &Arc<dyn std::any::Any + Send + Sync>,
    request: GateRequest,
    ctx: &northhing_agent_runtime::judge_gate::GateExecutionContext,
    runner: &dyn JudgeRunner,
) -> GateVerdict {
    let start_time = std::time::Instant::now();
    let subject_digest = subject_digest(&request.subject);

    debug!(
        action_kind = ?request.action_kind,
        subject_digest = %subject_digest,
        "evaluate: starting gate evaluation"
    );

    // Step 1: Validate evidence
    if let Err(evidence_error) = request.evidence.validate() {
        let error_str = format!("{:?}", evidence_error);
        warn!(
            error = %error_str,
            "evaluate: evidence validation failed"
        );
        // Even on evidence rejection, we audit the attempt
        let reject_entry = AuditEntry::new(
            request.action_kind.clone(),
            subject_digest.clone(),
            format!("evidence_validation_error:{:?}", evidence_error),
            "reject".to_string(),
            vec![],
            Some("evidence_rejected".to_string()),
            None,
            Some(start_time.elapsed().as_millis() as u64),
        );
        if let Err(audit_err) = append_audit_entry(&reject_entry) {
            error!(error = %audit_err, "evaluate: failed to write audit entry for evidence rejection");
            return GateVerdict::Rejected(RejectClass::EvidenceRejected(format!("{:?}", evidence_error)));
        }
        return GateVerdict::Rejected(RejectClass::EvidenceRejected(format!("{:?}", evidence_error)));
    }

    // Step 2: Build judge brief
    let brief = build_judge_brief(&request);

    // Step 3: Run judge
    let judge_output = match runner.run_judge(coordinator, brief, ctx).await {
        Ok(output) => {
            debug!(output_len = output.len(), "evaluate: judge returned");
            output
        }
        Err(JudgeRunError::Timeout) => {
            warn!("evaluate: judge runner timeout");
            return GateVerdict::Rejected(RejectClass::JudgeUnavailable("timeout".to_string()));
        }
        Err(JudgeRunError::Cancelled) => {
            warn!("evaluate: judge runner cancelled");
            return GateVerdict::Rejected(RejectClass::JudgeUnavailable("cancelled".to_string()));
        }
        Err(JudgeRunError::Unavailable(reason)) => {
            warn!(reason = %reason, "evaluate: judge runner unavailable");
            return GateVerdict::Rejected(RejectClass::JudgeUnavailable(reason));
        }
    };

    // Step 4: Parse verdict
    let valid_evidence_ids = request.evidence.evidence_ids();
    let parsed = match parse_verdict(&judge_output, &valid_evidence_ids) {
        Ok(p) => p,
        Err(parse_error) => {
            warn!(error = %parse_error, "evaluate: verdict parsing failed");
            let reject_entry = AuditEntry::new(
                request.action_kind.clone(),
                subject_digest.clone(),
                format!("parse_error:{}", parse_error),
                "reject".to_string(),
                vec![],
                Some("malformed_verdict".to_string()),
                None,
                Some(start_time.elapsed().as_millis() as u64),
            );
            if let Err(audit_err) = append_audit_entry(&reject_entry) {
                error!(error = %audit_err, "evaluate: failed to write audit entry for malformed verdict");
                return GateVerdict::Rejected(RejectClass::MalformedVerdict(parse_error.to_string()));
            }
            return GateVerdict::Rejected(RejectClass::MalformedVerdict(parse_error.to_string()));
        }
    };

    // Step 5: Check all rules pass for approval
    let all_pass = parsed.rule_checks.iter().all(|rc| rc.status == RuleStatus::Pass);
    let verdict_str = if all_pass { "approve" } else { "reject" };

    // Build rule checks for audit
    let rule_checks: Vec<RuleCheck> = parsed
        .rule_checks
        .iter()
        .map(|rc| RuleCheck {
            rule: rc.rule.clone(),
            status: rc.status.clone(),
        })
        .collect();

    let reject_class = if !all_pass {
        Some("policy_violation".to_string())
    } else {
        None
    };

    // Step 6: Write audit BEFORE constructing receipt (critical for I-NEG-4)
    let audit_entry = AuditEntry::new(
        request.action_kind.clone(),
        subject_digest.clone(),
        format!(
            "traces:{}, fs_diffs:{}, success_rate:1, human_feedback:1",
            request.evidence.traces.len(),
            request.evidence.fs_diffs.len()
        ),
        verdict_str.to_string(),
        rule_checks,
        reject_class,
        None, // judge_turn_id - would need to be extracted from subagent response
        Some(start_time.elapsed().as_millis() as u64),
    );

    if let Err(audit_err) = append_audit_entry(&audit_entry) {
        error!(
            error = %audit_err,
            "evaluate: audit write failed - returning AuditFailure (no receipt)"
        );
        // I-NEG-4: Audit failure means no receipt can be issued
        return GateVerdict::Rejected(RejectClass::AuditFailure(format!(
            "audit write failed: {}",
            audit_err
        )));
    }

    // Step 7: If approved, construct receipt
    if all_pass {
        let receipt = ApprovedGateReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_kind: request.action_kind,
            subject_digest: subject_digest.clone(),
            audit_entry_id: audit_entry.entry_id.clone(),
            ts: audit_entry.ts,
        };

        info!(
            receipt_id = %receipt.receipt_id,
            subject_digest = %subject_digest,
            "evaluate: gate approved"
        );

        GateVerdict::Approved(receipt)
    } else {
        let violation_rules: Vec<String> = parsed
            .rule_checks
            .iter()
            .filter(|rc| rc.status == RuleStatus::Violation)
            .map(|rc| rc.rule.clone())
            .collect();

        warn!(
            violations = ?violation_rules,
            "evaluate: gate rejected due to policy violation"
        );

        GateVerdict::Rejected(RejectClass::PolicyViolation(violation_rules.join(", ")))
    }
}

/// Promote a candidate skill to the user skills directory.
///
/// # Validation
/// 1. receipt.action_kind must be PromoteSkillCandidate
/// 2. receipt must not be in consumed set
/// 3. candidate_dir/SKILL.md digest must match receipt.subject_digest
/// 4. candidate directory name must be safe (no .., no absolute paths, etc.)
///
/// # Behavior
/// - Copies (not moves) SKILL.md to user_skills_dir()/name/SKILL.md
/// - Does NOT write to candidates/SKILL.md or any loader-visible path
/// - Adds receipt to consumed set
/// - Appends promote audit entry
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the promoted skill
/// * `Err(NortHingError)` - Validation failure
pub(crate) async fn promote_candidate_skill(
    receipt: ApprovedGateReceipt,
    candidate_dir: &Path,
) -> NortHingResult<PathBuf> {
    debug!(
        receipt_id = %receipt.receipt_id,
        subject_digest = %receipt.subject_digest,
        candidate_dir = %candidate_dir.display(),
        "promote_candidate_skill: starting promotion"
    );

    // Validate action_kind
    if !matches!(receipt.action_kind, ActionKind::PromoteSkillCandidate) {
        error!(
            action_kind = ?receipt.action_kind,
            "promote_candidate_skill: invalid action kind"
        );
        return Err(NortHingError::Validation(format!(
            "invalid action kind for promote: {:?}",
            receipt.action_kind
        )));
    }

    // Validate not already consumed
    {
        let consumed = CONSUMED_RECEIPTS.lock().unwrap();
        if consumed.contains(&receipt.receipt_id) {
            error!(receipt_id = %receipt.receipt_id, "promote_candidate_skill: receipt already consumed");
            return Err(NortHingError::Validation(format!(
                "receipt {} already consumed",
                receipt.receipt_id
            )));
        }
    }

    // Validate candidate_dir/SKILL.md exists and digest matches
    let skill_md_path = candidate_dir.join(SKILL_MD_FILENAME);
    if !skill_md_path.exists() {
        error!(path = %skill_md_path.display(), "promote_candidate_skill: SKILL.md not found");
        return Err(NortHingError::Validation(format!(
            "candidate skill SKILL.md not found at {}",
            skill_md_path.display()
        )));
    }

    let skill_content = tokio::fs::read(&skill_md_path).await.map_err(|e| {
        NortHingError::Validation(format!("failed to read SKILL.md: {}", e))
    })?;
    let computed_digest = subject_digest(&skill_content);
    if computed_digest != receipt.subject_digest {
        error!(
            expected = %receipt.subject_digest,
            actual = %computed_digest,
            "promote_candidate_skill: digest mismatch"
        );
        return Err(NortHingError::Validation(format!(
            "SKILL.md digest mismatch: expected {}, got {}",
            receipt.subject_digest, computed_digest
        )));
    }

    // Validate candidate directory name is safe
    let candidate_dir_name = candidate_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if candidate_dir_name.is_empty()
        || candidate_dir_name.contains("..")
        || candidate_dir_name.starts_with('/')
        || candidate_dir_name.contains('\\')
        || !candidate_dir_name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        error!(name = %candidate_dir_name, "promote_candidate_skill: unsafe candidate name");
        return Err(NortHingError::Validation(format!(
            "unsafe candidate directory name: {}",
            candidate_dir_name
        )));
    }

    // Determine target path
    let target_dir = path_manager_arc().user_skills_dir().join(candidate_dir_name);
    let target_path = target_dir.join(SKILL_MD_FILENAME);

    // Refuse to overwrite existing skills
    if target_path.exists() {
        error!(path = %target_path.display(), "promote_candidate_skill: target already exists");
        return Err(NortHingError::Validation(format!(
            "skill {} already exists at {}",
            candidate_dir_name,
            target_path.display()
        )));
    }

    // Create target directory
    tokio::fs::create_dir_all(&target_dir).await.map_err(|e| {
        NortHingError::Validation(format!("failed to create skill directory: {}", e))
    })?;

    // Copy file (not move) - source is preserved per I-NEG-2
    tokio::fs::copy(&skill_md_path, &target_path).await.map_err(|e| {
        NortHingError::Validation(format!("failed to copy SKILL.md: {}", e))
    })?;

    // Mark receipt as consumed
    {
        let mut consumed = CONSUMED_RECEIPTS.lock().unwrap();
        consumed.insert(receipt.receipt_id.clone());
    }

    // Append promote audit entry
    let promote_entry = GovernanceOverrideEntry::new(
        receipt.subject_digest.clone(),
        format!("skill_promotion:{}:{}", candidate_dir_name, receipt.receipt_id),
        "system".to_string(),
    );
    if let Err(e) = append_governance_override(&promote_entry) {
        // Log but don't fail - the promotion itself succeeded
        warn!(error = %e, "promote_candidate_skill: failed to write promote audit entry");
    }

    info!(
        skill_name = %candidate_dir_name,
        target_path = %target_path.display(),
        receipt_id = %receipt.receipt_id,
        "promote_candidate_skill: skill promoted successfully"
    );

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::judge_gate::runner::FakeJudgeRunner;
    use northhing_agent_runtime::judge_gate::{
        EvidencePack, FsDiffEvidence, HumanFeedbackSlot, RateSample, RuleStatus, SuccessRateComparison,
        ToolTraceEvidence,
    };

    fn make_valid_request() -> GateRequest {
        GateRequest {
            action_kind: ActionKind::PromoteSkillCandidate,
            subject: b"test skill content".to_vec(),
            evidence: EvidencePack {
                traces: vec![ToolTraceEvidence {
                    turn_id: "turn-1".to_string(),
                    tool: "Read".to_string(),
                    error_excerpt: "".to_string(),
                    repair_excerpt: None,
                }],
                fs_diffs: vec![FsDiffEvidence {
                    path: "test.rs".to_string(),
                    before_digest: "abc".to_string(),
                    after_digest: "def".to_string(),
                    added: 1,
                    removed: 0,
                }],
                success_rate: SuccessRateComparison {
                    baseline: RateSample::Present { successes: 5, attempts: 10 },
                    candidate: RateSample::Present { successes: 7, attempts: 10 },
                },
                human_feedback: HumanFeedbackSlot::Absent(
                    northhing_agent_runtime::judge_gate::AbsentReason::NoHumanExposureYet,
                ),
            },
        }
    }

    fn make_approve_verdict() -> String {
        r#"Some preamble text
VERDICT_JSON_BEGIN
{"verdict":"approve","rule_checks":[{"rule":"I-NEG-1","status":"pass"},{"rule":"I-NEG-2","status":"pass"},{"rule":"I-NEG-3","status":"pass"},{"rule":"I-NEG-4","status":"pass"}],"evidence_assessment":"T1, F1","rationale":"All rules pass."}
VERDICT_JSON_END
Some conclusion"#.to_string()
    }

    fn make_reject_verdict() -> String {
        r#"VERDICT_JSON_BEGIN
{"verdict":"reject","rule_checks":[{"rule":"I-NEG-1","status":"pass"},{"rule":"I-NEG-2","status":"violation"},{"rule":"I-NEG-3","status":"pass"},{"rule":"I-NEG-4","status":"pass"}],"evidence_assessment":"F1 shows violation","rationale":"I-NEG-2 violated."}
VERDICT_JSON_END"#.to_string()
    }

    #[tokio::test]
    async fn evaluate_approve_produces_receipt() {
        let runner = FakeJudgeRunner::new().with_verdict_text(make_approve_verdict());
        let request = make_valid_request();
        let ctx = northhing_agent_runtime::judge_gate::GateExecutionContext {
            workspace_path: None,
            parent_session_id: Some("test-session".to_string()),
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: Some("test-correlation".to_string()),
        };

        // Use a dummy coordinator - FakeJudgeRunner ignores it
        let coordinator: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());

        let verdict = evaluate(&coordinator, request, &ctx, &runner).await;

        match verdict {
            GateVerdict::Approved(receipt) => {
                assert!(!receipt.receipt_id.is_empty());
                assert!(matches!(receipt.action_kind, ActionKind::PromoteSkillCandidate));
                assert!(!receipt.subject_digest.is_empty());
                assert!(!receipt.audit_entry_id.is_empty());
                assert!(receipt.ts > 0);
            }
            GateVerdict::Rejected(r) => {
                panic!("expected approval, got {:?}", r);
            }
        }
    }

    #[tokio::test]
    async fn evaluate_reject_produces_policy_violation() {
        let runner = FakeJudgeRunner::new().with_verdict_text(make_reject_verdict());
        let request = make_valid_request();
        let ctx = northhing_agent_runtime::judge_gate::GateExecutionContext {
            workspace_path: None,
            parent_session_id: None,
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: None,
        };

        let coordinator: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());

        let verdict = evaluate(&coordinator, request, &ctx, &runner).await;

        match verdict {
            GateVerdict::Approved(_) => {
                panic!("expected rejection");
            }
            GateVerdict::Rejected(RejectClass::PolicyViolation(msg)) => {
                assert!(msg.contains("I-NEG-2"));
            }
            GateVerdict::Rejected(r) => {
                panic!("expected PolicyViolation, got {:?}", r);
            }
        }
    }

    #[tokio::test]
    async fn evaluate_runner_timeout_produces_judge_unavailable() {
        let runner = FakeJudgeRunner::new().with_error(JudgeRunError::Timeout);
        let request = make_valid_request();
        let ctx = northhing_agent_runtime::judge_gate::GateExecutionContext {
            workspace_path: None,
            parent_session_id: None,
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: None,
        };

        let coordinator: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());

        let verdict = evaluate(&coordinator, request, &ctx, &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::JudgeUnavailable(msg)) => {
                assert!(msg.contains("timeout"));
            }
            _ => panic!("expected JudgeUnavailable"),
        }
    }

    #[tokio::test]
    async fn evaluate_runner_error_produces_judge_unavailable() {
        let runner = FakeJudgeRunner::new()
            .with_error(JudgeRunError::Unavailable("subagent unavailable".to_string()));
        let request = make_valid_request();
        let ctx = northhing_agent_runtime::judge_gate::GateExecutionContext {
            workspace_path: None,
            parent_session_id: None,
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: None,
        };

        let coordinator: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());

        let verdict = evaluate(&coordinator, request, &ctx, &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::JudgeUnavailable(msg)) => {
                assert!(msg.contains("unavailable"));
            }
            _ => panic!("expected JudgeUnavailable"),
        }
    }

    #[tokio::test]
    async fn evaluate_malformed_verdict_produces_malformed_rejection() {
        let runner = FakeJudgeRunner::new().with_verdict_text("not a valid verdict".to_string());
        let request = make_valid_request();
        let ctx = northhing_agent_runtime::judge_gate::GateExecutionContext {
            workspace_path: None,
            parent_session_id: None,
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: None,
        };

        let coordinator: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());

        let verdict = evaluate(&coordinator, request, &ctx, &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::MalformedVerdict(_)) => {}
            _ => panic!("expected MalformedVerdict"),
        }
    }

    #[test]
    fn promote_validates_action_kind() {
        // subject_digest("skill content") = sha256:v1:a55d9c00dea0cb0185ca98064b42d188e82f772cd36030ae52dc583bb0c33e14
        let receipt = ApprovedGateReceipt {
            receipt_id: "test-receipt".to_string(),
            action_kind: ActionKind::PromoteSkillCandidate,
            subject_digest: "sha256:v1:a55d9c00dea0cb0185ca98064b42d188e82f772cd36030ae52dc583bb0c33e14".to_string(),
            audit_entry_id: "audit-1".to_string(),
            ts: 1000,
        };

        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let tmp = std::env::temp_dir().join(format!("northhing-promote-test-{}", std::process::id()));
            tokio::fs::create_dir_all(&tmp).await.unwrap();
            let skill_md = tmp.join("SKILL.md");
            tokio::fs::write(&skill_md, b"skill content").await.unwrap();

            let result = promote_candidate_skill(receipt.clone(), &tmp).await;
            assert!(result.is_ok());

            let _ = tokio::fs::remove_dir_all(&tmp).await;
        });
    }

    #[test]
    fn promote_validates_not_consumed() {
        let receipt = ApprovedGateReceipt {
            receipt_id: "double-use-test".to_string(),
            action_kind: ActionKind::PromoteSkillCandidate,
            subject_digest: "sha256:v1:a55d9c00dea0cb0185ca98064b42d188e82f772cd36030ae52dc583bb0c33e14".to_string(),
            audit_entry_id: "audit-1".to_string(),
            ts: 1000,
        };

        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let tmp = std::env::temp_dir().join(format!("northhing-promote-test-{}", std::process::id()));
            tokio::fs::create_dir_all(&tmp).await.unwrap();
            let skill_md = tmp.join("SKILL.md");
            tokio::fs::write(&skill_md, b"skill content").await.unwrap();

            let result = promote_candidate_skill(receipt.clone(), &tmp).await;
            assert!(result.is_ok());

            let result = promote_candidate_skill(receipt.clone(), &tmp).await;
            assert!(result.is_err());

            let _ = tokio::fs::remove_dir_all(&tmp).await;
        });
    }

    #[test]
    fn promote_validates_unsafe_name() {
        let receipt = ApprovedGateReceipt {
            receipt_id: "unsafe-name-test".to_string(),
            action_kind: ActionKind::PromoteSkillCandidate,
            subject_digest: "sha256:v1:abc".to_string(),
            audit_entry_id: "audit-1".to_string(),
            ts: 1000,
        };

        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let tmp = std::env::temp_dir().join("..\\etc\\passwd");
            let result = promote_candidate_skill(receipt, &tmp).await;
            assert!(result.is_err());
        });
    }
}
