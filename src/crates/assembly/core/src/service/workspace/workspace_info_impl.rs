//! R27b sibling workspace_info_impl — `WorkspaceInfo` impl + `WorkspaceSummary` struct + `WorkspaceManager` struct + `WorkspaceManagerConfig` + impl Default.
//!
//! Mavis take-over (impl-block god-impl sub-domain split R27b). impl+struct
//! kept in same sibling for private field access.

#[cfg(feature = "service-integrations")]
use crate::service::git::GitService;
use crate::service::remote_ssh::workspace_state::{
    canonicalize_local_workspace_root, local_workspace_roots_equal, local_workspace_stable_storage_id,
    normalize_local_workspace_root_for_stable_id, normalize_remote_workspace_path, LOCAL_WORKSPACE_SSH_HOST,
};
use crate::util::{errors::*, FrontMatterMarkdown};
use tracing::{info, warn};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub use northhing_runtime_ports::RelatedPath;

use super::types::*;
use super::*;

impl WorkspaceInfo {
    /// SSH connection id persisted in [`WorkspaceInfo::metadata`] for remote workspaces.
    pub fn remote_ssh_connection_id(&self) -> Option<&str> {
        self.metadata
            .get("connectionId")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    }

    /// Creates a new workspace record.
    pub async fn new(root_path: PathBuf, options: WorkspaceOpenOptions) -> NortHingResult<Self> {
        let default_name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let workspace_kind = options.workspace_kind.clone();
        let assistant_id = if workspace_kind == WorkspaceKind::Assistant {
            options.assistant_id.clone()
        } else {
            None
        };

        let now = chrono::Utc::now();
        let is_remote = workspace_kind == WorkspaceKind::Remote;
        let (id, resolved_root_path) = if is_remote {
            let id = options
                .stable_workspace_id
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            (id, root_path.clone())
        } else {
            let (canonical_pb, norm_str) =
                canonicalize_local_workspace_root(&root_path).map_err(NortHingError::service)?;
            let id = local_workspace_stable_storage_id(&norm_str);
            (id, canonical_pb)
        };

        let mut workspace = Self {
            id,
            name: options.display_name.clone().unwrap_or(default_name),
            root_path: resolved_root_path,
            workspace_type: WorkspaceType::Other,
            workspace_kind,
            assistant_id,
            status: WorkspaceStatus::Loading,
            languages: Vec::new(),
            opened_at: now,
            last_accessed: now,
            description: None,
            tags: Vec::new(),
            statistics: None,
            identity: None,
            worktree: None,
            related_paths: Vec::new(),
            metadata: HashMap::new(),
        };

        if is_remote {
            if let Some(ssh_host) = options.remote_ssh_host.as_ref().filter(|s| !s.trim().is_empty()) {
                workspace.metadata.insert(
                    "sshHost".to_string(),
                    serde_json::Value::String(ssh_host.trim().to_string()),
                );
            }
            if let Some(conn_id) = options.remote_connection_id.as_ref().filter(|s| !s.trim().is_empty()) {
                workspace.metadata.insert(
                    "connectionId".to_string(),
                    serde_json::Value::String(conn_id.trim().to_string()),
                );
            }
        } else {
            workspace.metadata.insert(
                "sshHost".to_string(),
                serde_json::Value::String(LOCAL_WORKSPACE_SSH_HOST.to_string()),
            );
            workspace.detect_workspace_type().await;
            workspace.load_identity().await;
            workspace.load_worktree().await;

            if options.scan_options.calculate_statistics {
                workspace.scan_workspace(options.scan_options).await?;
            }
        }

        workspace.status = if options.auto_set_current {
            WorkspaceStatus::Active
        } else {
            WorkspaceStatus::Inactive
        };
        Ok(workspace)
    }

    pub(super) async fn load_identity(&mut self) {
        let identity = match WorkspaceIdentity::load_from_workspace_root(&self.root_path).await {
            Ok(identity) => identity,
            Err(error) => {
                warn!(
                    "Failed to load workspace identity: path={} error={}",
                    self.root_path.join(IDENTITY_FILE_NAME).display(),
                    error
                );
                self.identity = None;
                return;
            }
        };

        if self.workspace_kind == WorkspaceKind::Assistant {
            if let Some(name) = identity.as_ref().and_then(|identity| identity.name.as_ref()) {
                self.name = name.clone();
            }
        }

        self.identity = identity;
    }

    pub(super) async fn load_worktree(&mut self) {
        self.worktree = Self::resolve_worktree_info(&self.root_path).await;
    }

