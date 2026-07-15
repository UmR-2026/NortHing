//! Workspace module group facade
//!
//! Re-exports the public API of the workspace module group.

pub use super::factory::WorkspaceFactory;
pub use super::identity_watch::WorkspaceIdentityWatchService;
pub use super::manager::*;
pub use super::provider::{WorkspaceCleanupResult, WorkspaceProvider, WorkspaceSystemSummary};
pub use super::service::{
    global_workspace_service, set_global_workspace_service, BatchImportResult, BatchRemoveResult,
    WorkspaceCreateOptions, WorkspaceExport, WorkspaceHealthStatus, WorkspaceIdentityChangedEvent,
    WorkspaceImportResult, WorkspaceInfoUpdates, WorkspaceQuickSummary, WorkspaceService,
};
