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
    EvidenceRejection, GateExecutionContext, GateRequest, GateVerdict, ParsedVerdict, RateSample,
    RejectClass, RuleCheck, RuleStatus, VerdictKind, VerdictMalformed,
};
pub use verdict::parse_verdict;