    pub(super) async fn resolve_worktree_info(workspace_root: &Path) -> Option<WorkspaceWorktreeInfo> {
        #[cfg(not(feature = "service-integrations"))]
        {
            let _ = workspace_root;
            return None;
        }

        #[cfg(feature = "service-integrations")]
        {
            let normalized_workspace_path = workspace_root.to_string_lossy().replace('\\', "/");
            let worktrees = match GitService::list_worktrees(workspace_root).await {
                Ok(worktrees) => worktrees,
                Err(_) => return None,
            };

            let main_repo_path = worktrees
                .iter()
                .find(|worktree| worktree.is_main)
                .map(|worktree| worktree.path.clone())?;

            worktrees
                .into_iter()
                .find(|worktree| worktree.path == normalized_workspace_path)
                .map(|worktree| WorkspaceWorktreeInfo {
                    path: worktree.path,
                    branch: worktree.branch,
                    main_repo_path: main_repo_path.clone(),
                    is_main: worktree.is_main,
                })
        }
    }

    /// Detects the workspace type.
    pub(super) async fn detect_workspace_type(&mut self) {
        let root = &self.root_path;

        if root.join("Cargo.toml").exists() {
            self.workspace_type = WorkspaceType::RustProject;
            self.languages.push("Rust".to_string());
        } else if root.join("package.json").exists() {
            self.workspace_type = WorkspaceType::NodeProject;
            self.languages.push("JavaScript".to_string());
            self.languages.push("TypeScript".to_string());
        } else if root.join("requirements.txt").exists()
            || root.join("pyproject.toml").exists()
            || root.join("setup.py").exists()
        {
            self.workspace_type = WorkspaceType::PythonProject;
            self.languages.push("Python".to_string());
        } else if root.join("pom.xml").exists() || root.join("build.gradle").exists() {
            self.workspace_type = WorkspaceType::JavaProject;
            self.languages.push("Java".to_string());
        } else if root.join("CMakeLists.txt").exists() || root.join("Makefile").exists() {
            self.workspace_type = WorkspaceType::CppProject;
            self.languages.push("C++".to_string());
        } else if root.join("index.html").exists() || root.join("webpack.config.js").exists() {
            self.workspace_type = WorkspaceType::WebProject;
            self.languages.push("HTML".to_string());
            self.languages.push("CSS".to_string());
            self.languages.push("JavaScript".to_string());
        }

        self.detect_languages_from_files().await;
    }

