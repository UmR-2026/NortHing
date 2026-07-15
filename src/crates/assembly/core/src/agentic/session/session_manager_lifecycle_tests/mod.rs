//! Round 9b split: session_manager_lifecycle_tests facade
//!
//! Test fns moved to 4 sibling files (concern-grouped):
//!   - session_manager_lifecycle_tests_session_state_reset.rs
//!   - session_manager_lifecycle_tests_ephemeral_lineage.rs
//!   - session_manager_lifecycle_tests_restore_dialog.rs
//!   - session_manager_lifecycle_tests_rollback_delete.rs
//!
//! Helpers (TestWorkspace, test_manager, etc.) live in session_manager_tests.rs
//! and are imported via super::super::{...}.

#[cfg(test)]
pub use super::*;

#[cfg(test)]
mod session_manager_lifecycle_tests_ephemeral_lineage;
#[cfg(test)]
mod session_manager_lifecycle_tests_restore_dialog;
#[cfg(test)]
mod session_manager_lifecycle_tests_rollback_delete;
#[cfg(test)]
mod session_manager_lifecycle_tests_session_state_reset;
