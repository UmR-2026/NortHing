//! Public DTOs and persistence payloads for [`super::WorkspaceService`].
//!
//! These types are part of the workspace service's public API surface.
//! They live in their own module so the facade can stay small and so the
//! type definitions are easy to locate when reading or evolving them.

use super::manager::{RelatedPath, WorkspaceIdentity, WorkspaceInfo, WorkspaceSummary, WorkspaceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Workspace creation options.
#[derive(Debug, Clone)]
pub struct WorkspaceCreateOptions {
    pub scan_options: super::manager::ScanOptions,
    pub auto_set_current: bool,
    pub add_to_recent: bool,
    pub workspace_kind: super::manager::WorkspaceKind,
    pub assistant_id: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    /// See [`crate::service::workspace::manager::WorkspaceOpenOptions::remote_connection_id`].
    pub remote_connection_id: Option<String>,
    /// SSH `host` from connection config; used for `~/.northhing/remote_ssh/...` and stable remote ids.
    pub remote_ssh_host: Option<String>,
    /// Deterministic id for [`super::manager::WorkspaceKind::Remote`] (host + remote path hash).
    pub stable_workspace_id: Option<String>,
}

impl Default for WorkspaceCreateOptions {
    fn default() -> Self {
        Self {
            scan_options: super::manager::ScanOptions::default(),
            auto_set_current: true,
            add_to_recent: true,
            workspace_kind: super::manager::WorkspaceKind::Normal,
            assistant_id: None,
            display_name: None,
            description: None,
            tags: Vec::new(),
            remote_connection_id: None,
            remote_ssh_host: None,
            stable_workspace_id: None,
        }
    }
}

/// Batch import result.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchImportResult {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>, // (path, error_message)
    pub total_processed: usize,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIdentityChangedEvent {
    pub workspace_id: String,
    pub workspace_path: String,
    pub name: String,
    pub identity: Option<WorkspaceIdentity>,
    pub changed_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AssistantWorkspaceDescriptor {
    pub(super) path: PathBuf,
    pub(super) assistant_id: Option<String>,
    pub(super) display_name: String,
}

/// Workspace info updates.
#[derive(Debug, Clone)]
pub struct WorkspaceInfoUpdates {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub related_paths: Option<Vec<RelatedPath>>,
}

/// Batch remove result.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRemoveResult {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub total_processed: usize,
}

/// Workspace health status.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceHealthStatus {
    pub healthy: bool,
    pub total_workspaces: usize,
    pub active_workspaces: usize,
    pub current_workspace_valid: bool,
    pub total_files: usize,
    pub total_size_mb: u64,
    pub warnings: Vec<String>,
    pub issues: Vec<String>,
    pub message: String,
}

/// Workspace export format.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceExport {
    pub workspaces: Vec<WorkspaceInfo>,
    pub current_workspace_id: Option<String>,
    pub recent_workspaces: Vec<String>,
    #[serde(default)]
    pub recent_assistant_workspaces: Vec<String>,
    pub export_timestamp: String,
    pub version: String,
}

/// Workspace import result.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceImportResult {
    pub imported_workspaces: usize,
    pub skipped_workspaces: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Workspace quick summary.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceQuickSummary {
    pub total_workspaces: usize,
    pub active_workspaces: usize,
    pub current_workspace: Option<WorkspaceSummary>,
    pub recent_workspaces: Vec<WorkspaceSummary>,
    #[serde(default)]
    pub recent_assistant_workspaces: Vec<WorkspaceSummary>,
    pub workspace_types: HashMap<WorkspaceType, usize>,
}

/// Workspace persistence data.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkspacePersistenceData {
    pub workspaces: HashMap<String, WorkspaceInfo>,
    #[serde(default)]
    pub opened_workspace_ids: Vec<String>,
    pub current_workspace_id: Option<String>,
    #[serde(default)]
    pub recent_workspaces: Vec<String>,
    #[serde(default)]
    pub recent_assistant_workspaces: Vec<String>,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}
