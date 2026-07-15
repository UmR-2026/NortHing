//! Path accessors for project-scoped storage and the project runtime slug cache.
//!
//! R73-1 split: extracted from `path_manager.rs` (was lines 308-440).
//! All `{workspace}/.northhing/...` and `~/.northhing/projects/<slug>/...`
//! path resolution lives here, including the slug cache that maps a
//! workspace's canonical path to a short, filesystem-safe slug.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use super::PathManager;

const MAX_PROJECT_SLUG_LEN: usize = 120;

impl PathManager {
    /// Get project config root directory: {project}/.northhing/
    pub fn project_root(&self, workspace_path: &Path) -> PathBuf {
        workspace_path.join(".northhing")
    }

    /// Get the shared runtime projects root directory: ~/.northhing/projects/
    pub fn projects_root(&self) -> PathBuf {
        self.northhing_home_dir().join("projects")
    }

    /// Get the runtime root for a workspace: ~/.northhing/projects/<workspace-slug>/
    pub fn project_runtime_root(&self, workspace_path: &Path) -> PathBuf {
        self.projects_root().join(self.project_runtime_slug(workspace_path))
    }

    /// Get project internal config directory: {project}/.northhing/config/
    pub fn project_internal_config_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_root(workspace_path).join("config")
    }

    /// Get project agent profiles file: {project}/.northhing/config/agent_profiles.json
    pub fn project_agent_profiles_file(&self, workspace_path: &Path) -> PathBuf {
        self.project_internal_config_dir(workspace_path)
            .join("agent_profiles.json")
    }

    /// Get project mode skills file: {project}/.northhing/config/mode_skills.json
    pub fn project_mode_skills_file(&self, workspace_path: &Path) -> PathBuf {
        self.project_internal_config_dir(workspace_path)
            .join("mode_skills.json")
    }

    /// Get project subagent overrides file: {project}/.northhing/config/agent_subagents.json
    pub fn project_agent_subagents_file(&self, workspace_path: &Path) -> PathBuf {
        self.project_internal_config_dir(workspace_path)
            .join("agent_subagents.json")
    }

    /// Get project agent directory: {project}/.northhing/agents/
    pub fn project_agents_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_root(workspace_path).join("agents")
    }

    /// Get project-level rules directory: {project}/.northhing/rules/
    pub fn project_rules_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_root(workspace_path).join("rules")
    }

    /// Get project snapshots directory: ~/.northhing/projects/<workspace-slug>/snapshots/
    pub fn project_snapshots_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_runtime_root(workspace_path).join("snapshots")
    }

    /// Get project sessions directory: ~/.northhing/projects/<workspace-slug>/sessions/
    pub fn project_sessions_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_runtime_root(workspace_path).join("sessions")
    }

    /// Get project plans directory: ~/.northhing/projects/<workspace-slug>/plans/
    pub fn project_plans_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_runtime_root(workspace_path).join("plans")
    }

    /// Get project memory directory: ~/.northhing/projects/<workspace-slug>/memory/
    pub fn project_memory_dir(&self, workspace_path: &Path) -> PathBuf {
        self.project_runtime_root(workspace_path).join("memory")
    }

    /// Derive the runtime slug for a workspace path. Returns a
    /// filesystem-safe, human-readable slug (lowercased alphanumerics +
    /// `-`; truncated with a 12-char SHA-256 suffix when long).
    fn project_runtime_slug(&self, workspace_path: &Path) -> String {
        let requested_path = workspace_path.to_path_buf();
        if let Some(slug) = self.cached_project_runtime_slug(&requested_path) {
            return slug;
        }

        let canonical_path = dunce::canonicalize(workspace_path).unwrap_or_else(|_| requested_path.clone());
        if canonical_path != requested_path {
            if let Some(slug) = self.cached_project_runtime_slug(&canonical_path) {
                self.store_project_runtime_slug(&requested_path, &slug);
                return slug;
            }
        }

        let canonical = canonical_path.to_string_lossy().to_string();
        let slug = Self::build_project_runtime_slug(&canonical);

        self.store_project_runtime_slug(&canonical_path, &slug);
        if canonical_path != requested_path {
            self.store_project_runtime_slug(&requested_path, &slug);
        }

        slug
    }

    fn cached_project_runtime_slug(&self, workspace_path: &Path) -> Option<String> {
        self.project_runtime_slug_cache
            .lock()
            .expect("project runtime slug cache poisoned")
            .get(workspace_path)
            .cloned()
    }

    fn store_project_runtime_slug(&self, workspace_path: &Path, slug: &str) {
        self.project_runtime_slug_cache
            .lock()
            .expect("project runtime slug cache poisoned")
            .insert(workspace_path.to_path_buf(), slug.to_string());
    }

    fn build_project_runtime_slug(canonical: &str) -> String {
        let slug: String = canonical
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();

        let slug = slug.trim_matches('-');
        let slug = if slug.is_empty() { "workspace" } else { slug };

        if slug.len() <= MAX_PROJECT_SLUG_LEN {
            return slug.to_string();
        }

        let hash = hex::encode(Sha256::digest(canonical.as_bytes()));
        let suffix = &hash[..12];
        let max_prefix_len = MAX_PROJECT_SLUG_LEN.saturating_sub(suffix.len() + 1);
        let prefix = slug[..max_prefix_len].trim_end_matches('-');
        format!("{}-{}", prefix, suffix)
    }
}

// `Arc` and `Mutex` are used by PathManager's runtime-slug cache via `pub(super)`
// visibility on the struct fields; this submodule is the only place that touches
// the cache directly, so we silence the unused-import lint with the explicit
// reference below.
#[allow(unused_imports)]
use {Arc as _, Mutex as _};

#[cfg(test)]
mod tests {
    use super::PathManager;
    use std::path::Path;

    #[test]
    fn project_runtime_root_uses_human_readable_workspace_slug() {
        let pm = PathManager::default();
        let runtime_root = pm.project_runtime_root(Path::new(r"E:\Projects\Opennorthhing\northhing"));
        let slug = runtime_root
            .file_name()
            .and_then(|value| value.to_str())
            .expect("runtime root should have terminal component");

        assert!(slug.starts_with("e--projects-opennorthhing-northhing"));
        assert_eq!(runtime_root.parent(), Some(pm.projects_root().as_path()));
    }
}
