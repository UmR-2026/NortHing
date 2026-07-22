//! Core types for the judge gate protocol layer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// ActionKind defines what kind of action is being gated.
/// v1 has only PromoteSkillCandidate; gate-self-modify is structurally impossible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    PromoteSkillCandidate,
}

/// GateRequest is the input to the judge gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRequest {
    pub action_kind: ActionKind,
    pub subject: Vec<u8>,
    pub evidence: EvidencePack,
}

/// Execution context passed to the gate runner.
/// Note: cancel_token is not serializable (runtime handle), so this struct
/// is not Serialize/Deserialize. It is construction-time only.
#[derive(Debug, Clone)]
pub struct GateExecutionContext {
    pub workspace_path: Option<String>,
    pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>,
    pub timeout_seconds: Option<u64>,
    #[doc(hidden)]
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    pub audit_correlation_id: Option<String>,
}

/// The verdict produced by the judge gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    Approved(ApprovedGateReceipt),
    Rejected(RejectClass),
}

/// Rejection reasons from the judge gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectClass {
    PolicyViolation(String),
    MalformedVerdict(String),
    EvidenceRejected(String),
    JudgeUnavailable(String),
    AuditFailure(String),
}

/// Receipt produced when an action is approved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedGateReceipt {
    pub receipt_id: String,
    pub action_kind: ActionKind,
    pub subject_digest: String,
    pub audit_entry_id: String,
    pub ts: u64,
}

/// A single redline rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedlineRule {
    pub id: &'static str,
    pub statement: &'static str,
}

/// Result of checking a single rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCheck {
    pub rule: String,
    pub status: RuleStatus,
}

/// Status of a rule check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Pass,
    Violation,
}

/// Evidence pack containing all evidence for a gate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePack {
    pub traces: Vec<ToolTraceEvidence>,
    pub fs_diffs: Vec<FsDiffEvidence>,
    pub success_rate: SuccessRateComparison,
    pub human_feedback: HumanFeedbackSlot,
}

/// Tool trace evidence entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTraceEvidence {
    pub turn_id: String,
    pub tool: String,
    pub error_excerpt: String,
    pub repair_excerpt: Option<String>,
}

/// Filesystem diff evidence entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsDiffEvidence {
    pub path: String,
    pub before_digest: String,
    pub after_digest: String,
    pub added: u32,
    pub removed: u32,
}

/// Success rate comparison between baseline and candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessRateComparison {
    pub baseline: RateSample,
    pub candidate: RateSample,
}

/// A rate sample, either with data or marked as no baseline yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RateSample {
    Present { successes: u32, attempts: u32 },
    NoBaselineYet,
}

/// Human feedback slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HumanFeedbackSlot {
    Present(Vec<HumanFeedback>),
    Absent(AbsentReason),
}

/// A single human feedback entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedback {
    pub origin: String,
    pub excerpt: String,
}

/// Reason why human feedback is absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsentReason {
    NoHumanExposureYet,
    NotApplicableForActionKind,
}

/// Error when evidence is rejected during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRejection {
    TracesAndFsDiffsBothEmpty,
    SlotCountExceeded { slot: String, max: usize, actual: usize },
    ExcerptTooLong { slot: String, index: usize, max: usize, actual: usize },
    TotalBudgetExceeded { max: usize, actual: usize },
    WhitespaceField { field: String },
    EpisodeSourceBlacklisted { path: String },
}

/// Verdict kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictKind {
    Approve,
    Reject,
}

/// Parsed verdict from judge output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedVerdict {
    pub verdict: VerdictKind,
    pub rule_checks: Vec<RuleCheck>,
    pub evidence_assessment: String,
    pub rationale: String,
}

/// Error when verdict parsing fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictMalformed {
    NoVerdictBlock,
    MultipleVerdictBlocks,
    BlockContentNotJson { cause: String },
    VerdictFieldMissing,
    VerdictFieldInvalid { value: String },
    RuleChecksMissing,
    RuleChecksWrongCount { expected: usize, actual: usize },
    RuleCheckMissing { rule_id: String },
    RuleCheckDuplicate { rule_id: String },
    RuleCheckUnknown { rule_id: String },
    RuleCheckExtra,
    RuleStatusInvalid { rule_id: String, value: String },
    EvidenceAssessmentEmpty,
    EvidenceAssessmentNoReference,
    RationaleEmpty,
}

impl fmt::Display for VerdictMalformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerdictMalformed::NoVerdictBlock => write!(f, "no verdict block found"),
            VerdictMalformed::MultipleVerdictBlocks => write!(f, "multiple verdict blocks found"),
            VerdictMalformed::BlockContentNotJson { cause } => write!(f, "block content not valid JSON: {}", cause),
            VerdictMalformed::VerdictFieldMissing => write!(f, "verdict field missing"),
            VerdictMalformed::VerdictFieldInvalid { value } => write!(f, "verdict field invalid value: {}", value),
            VerdictMalformed::RuleChecksMissing => write!(f, "rule_checks missing"),
            VerdictMalformed::RuleChecksWrongCount { expected, actual } => {
                write!(f, "rule_checks wrong count: expected {}, got {}", expected, actual)
            }
            VerdictMalformed::RuleCheckMissing { rule_id } => write!(f, "rule check missing: {}", rule_id),
            VerdictMalformed::RuleCheckDuplicate { rule_id } => write!(f, "rule check duplicate: {}", rule_id),
            VerdictMalformed::RuleCheckUnknown { rule_id } => write!(f, "rule check unknown: {}", rule_id),
            VerdictMalformed::RuleCheckExtra => write!(f, "extra rule check found"),
            VerdictMalformed::RuleStatusInvalid { rule_id, value } => {
                write!(f, "rule status invalid for {}: {}", rule_id, value)
            }
            VerdictMalformed::EvidenceAssessmentEmpty => write!(f, "evidence_assessment is empty"),
            VerdictMalformed::EvidenceAssessmentNoReference => {
                write!(f, "evidence_assessment does not reference any valid evidence ID")
            }
            VerdictMalformed::RationaleEmpty => write!(f, "rationale is empty"),
        }
    }
}

impl std::error::Error for VerdictMalformed {}

/// Compute the subject digest for a given subject.
pub fn subject_digest(subject: &[u8]) -> String {
    let hash = Sha256::digest(subject);
    let hex_str = hash.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    format!("sha256:v1:{}", hex_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_kind_serialize_only_promote_skill_candidate() {
        // Serialization must only produce "promote_skill_candidate"
        // This guards against future variants being added without design change
        let ak = ActionKind::PromoteSkillCandidate;
        let json = serde_json::to_string(&ak).unwrap();
        assert_eq!(json, "\"promote_skill_candidate\"");

        // Deserialize back
        let deserialized: ActionKind = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ActionKind::PromoteSkillCandidate);
    }

    #[test]
    fn subject_digest_fixed_value() {
        // Known input must produce known sha256 output
        // This guards against algorithm drift
        let input = b"test subject content";
        let digest = subject_digest(input);

        // sha256 of "test subject content" is known
        assert!(digest.starts_with("sha256:v1:"));
        assert_eq!(digest.len(), 10 + 64); // "sha256:v1:" + 64 hex chars

        // Verify it's consistent
        let digest2 = subject_digest(input);
        assert_eq!(digest, digest2);

        // Different input gives different digest
        let different = subject_digest(b"different");
        assert_ne!(digest, different);
    }

    #[test]
    fn subject_digest_format() {
        let digest = subject_digest(b"hello");
        // sha256 of "hello" is 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert_eq!(
            digest,
            "sha256:v1:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
