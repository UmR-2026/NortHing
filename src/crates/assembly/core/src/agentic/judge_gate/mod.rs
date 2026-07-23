// allow-god-file: 922L — C4 Phase 0 newly created; split deferred to C4 Phase 1 design
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

use crate::agentic::coordination::ConversationCoordinator;
use crate::agentic::judge_gate::audit::{append_audit_entry, AuditEntry};
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

/// Pre-audit verdict types: all outcomes before audit writing.
#[derive(Debug)]
enum PreAuditVerdict {
    Approved {
        rule_checks: Vec<RuleCheck>,
    },
    Rejected {
        rejection: RejectClass,
        reject_class: Option<String>,
        rule_checks: Vec<RuleCheck>,
    },
}

/// Evaluate a gate request.
///
/// # Flow
/// 1. Validate evidence pack → PreAuditVerdict::Rejected(EvidenceRejected)
/// 2. Build judge brief
/// 3. Run judge via runner → PreAuditVerdict::Rejected(JudgeUnavailable) on any error
/// 4. Parse verdict → PreAuditVerdict::Rejected(MalformedVerdict) on parse failure
/// 5. Check all 4 rules → PreAuditVerdict::Approved or PreAuditVerdict::Rejected(PolicyViolation)
/// 6. Write audit entry for the pre-audit verdict → AuditFailure on write failure
/// 7. Only if audit success AND PreAuditVerdict::Approved → construct ApprovedGateReceipt
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
    coordinator: &Arc<ConversationCoordinator>,
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
        warn!(error = ?evidence_error, "evaluate: evidence validation failed");
        let verdict = PreAuditVerdict::Rejected {
            rejection: RejectClass::EvidenceRejected(format!("{:?}", evidence_error)),
            reject_class: Some("evidence_rejected".to_string()),
            rule_checks: vec![],
        };
        return write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time);
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
            let verdict = PreAuditVerdict::Rejected {
                rejection: RejectClass::JudgeUnavailable("timeout".to_string()),
                reject_class: Some("judge_unavailable".to_string()),
                rule_checks: vec![],
            };
            return write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time);
        }
        Err(JudgeRunError::Cancelled) => {
            warn!("evaluate: judge runner cancelled");
            let verdict = PreAuditVerdict::Rejected {
                rejection: RejectClass::JudgeUnavailable("cancelled".to_string()),
                reject_class: Some("judge_unavailable".to_string()),
                rule_checks: vec![],
            };
            return write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time);
        }
        Err(JudgeRunError::Unavailable(reason)) => {
            warn!(reason = %reason, "evaluate: judge runner unavailable");
            let verdict = PreAuditVerdict::Rejected {
                rejection: RejectClass::JudgeUnavailable(reason),
                reject_class: Some("judge_unavailable".to_string()),
                rule_checks: vec![],
            };
            return write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time);
        }
    };

    // Step 4: Parse verdict
    let valid_evidence_ids = request.evidence.evidence_ids();
    let parsed = match parse_verdict(&judge_output, &valid_evidence_ids) {
        Ok(p) => p,
        Err(parse_error) => {
            warn!(error = %parse_error, "evaluate: verdict parsing failed");
            let verdict = PreAuditVerdict::Rejected {
                rejection: RejectClass::MalformedVerdict(parse_error.to_string()),
                reject_class: Some("malformed_verdict".to_string()),
                rule_checks: vec![],
            };
            return write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time);
        }
    };

    // Step 5: Check all rules pass for approval
    let all_pass = parsed.rule_checks.iter().all(|rc| rc.status == RuleStatus::Pass);
    let rule_checks: Vec<RuleCheck> = parsed
        .rule_checks
        .iter()
        .map(|rc| RuleCheck {
            rule: rc.rule.clone(),
            status: rc.status.clone(),
        })
        .collect();

    let verdict = if all_pass {
        PreAuditVerdict::Approved { rule_checks }
    } else {
        let violation_rules: Vec<String> = parsed
            .rule_checks
            .iter()
            .filter(|rc| rc.status == RuleStatus::Violation)
            .map(|rc| rc.rule.clone())
            .collect();
        warn!(violations = ?violation_rules, "evaluate: gate rejected due to policy violation");
        PreAuditVerdict::Rejected {
            rejection: RejectClass::PolicyViolation(violation_rules.join(", ")),
            reject_class: Some("policy_violation".to_string()),
            rule_checks,
        }
    };

    // Step 6 + 7: Write audit, then finalize (only approved + audit success → receipt)
    write_audit_and_finalize(verdict, request.action_kind, subject_digest, start_time)
}

