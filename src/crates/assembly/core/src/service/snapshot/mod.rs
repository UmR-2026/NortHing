pub mod baseline_cache;
pub mod events;
pub mod file_lock_manager;
pub mod isolation_manager;
pub mod manager;
pub mod service;
pub mod snapshot_core;
pub mod snapshot_system;
pub mod snapshot_system_helpers;
pub mod types;

// R46c split siblings — declared at the `snapshot/` parent level so each
// sibling resolves as a peer of `manager.rs`, not as a child of it.
// Visibility from siblings into the facade is `super::manager::SnapshotManager`
// (the same convention used by R42c for `snapshot_core.rs` and by R33 for
// `snapshot_system.rs`). Free functions live in their owning sibling and
// are re-exported from `manager.rs` so the existing
// `crate::service::snapshot::manager::X` import paths keep working.
mod manager_capture;
mod manager_invalidate;
mod manager_lock;
mod manager_query;
mod manager_registry;
mod manager_wrapped;

pub use events::{
    emit_snapshot_event, emit_snapshot_session_event, initialize_snapshot_event_emitter, SnapshotEvent,
    SnapshotEventEmitter,
};
pub use manager::{
    ensure_snapshot_manager_for_workspace, get_or_create_snapshot_manager, get_snapshot_manager_for_workspace,
    initialize_snapshot_manager_for_workspace, snapshot_wrapped_tools, wrap_tool_for_snapshot_tracking,
    SnapshotManager,
};
pub use service::{SnapshotService, SystemStats};
pub use snapshot_core::{FileChangeEntry, FileChangeQueue, SessionStats, SnapshotCore};
pub use types::*;
