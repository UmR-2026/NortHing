//! Session Management Layer
//!
//! Provides session lifecycle management and context management.

pub(crate) mod compression;
pub(crate) mod context_store;
pub(crate) mod ev_collect;
pub(crate) mod ev_listing;
pub(crate) mod ev_reconcile;
pub(crate) mod ev_snapshot;
pub(crate) mod evidence_ledger;
pub(crate) mod file_read_state;
pub(crate) mod prompt_cache;
pub(crate) mod restore_apply;
pub(crate) mod restore_load;
pub(crate) mod restore_validate;
pub(crate) mod session_evidence;
pub(crate) mod session_manager;
pub(crate) mod session_manager_auto_save_cleanup;
pub(crate) mod session_manager_lifecycle;
pub(crate) mod session_manager_metadata;
pub(crate) mod session_manager_model_selection;
pub(crate) mod session_manager_persistence_predicate;
pub(crate) mod session_manager_tests;
pub(crate) mod session_manager_titles;
pub(crate) mod session_manager_workspace_path;
pub(crate) mod session_persistence;
pub(crate) mod session_restore;
pub(crate) mod session_store_port;
pub(crate) mod turn_skill_agent_snapshot_store;

mod facade;

pub use facade::*;
