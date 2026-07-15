//! Round 9a split: session_metadata tests facade
//!
//! Test fns moved to 4 sibling files (concern-grouped, not 1:1 with production siblings):
//!   - session_manager_metadata_tests_subagent_metadata.rs
//!   - session_manager_metadata_tests_skill_agent_baseline.rs
//!   - session_manager_metadata_tests_session_view_restore.rs
//!   - session_manager_metadata_tests_prompt_cache_persistence.rs
//!
//! This facade uses `#[path = ...]` to keep files in `src/agentic/session/`
//! (reducing import-path depth vs. the nested-directory alternative).
//!
//! Helpers (TestWorkspace, test_manager, etc.) live in session_manager_tests.rs
//! (the parent facade, 157 canonical lines). Sibling files access them via
//! `super::super::{...}` (TWO super::) and lib SessionManager via
//! `super::super::super::session_manager::SessionManager` (THREE super::).

#![cfg(test)]
#![allow(unused_imports)]

#[cfg(test)]
#[path = "session_manager_metadata_tests_subagent_metadata.rs"]
mod subagent_metadata;

#[cfg(test)]
#[path = "session_manager_metadata_tests_skill_agent_baseline.rs"]
mod skill_agent_baseline;

#[cfg(test)]
#[path = "session_manager_metadata_tests_session_view_restore.rs"]
mod session_view_restore;

#[cfg(test)]
#[path = "session_manager_metadata_tests_prompt_cache_persistence.rs"]
mod prompt_cache_persistence;