/// Write audit entry for the pre-audit verdict, then finalize.
///
/// All terminal states write audit. Audit failure → AuditFailure (no receipt).
/// Only audit success + PreAuditVerdict::Approved → produces receipt.
fn write_audit_and_finalize(
    verdict: PreAuditVerdict,
    action_kind: ActionKind,
    subject_digest: String,
    start_time: std::time::Instant,
) -> GateVerdict {
    let (verdict_str, rule_checks, reject_class) = match &verdict {
        PreAuditVerdict::Approved { rule_checks } => ("approve".to_string(), rule_checks.clone(), None),
        PreAuditVerdict::Rejected { rule_checks, reject_class, .. } => {
            ("reject".to_string(), rule_checks.clone(), reject_class.clone())
        }
    };

    let audit_entry = AuditEntry::new(
        action_kind.clone(),
        subject_digest.clone(),
        format!("elapsed_ms:{}", start_time.elapsed().as_millis()),
        verdict_str,
        rule_checks,
        reject_class,
        None,
        Some(start_time.elapsed().as_millis() as u64),
    );

    if let Err(audit_err) = append_audit_entry(&audit_entry) {
        error!(error = %audit_err, "evaluate: audit write failed - returning AuditFailure (no receipt)");
        return GateVerdict::Rejected(RejectClass::AuditFailure(format!(
            "audit write failed: {}",
            audit_err
        )));
    }

    match verdict {
        PreAuditVerdict::Approved { .. } => {
            let receipt = ApprovedGateReceipt {
                receipt_id: Uuid::new_v4().to_string(),
                action_kind,
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
        }
        PreAuditVerdict::Rejected { rejection, .. } => GateVerdict::Rejected(rejection),
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
    let skills_root = path_manager_arc().user_skills_dir();
    promote_candidate_skill_to(receipt, candidate_dir, &skills_root).await
}

/// Test-seam variant of `promote_candidate_skill` with an explicit skills root,
/// so tests never touch the real user skills directory.
async fn promote_candidate_skill_to(
    receipt: ApprovedGateReceipt,
    candidate_dir: &Path,
    skills_root: &Path,
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
        || candidate_dir_name.to_lowercase() == "candidates"
        || !candidate_dir_name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        error!(name = %candidate_dir_name, "promote_candidate_skill: unsafe candidate name");
        return Err(NortHingError::Validation(format!(
            "unsafe candidate directory name: {}",
            candidate_dir_name
        )));
    }

    // Determine target path
    let target_dir = skills_root.join(&candidate_dir_name);
    let target_path = target_dir.join(SKILL_MD_FILENAME);

    // Atomic consumed check+mark: merge into single lock acquisition
    // On any subsequent validation failure, we release the mark (see cleanup below)
    let consumed_marked = {
        match CONSUMED_RECEIPTS.lock() {
            Ok(mut consumed) => {
                if consumed.contains(&receipt.receipt_id) {
                    error!(receipt_id = %receipt.receipt_id, "promote_candidate_skill: receipt already consumed");
                    return Err(NortHingError::Validation(format!(
                        "receipt {} already consumed",
                        receipt.receipt_id
                    )));
                }
                consumed.insert(receipt.receipt_id.clone());
                true
            }
            Err(poisoned) => {
                // Poisoned mutex - recover and treat as locked
                error!("consumed receipts mutex poisoned: {:?}", poisoned);
                return Err(NortHingError::Validation("consumed receipts lock poisoned".to_string()));
            }
        }
    };

    // Write promote audit entry BEFORE copying file (per I-NEG-4 enforcement)
    let promote_entry = AuditEntry::new(
        ActionKind::PromoteSkillCandidate,
        receipt.subject_digest.clone(),
        format!("promotion:{}:{}", candidate_dir_name, receipt.receipt_id),
        "promote".to_string(),
        vec![],
        None,
        None,
        None,
    );
    // Audit must succeed before we consider promotion valid
    if let Err(e) = append_audit_entry(&promote_entry) {
        error!(error = %e, "promote_candidate_skill: failed to write promote audit entry");
        // Release consumed mark since promotion failed
        if consumed_marked {
            if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                consumed.remove(&receipt.receipt_id);
            }
        }
        return Err(NortHingError::Validation(format!(
            "promotion audit write failed: {}",
            e
        )));
    }

    // Create target directory
    if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
        // Release consumed mark since promotion failed
        if consumed_marked {
            if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                consumed.remove(&receipt.receipt_id);
            }
        }
        return Err(NortHingError::Validation(format!(
            "failed to create skill directory: {}",
            e
        )));
    }

    // Atomic file copy: use create_new to prevent TOCTOU
    // This atomically checks file existence and creates, eliminating the race window
    let mut file = match tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&target_path)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            // Release consumed mark since promotion failed
            if consumed_marked {
                if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                    consumed.remove(&receipt.receipt_id);
                }
            }
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(NortHingError::Validation(format!(
                    "skill {} already exists at {}",
                    candidate_dir_name,
                    target_path.display()
                )));
            }
            return Err(NortHingError::Validation(format!(
                "failed to create skill file: {}",
                e
            )));
        }
    };

    // Copy content (source preserved per I-NEG-2)
    let skill_content = match tokio::fs::read(&skill_md_path).await {
        Ok(c) => c,
        Err(e) => {
            // Release consumed mark since promotion failed
            if consumed_marked {
                if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                    consumed.remove(&receipt.receipt_id);
                }
            }
            return Err(NortHingError::Validation(format!("failed to read SKILL.md: {}", e)));
        }
    };

    use tokio::io::AsyncWriteExt;
    if let Err(e) = file.write_all(&skill_content).await {
        // Release consumed mark since promotion failed
        if consumed_marked {
            if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                consumed.remove(&receipt.receipt_id);
            }
        }
        return Err(NortHingError::Validation(format!("failed to write SKILL.md: {}", e)));
    }

    if let Err(e) = file.flush().await {
        // Release consumed mark since promotion failed
        if consumed_marked {
            if let Ok(mut consumed) = CONSUMED_RECEIPTS.lock() {
                consumed.remove(&receipt.receipt_id);
            }
        }
        return Err(NortHingError::Validation(format!("failed to flush SKILL.md: {}", e)));
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
    use crate::agentic::coordination::tests::build_isolated_coordinator;
    use crate::agentic::judge_gate::audit::{set_audit_dir_override_for_tests, today_audit_path, TEST_ENV_LOCK};
    use crate::agentic::judge_gate::runner::FakeJudgeRunner;
    use northhing_agent_runtime::judge_gate::{
        AbsentReason, EvidencePack, FsDiffEvidence, GateExecutionContext, HumanFeedbackSlot, RateSample,
        SuccessRateComparison, ToolTraceEvidence,
    };

    fn unique_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("northhing-judge-gate-test-{}-{}", tag, Uuid::new_v4()))
    }

    fn test_ctx() -> GateExecutionContext {
        GateExecutionContext {
            workspace_path: None,
            parent_session_id: None,
            parent_turn_id: None,
            timeout_seconds: Some(60),
            cancel_token: None,
            audit_correlation_id: None,
        }
    }

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
                human_feedback: HumanFeedbackSlot::Absent(AbsentReason::NoHumanExposureYet),
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

    fn make_receipt_for(content: &[u8]) -> ApprovedGateReceipt {
        ApprovedGateReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_kind: ActionKind::PromoteSkillCandidate,
            subject_digest: subject_digest(content),
            audit_entry_id: "audit-test".to_string(),
            ts: 1000,
        }
    }

    async fn write_candidate(dir: &Path, content: &[u8]) {
        tokio::fs::create_dir_all(dir).await.unwrap();
        tokio::fs::write(dir.join(SKILL_MD_FILENAME), content).await.unwrap();
    }

    fn read_audit_lines() -> Vec<serde_json::Value> {
        let path = today_audit_path();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("audit line must be valid JSON"))
            .collect()
    }

    #[tokio::test]
    async fn evaluate_approve_produces_receipt_and_audit() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-approve");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_verdict_text(make_approve_verdict());
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        let GateVerdict::Approved(receipt) = verdict else {
            panic!("expected approval, got {:?}", verdict);
        };
        assert!(!receipt.receipt_id.is_empty());
        assert!(matches!(receipt.action_kind, ActionKind::PromoteSkillCandidate));
        assert_eq!(receipt.subject_digest, subject_digest(b"test skill content"));
        assert!(!receipt.audit_entry_id.is_empty());
        assert!(receipt.ts > 0);

        let lines = read_audit_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["verdict"], "approve");
        assert_eq!(lines[0]["entry_id"].as_str().unwrap(), receipt.audit_entry_id);

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_reject_produces_policy_violation_and_audit() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-reject");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_verdict_text(make_reject_verdict());
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::PolicyViolation(msg)) => {
                assert!(msg.contains("I-NEG-2"));
            }
            other => panic!("expected PolicyViolation, got {:?}", other),
        }

        let lines = read_audit_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["verdict"], "reject");
        assert_eq!(lines[0]["reject_class"], "policy_violation");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_runner_timeout_produces_judge_unavailable_and_audit() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-timeout");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_error(JudgeRunError::Timeout);
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::JudgeUnavailable(msg)) => assert!(msg.contains("timeout")),
            other => panic!("expected JudgeUnavailable, got {:?}", other),
        }

        let lines = read_audit_lines();
        assert_eq!(lines.len(), 1, "runner failures must also be audited");
        assert_eq!(lines[0]["verdict"], "reject");
        assert_eq!(lines[0]["reject_class"], "judge_unavailable");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_runner_cancelled_produces_judge_unavailable() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-cancelled");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_error(JudgeRunError::Cancelled);
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::JudgeUnavailable(msg)) => assert!(msg.contains("cancelled")),
            other => panic!("expected JudgeUnavailable(cancelled), got {:?}", other),
        }

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_runner_error_produces_judge_unavailable() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-error");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_error(JudgeRunError::Unavailable("subagent unavailable".to_string()));
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::JudgeUnavailable(msg)) => assert!(msg.contains("unavailable")),
            other => panic!("expected JudgeUnavailable, got {:?}", other),
        }

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_malformed_verdict_produces_malformed_rejection() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-malformed");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let runner = FakeJudgeRunner::new().with_verdict_text("not a valid verdict".to_string());
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        assert!(matches!(verdict, GateVerdict::Rejected(RejectClass::MalformedVerdict(_))));

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_invalid_evidence_produces_evidence_rejected() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = unique_dir("eval-evidence");
        set_audit_dir_override_for_tests(Some(dir.clone()));

        let mut request = make_valid_request();
        request.evidence.traces.clear();
        request.evidence.fs_diffs.clear();

        let runner = FakeJudgeRunner::new().with_verdict_text(make_approve_verdict());
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, request, &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::EvidenceRejected(_)) => {}
            other => panic!("expected EvidenceRejected, got {:?}", other),
        }

        let lines = read_audit_lines();
        assert_eq!(lines.len(), 1, "evidence rejection must be audited");
        assert_eq!(lines[0]["reject_class"], "evidence_rejected");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn evaluate_audit_failure_yields_no_receipt() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Point the audit dir at an existing FILE so the append fails.
        let blocked = unique_dir("eval-audit-blocked");
        std::fs::write(&blocked, b"not a directory").unwrap();
        set_audit_dir_override_for_tests(Some(blocked.clone()));

        let runner = FakeJudgeRunner::new().with_verdict_text(make_approve_verdict());
        let (coordinator, _session_manager) = build_isolated_coordinator();
        let verdict = evaluate(&coordinator, make_valid_request(), &test_ctx(), &runner).await;

        match verdict {
            GateVerdict::Rejected(RejectClass::AuditFailure(_)) => {}
            GateVerdict::Approved(_) => panic!("audit failure must never produce a receipt"),
            other => panic!("expected AuditFailure, got {:?}", other),
        }

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_file(&blocked);
    }

    #[tokio::test]
    async fn promote_happy_path_writes_identical_content_source_retained_and_audit() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let audit_dir = unique_dir("promote-audit");
        set_audit_dir_override_for_tests(Some(audit_dir.clone()));

        let content = b"skill content";
        let candidate = unique_dir("promote-src").join("good-skill");
        write_candidate(&candidate, content).await;
        let skills_root = unique_dir("promote-skills");
        let receipt = make_receipt_for(content);
        let receipt_id = receipt.receipt_id.clone();

        let result = promote_candidate_skill_to(receipt.clone(), &candidate, &skills_root).await;
        let target = result.expect("promotion should succeed");

        let written = tokio::fs::read(&target).await.expect("target should exist");
        assert_eq!(written, content, "promoted content must be byte-identical");
        assert!(candidate.join(SKILL_MD_FILENAME).exists(), "candidate source must be retained");

        // Receipt is now consumed: a second promote with the same receipt must fail.
        let second = promote_candidate_skill_to(receipt, &candidate, &skills_root).await;
        assert!(second.is_err(), "consumed receipt must not be reusable");

        let lines = read_audit_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["verdict"], "promote");
        assert_eq!(lines[0]["evidence_summary"].as_str().unwrap().contains("good-skill"), true);

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&audit_dir);
        let _ = std::fs::remove_dir_all(unique_dir("promote-src"));
        let _ = std::fs::remove_dir_all(&skills_root);
        let _ = receipt_id;
    }

    #[tokio::test]
    async fn promote_rejects_wrong_digest() {
        let candidate = unique_dir("promote-wrong-digest").join("some-skill");
        write_candidate(&candidate, b"actual content").await;
        let skills_root = unique_dir("promote-skills");
        let mut receipt = make_receipt_for(b"actual content");
        receipt.subject_digest = "sha256:v1:0000000000000000000000000000000000000000000000000000000000000000".to_string();

        let result = promote_candidate_skill_to(receipt, &candidate, &skills_root).await;
        assert!(result.is_err(), "digest mismatch must be rejected");
        assert!(!skills_root.join("some-skill").exists());
    }

    #[tokio::test]
    async fn promote_rejects_reserved_name_candidates() {
        let content = b"skill content";
        let skills_root = unique_dir("promote-skills");

        for reserved in ["candidates", "CANDIDATES"] {
            let base = unique_dir("promote-reserved");
            let candidate = base.join(reserved);
            write_candidate(&candidate, content).await;
            let receipt = make_receipt_for(content);

            let result = promote_candidate_skill_to(receipt, &candidate, &skills_root).await;
            assert!(result.is_err(), "reserved name {} must be rejected", reserved);
            assert!(
                !skills_root.join(reserved).exists(),
                "nothing may be written to the loader-visible candidates root"
            );
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[tokio::test]
    async fn promote_rejects_unsafe_name() {
        let content = b"skill content";
        let skills_root = unique_dir("promote-skills");

        for bad_name in ["bad..name", "BadCase", "with space"] {
            let base = unique_dir("promote-unsafe");
            let candidate = base.join(bad_name);
            write_candidate(&candidate, content).await;
            let receipt = make_receipt_for(content);

            let result = promote_candidate_skill_to(receipt, &candidate, &skills_root).await;
            assert!(result.is_err(), "unsafe name {} must be rejected", bad_name);
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[tokio::test]
    async fn promote_rejects_existing_target() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let audit_dir = unique_dir("promote-audit");
        set_audit_dir_override_for_tests(Some(audit_dir.clone()));

        let content = b"skill content";
        let candidate = unique_dir("promote-src").join("taken-skill");
        write_candidate(&candidate, content).await;
        let skills_root = unique_dir("promote-skills");
        // Pre-create the target so the atomic create_new must fail.
        let taken_dir = skills_root.join("taken-skill");
        tokio::fs::create_dir_all(&taken_dir).await.unwrap();
        tokio::fs::write(taken_dir.join(SKILL_MD_FILENAME), b"other").await.unwrap();

        let result = promote_candidate_skill_to(make_receipt_for(content), &candidate, &skills_root).await;
        assert!(result.is_err(), "existing target must not be overwritten");
        let existing = tokio::fs::read(taken_dir.join(SKILL_MD_FILENAME)).await.unwrap();
        assert_eq!(existing, b"other", "existing target content must be untouched");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&audit_dir);
        let _ = std::fs::remove_dir_all(&skills_root);
    }

    #[tokio::test]
    async fn promote_concurrent_same_receipt_only_one_succeeds() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let audit_dir = unique_dir("promote-audit");
        set_audit_dir_override_for_tests(Some(audit_dir.clone()));

        let content: &'static [u8] = b"skill content";
        let candidate = unique_dir("promote-src").join("race-skill");
        write_candidate(&candidate, content).await;
        let skills_root = unique_dir("promote-skills");
        let receipt = make_receipt_for(content);

        let first = {
            let receipt = receipt.clone();
            let candidate = candidate.clone();
            let skills_root = skills_root.clone();
            tokio::spawn(async move { promote_candidate_skill_to(receipt, &candidate, &skills_root).await })
        };
        let second = {
            let receipt = receipt.clone();
            let candidate = candidate.clone();
            let skills_root = skills_root.clone();
            tokio::spawn(async move { promote_candidate_skill_to(receipt, &candidate, &skills_root).await })
        };
        let (first, second) = tokio::join!(first, second);
        let successes = [first.unwrap(), second.unwrap()].into_iter().filter(|r| r.is_ok()).count();
        assert_eq!(successes, 1, "exactly one concurrent promote may succeed");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_dir_all(&audit_dir);
        let _ = std::fs::remove_dir_all(&skills_root);
    }

    #[tokio::test]
    async fn promote_rejects_when_audit_fails_and_releases_consumed_mark() {
        let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let blocked = unique_dir("promote-audit-blocked");
        std::fs::write(&blocked, b"not a directory").unwrap();
        set_audit_dir_override_for_tests(Some(blocked.clone()));

        let content = b"skill content";
        let candidate = unique_dir("promote-src").join("audit-skill");
        write_candidate(&candidate, content).await;
        let skills_root = unique_dir("promote-skills");
        let receipt = make_receipt_for(content);

        let result = promote_candidate_skill_to(receipt.clone(), &candidate, &skills_root).await;
        assert!(result.is_err(), "audit failure must reject the promotion");
        assert!(!skills_root.join("audit-skill").exists(), "no unaudited loader-visible write may remain");

        // The consumed mark must have been released: retry with a working audit dir succeeds.
        let audit_dir = unique_dir("promote-audit");
        set_audit_dir_override_for_tests(Some(audit_dir.clone()));
        let retry = promote_candidate_skill_to(receipt, &candidate, &skills_root).await;
        assert!(retry.is_ok(), "released consumed mark must allow retry");

        set_audit_dir_override_for_tests(None);
        let _ = std::fs::remove_file(&blocked);
        let _ = std::fs::remove_dir_all(&audit_dir);
        let _ = std::fs::remove_dir_all(&skills_root);
    }
}
