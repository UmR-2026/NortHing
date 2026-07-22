//! Judge gate protocol layer - pure types, no async/IO.
//!
//! This module implements the pure protocol layer for the judge gate
//! as specified in C4 Phase 0 design §5.1.

pub mod brief;
pub mod evidence;
pub mod redlines;
pub mod types;
pub mod verdict;

// Re-export commonly used types
pub use brief::build_judge_brief;
pub use redlines::{redline_ids, REDLINE_TABLE};
pub use types::{
    subject_digest, ActionKind, ApprovedGateReceipt, EvidencePack,
    EvidenceRejection, FsDiffEvidence, GateExecutionContext, GateRequest, GateVerdict,
    HumanFeedbackSlot, ParsedVerdict, RateSample, RejectClass, RuleCheck, RuleStatus,
    SuccessRateComparison, ToolTraceEvidence, VerdictKind, VerdictMalformed, AbsentReason,
};
pub use verdict::parse_verdict;