    /// Detects languages from file extensions.
    pub(super) async fn detect_languages_from_files(&mut self) {
        const LANGUAGE_SCAN_LIMIT: usize = 50;

        let mut language_map = HashMap::new();
        language_map.insert("rs", "Rust");
        language_map.insert("js", "JavaScript");
        language_map.insert("ts", "TypeScript");
        language_map.insert("py", "Python");
        language_map.insert("java", "Java");
        language_map.insert("cpp", "C++");
        language_map.insert("c", "C");
        language_map.insert("h", "C/C++");
        language_map.insert("html", "HTML");
        language_map.insert("css", "CSS");
        language_map.insert("go", "Go");
        language_map.insert("php", "PHP");
        language_map.insert("rb", "Ruby");
        language_map.insert("swift", "Swift");
        language_map.insert("kt", "Kotlin");

        if let Ok(mut read_dir) = fs::read_dir(&self.root_path).await {
            let mut found_languages = std::collections::HashSet::new();
            let mut count = 0;

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                if count > LANGUAGE_SCAN_LIMIT {
                    break;
                }
                count += 1;

                if let Some(extension) = entry.path().extension().and_then(|s| s.to_str()) {
                    if let Some(language) = language_map.get(extension) {
                        found_languages.insert(language.to_string());
                    }
                }
            }

            for lang in found_languages {
                if !self.languages.contains(&lang) {
                    self.languages.push(lang);
                }
            }
        }
    }

    /// Scans the workspace.
    pub(super) async fn scan_workspace(&mut self, options: ScanOptions) -> NortHingResult<()> {
        let mut stats = WorkspaceStatistics {
            total_files: 0,
            total_directories: 0,
            total_size_bytes: 0,
            file_extensions: HashMap::new(),
            last_modified: None,
            git_info: None,
        };

        self.scan_directory(&self.root_path.clone(), &mut stats, &options, 0)
            .await?;

        if options.scan_git_info {
            stats.git_info = self.scan_git_info().await;
        }

        self.statistics = Some(stats);
        Ok(())
    }

    /// Recursively scans a directory.
    fn scan_directory<'a>(
        &'a self,
        dir: &'a Path,
        stats: &'a mut WorkspaceStatistics,
        options: &'a ScanOptions,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NortHingResult<()>> + 'a + Send>> {
        Box::pin(async move {
            if let Some(max_depth) = options.max_depth {
                if depth > max_depth {
                    return Ok(());
                }
            }

            let mut read_dir = fs::read_dir(dir)
                .await
                .map_err(|e| NortHingError::service(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| NortHingError::service(format!("Failed to read directory entry: {}", e)))?
            {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if !options.include_hidden && file_name.starts_with('.') {
                    continue;
                }

                if options
                    .ignore_patterns
                    .iter()
                    .any(|pattern| file_name.contains(pattern))
                {
                    continue;
                }

                let metadata = entry
                    .metadata()
                    .await
                    .map_err(|e| NortHingError::service(format!("Failed to read metadata: {}", e)))?;

                if metadata.is_file() {
                    stats.total_files += 1;
                    stats.total_size_bytes += metadata.len();

                    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                        *stats.file_extensions.entry(extension.to_string()).or_insert(0) += 1;
                    }

                    if let Ok(modified) = metadata.modified() {
                        let modified_dt = chrono::DateTime::<chrono::Utc>::from(modified);
                        if stats
                            .last_modified
                            .as_ref()
                            .is_none_or(|last_modified| last_modified < &modified_dt)
                        {
                            stats.last_modified = Some(modified_dt);
                        }
                    }
                } else if metadata.is_dir() {
                    stats.total_directories += 1;

                    if let Err(e) = self.scan_directory(&path, stats, options, depth + 1).await {
                        warn!("Failed to scan subdirectory {:?}: {}", path, e);
                    }
                }
            }

            Ok(())
        })
    }

    /// Scans Git information.
    pub(super) async fn scan_git_info(&self) -> Option<GitInfo> {
        let git_dir = self.root_path.join(".git");
        if !git_dir.exists() {
            return Some(GitInfo {
                is_git_repo: false,
                current_branch: None,
                remote_url: None,
                has_uncommitted_changes: false,
                total_commits: None,
            });
        }

        let mut git_info = GitInfo {
            is_git_repo: true,
            current_branch: None,
            remote_url: None,
            has_uncommitted_changes: false,
            total_commits: None,
        };

        if let Ok(head_content) = fs::read_to_string(git_dir.join("HEAD")).await {
            if let Some(branch) = head_content.strip_prefix("ref: refs/heads/") {
                git_info.current_branch = Some(branch.trim().to_string());
            }
        }

        if let Ok(status_output) = crate::util::process_manager::create_tokio_command("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.root_path)
            .output()
            .await
        {
            git_info.has_uncommitted_changes = !status_output.stdout.is_empty();
        }

        Some(git_info)
    }

    /// Updates the last-accessed timestamp.
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now();
    }

    /// Checks whether the workspace is still valid.
    pub async fn is_valid(&self) -> bool {
        if self.workspace_kind == WorkspaceKind::Remote {
            return true;
        }
        self.root_path.exists() && self.root_path.is_dir()
    }

    /// Returns a workspace summary.
    pub fn summary(&self) -> WorkspaceSummary {
        WorkspaceSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            root_path: self.root_path.clone(),
            workspace_type: self.workspace_type.clone(),
            workspace_kind: self.workspace_kind.clone(),
            assistant_id: self.assistant_id.clone(),
            status: self.status.clone(),
            languages: self.languages.clone(),
            last_accessed: self.last_accessed,
            file_count: self.statistics.as_ref().map(|s| s.total_files).unwrap_or(0),
            tags: self.tags.clone(),
        }
    }
}

/// Workspace summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "rootPath")]
    pub root_path: PathBuf,
    #[serde(rename = "workspaceType")]
    pub workspace_type: WorkspaceType,
    #[serde(rename = "workspaceKind")]
    pub workspace_kind: WorkspaceKind,
    #[serde(rename = "assistantId", skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
    pub status: WorkspaceStatus,
    pub languages: Vec<String>,
    #[serde(rename = "lastAccessed")]
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "fileCount")]
    pub file_count: usize,
    pub tags: Vec<String>,
}

/// Workspace manager.
pub struct WorkspaceManager {
    pub(super) workspaces: HashMap<String, WorkspaceInfo>,
    pub(super) opened_workspace_ids: Vec<String>,
    pub(super) current_workspace_id: Option<String>,
    pub(super) recent_workspaces: Vec<String>,
    pub(super) recent_assistant_workspaces: Vec<String>,
    pub(super) max_recent_workspaces: usize,
}

/// Workspace manager configuration.
#[derive(Debug, Clone)]
pub struct WorkspaceManagerConfig {
    pub max_recent_workspaces: usize,
    pub auto_cleanup_invalid: bool,
    pub default_scan_options: ScanOptions,
}

impl Default for WorkspaceManagerConfig {
    fn default() -> Self {
        Self {
            max_recent_workspaces: 20,
            auto_cleanup_invalid: true,
            default_scan_options: ScanOptions::default(),
        }
    }
}
