//! Session restore: restore_session family + rollback + view restore
//!
//! (R49b) Split into sibling modules (declared in mod.rs):
//! - restore_load:    view restore family (restore_session_view* + internal)
//! - restore_apply:   full restore family (restore_session* + with_turns* + internal)
//! - restore_validate: rollback_context_to_turn_start
//!
//! Re-exports SessionViewRestoreTiming to preserve the
//! `crate::agentic::session::session_manager::SessionViewRestoreTiming`
//! path that dialog_turn.rs depends on.

pub use northhing_runtime_ports::SessionViewRestoreTiming;
