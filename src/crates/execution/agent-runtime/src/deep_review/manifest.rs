//! Typed Deep Review launch manifest accessors.
//!
//! The frontend builds the launch manifest, but Rust owns defensive parsing
//! and the final trust boundary. Accessors in this module must remain
//! backward compatible with older manifest field spellings and should not
//! silently hide reduced coverage, omitted files, or stale evidence hints.
//!
//! This file is a thin facade. The implementation lives in sibling files
//! (one sub-domain per file, matching the R37c task_execution split pattern):
//!
//! - `scope_profile` — `DeepReviewScopeProfile` (typed view of
//!   `manifest.scopeProfile`)
//! - `evidence_pack` — `DeepReviewEvidencePack` (typed view of
//!   `manifest.evidencePack`, plus budget / privacy / forbidden-key
//!   validation)
//! - `run_manifest_gate` — `DeepReviewRunManifestGate` (active-vs-skipped
//!   subagent accounting + policy gate)
//! - `manifest_helpers` — shared error type and cross-sibling JSON helpers
//!   (owned by this facade; the `deep_review::types` file is owned by the
//!   task-execution facade)
//!
//! Cross-crate consumers (`report.rs`, `mod.rs`) keep importing through
//! `deep_review::manifest::` because this facade re-exports each public
//! item explicitly.

pub use super::evidence_pack::*;
pub use super::manifest_helpers::*;
pub use super::run_manifest_gate::*;
pub use super::scope_profile::*;
