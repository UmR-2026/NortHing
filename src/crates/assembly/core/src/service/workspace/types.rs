//! R27 sibling types — workspace struct/enum + impl Default + impl WorkspaceIdentity + free fn.
//!
//! Mavis take-over (impl-block god-impl, horizontal split). impl+struct
//! in same sibling for private field access.

#[cfg(feature = "service-integrations")]
use crate::service::git::GitService;
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, local_workspace_roots_equal, local_workspace_stable_storage_id,
    normalize_local_workspace_root_for_stable_id, normalize_remote_workspace_path, LOCAL_WORKSPACE_SSH_HOST,
};
use crate::util::{errors::*, FrontMatterMarkdown};
use tracing::warn;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub use northhing_runtime_ports::RelatedPath;

use super::*;

/// Workspace type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkspaceType {
    RustProject,
    NodeProject,
    PythonProject,
    JavaProject,
    CppProject,
    WebProject,
    MobileProject,
    Other,
}

/// Workspace status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkspaceStatus {
    Active,
    Inactive,
    Loading,
    Error,
    Archived,
}

/// Workspace lifecycle kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceKind {
    #[default]
    Normal,
    Assistant,
    Remote,
}

pub const IDENTITY_FILE_NAME: &str = "IDENTITY.md";

/// Parsed agent identity fields from `IDENTITY.md` frontmatter.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

/// Git worktree metadata attached to a workspace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceWorktreeInfo {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub main_repo_path: String,
    pub is_main: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct WorkspaceIdentityFrontmatter {
    name: Option<String>,
    creature: Option<String>,
    vibe: Option<String>,
    emoji: Option<String>,
}

impl WorkspaceIdentity {
    pub(crate) async fn load_from_workspace_root(workspace_root: &Path) -> Result<Option<Self>, String> {
        let identity_path = workspace_root.join(IDENTITY_FILE_NAME);
        if !identity_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&identity_path)
            .await
            .map_err(|e| format!("Failed to read identity file '{}': {}", identity_path.display(), e))?;

        let identity = Self::from_markdown(&content)?;
        if identity.is_empty() {
            Ok(None)
        } else {
            Ok(Some(identity))
        }
    }

    pub(super) fn from_markdown(content: &str) -> Result<Self, String> {
        let (metadata, _) = FrontMatterMarkdown::load_str(content)?;
        let frontmatter: WorkspaceIdentityFrontmatter =
            serde_yaml::from_value(metadata).map_err(|e| format!("Failed to parse identity frontmatter: {}", e))?;

        Ok(Self {
            name: normalize_identity_field(frontmatter.name),
            creature: normalize_identity_field(frontmatter.creature),
            vibe: normalize_identity_field(frontmatter.vibe),
            emoji: normalize_identity_field(frontmatter.emoji),
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.name.is_none() && self.creature.is_none() && self.vibe.is_none() && self.emoji.is_none()
    }

    pub(crate) fn collect_changed_fields(
        previous: Option<&WorkspaceIdentity>,
        current: Option<&WorkspaceIdentity>,
    ) -> Vec<String> {
        let previous_name = previous.and_then(|identity| identity.name.as_deref());
        let current_name = current.and_then(|identity| identity.name.as_deref());
        let previous_creature = previous.and_then(|identity| identity.creature.as_deref());
        let current_creature = current.and_then(|identity| identity.creature.as_deref());
        let previous_vibe = previous.and_then(|identity| identity.vibe.as_deref());
        let current_vibe = current.and_then(|identity| identity.vibe.as_deref());
        let previous_emoji = previous.and_then(|identity| identity.emoji.as_deref());
        let current_emoji = current.and_then(|identity| identity.emoji.as_deref());

        let mut changed_fields = Vec::new();
        if previous_name != current_name {
            changed_fields.push("name".to_string());
        }
        if previous_creature != current_creature {
            changed_fields.push("creature".to_string());
        }
        if previous_vibe != current_vibe {
            changed_fields.push("vibe".to_string());
        }
        if previous_emoji != current_emoji {
            changed_fields.push("emoji".to_string());
        }

        changed_fields
    }
}

fn normalize_identity_field(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Workspace metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "rootPath")]
    pub root_path: PathBuf,
    #[serde(rename = "workspaceType")]
    pub workspace_type: WorkspaceType,
    #[serde(rename = "workspaceKind", default)]
    pub workspace_kind: WorkspaceKind,
    #[serde(rename = "assistantId", default, skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
    pub status: WorkspaceStatus,
    pub languages: Vec<String>,
    #[serde(rename = "openedAt")]
    pub opened_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastAccessed")]
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub statistics: Option<WorkspaceStatistics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<WorkspaceIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorkspaceWorktreeInfo>,
    #[serde(rename = "relatedPaths", default)]
    pub related_paths: Vec<RelatedPath>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Workspace statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatistics {
    pub total_files: usize,
    pub total_directories: usize,
    pub total_size_bytes: u64,
    pub file_extensions: HashMap<String, usize>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub git_info: Option<GitInfo>,
}

/// Git information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub is_git_repo: bool,
    pub current_branch: Option<String>,
    pub remote_url: Option<String>,
    pub has_uncommitted_changes: bool,
    pub total_commits: Option<usize>,
}

/// Options for scanning a workspace.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    pub scan_git_info: bool,
    pub calculate_statistics: bool,
    pub ignore_patterns: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include_hidden: false,
            max_depth: Some(10),
            scan_git_info: true,
            calculate_statistics: false,
            ignore_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "__pycache__".to_string(),
                "build".to_string(),
                "dist".to_string(),
            ],
        }
    }
}

/// Options for opening a workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceOpenOptions {
    pub scan_options: ScanOptions,
    pub auto_set_current: bool,
    pub add_to_recent: bool,
    pub workspace_kind: WorkspaceKind,
    pub assistant_id: Option<String>,
    pub display_name: Option<String>,
    /// For [`WorkspaceKind::Remote`], must match persisted `metadata["connectionId"]` so two
    /// servers opened at the same path (e.g. `/`) are separate workspace tabs.
    pub remote_connection_id: Option<String>,
    /// SSH `host` (connection config) for remote mirror paths and metadata.
    pub remote_ssh_host: Option<String>,
    /// Deterministic workspace id for remote workspaces (see `remote_workspace_stable_id`).
    /// Local/assistant workspaces use a stable `local_*` id from `localhost` + canonical root path.
    pub stable_workspace_id: Option<String>,
}

impl Default for WorkspaceOpenOptions {
    fn default() -> Self {
        Self {
            scan_options: ScanOptions::default(),
            auto_set_current: true,
            add_to_recent: true,
            workspace_kind: WorkspaceKind::Normal,
            assistant_id: None,
            display_name: None,
            remote_connection_id: None,
            remote_ssh_host: None,
            stable_workspace_id: None,
        }
    }
}
