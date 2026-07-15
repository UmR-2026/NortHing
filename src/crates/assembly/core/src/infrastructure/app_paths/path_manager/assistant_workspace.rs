//! Path accessors for the assistant workspace tree.
//!
//! R73-1 split: extracted from `path_manager.rs` (was lines 117-194).
//! All `~/.northhing/personal_assistant/workspace-*` path resolution
//! lives here. The "legacy" `~/.northhing/workspace-*` paths (used by
//! older clients) are also resolved here for back-compat.

use std::path::{Component, Path, PathBuf};

use super::PathManager;

impl PathManager {
    /// Get assistant home root directory: ~/.northhing/
    pub fn northhing_home_dir(&self) -> PathBuf {
        if let Some(path) = &self.northhing_home_override {
            return path.clone();
        }
        dirs::home_dir()
            .unwrap_or_else(|| self.user_root.clone())
            .join(".northhing")
    }

    /// Get the legacy assistant workspace base directory: ~/.northhing/
    ///
    /// `override_root` is reserved for future user customization.
    pub fn legacy_assistant_workspace_base_dir(&self, override_root: Option<&Path>) -> PathBuf {
        override_root
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.northhing_home_dir())
    }

    /// Get assistant workspace base directory: ~/.northhing/personal_assistant/
    ///
    /// `override_root` is reserved for future user customization.
    pub fn assistant_workspace_base_dir(&self, override_root: Option<&Path>) -> PathBuf {
        self.legacy_assistant_workspace_base_dir(override_root)
            .join("personal_assistant")
    }

    /// Get the legacy default assistant workspace directory: ~/.northhing/workspace
    pub fn legacy_default_assistant_workspace_dir(&self, override_root: Option<&Path>) -> PathBuf {
        self.legacy_assistant_workspace_base_dir(override_root)
            .join("workspace")
    }

    /// Get the default assistant workspace directory: ~/.northhing/personal_assistant/workspace
    pub fn default_assistant_workspace_dir(&self, override_root: Option<&Path>) -> PathBuf {
        self.assistant_workspace_base_dir(override_root).join("workspace")
    }

    /// Get a legacy named assistant workspace directory: ~/.northhing/workspace-<id>
    pub fn legacy_assistant_workspace_dir(&self, assistant_id: &str, override_root: Option<&Path>) -> PathBuf {
        self.legacy_assistant_workspace_base_dir(override_root)
            .join(format!("workspace-{}", assistant_id))
    }

    /// Get a named assistant workspace directory: ~/.northhing/personal_assistant/workspace-<id>
    pub fn assistant_workspace_dir(&self, assistant_id: &str, override_root: Option<&Path>) -> PathBuf {
        self.assistant_workspace_base_dir(override_root)
            .join(format!("workspace-{}", assistant_id))
    }

    /// Resolve assistant workspace directory for default or named assistant.
    pub fn resolve_assistant_workspace_dir(&self, assistant_id: Option<&str>, override_root: Option<&Path>) -> PathBuf {
        match assistant_id {
            Some(id) if !id.trim().is_empty() => self.assistant_workspace_dir(id, override_root),
            _ => self.default_assistant_workspace_dir(override_root),
        }
    }

    /// True if `path` is this machine's northhing **assistant** workspace directory.
    ///
    /// Used so remote-workspace registry (especially roots like `/`) does not
    /// mis-classify client paths such as `/Users/.../.northhing/personal_assistant/workspace-*`
    /// as SSH remote paths.
    pub fn is_local_assistant_workspace_path(&self, path: &str) -> bool {
        let p = Path::new(path);
        if !p.is_absolute() {
            return false;
        }
        if p.starts_with(self.assistant_workspace_base_dir(None)) {
            return true;
        }
        if p.starts_with(self.default_assistant_workspace_dir(None)) {
            return true;
        }
        if p.starts_with(self.legacy_default_assistant_workspace_dir(None)) {
            return true;
        }
        let legacy_base = self.legacy_assistant_workspace_base_dir(None);
        if let Ok(rest) = p.strip_prefix(&legacy_base) {
            if let Some(Component::Normal(first)) = rest.components().next() {
                let name = first.to_string_lossy();
                if name == "workspace" || name.starts_with("workspace-") {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::PathManager;

    #[test]
    fn assistant_workspace_paths_use_personal_assistant_subdir() {
        let path_manager = PathManager::default();
        let base_dir = path_manager.assistant_workspace_base_dir(None);

        assert_eq!(base_dir, path_manager.northhing_home_dir().join("personal_assistant"));
        assert_eq!(
            path_manager.default_assistant_workspace_dir(None),
            base_dir.join("workspace")
        );
        assert_eq!(
            path_manager.assistant_workspace_dir("demo", None),
            base_dir.join("workspace-demo")
        );
        assert_eq!(
            path_manager.resolve_assistant_workspace_dir(None, None),
            base_dir.join("workspace")
        );
        assert_eq!(
            path_manager.resolve_assistant_workspace_dir(Some("demo"), None),
            base_dir.join("workspace-demo")
        );
    }

    #[test]
    fn legacy_assistant_workspace_paths_remain_at_northhing_root() {
        let path_manager = PathManager::default();
        let legacy_base_dir = path_manager.legacy_assistant_workspace_base_dir(None);

        assert_eq!(legacy_base_dir, path_manager.northhing_home_dir());
        assert_eq!(
            path_manager.legacy_default_assistant_workspace_dir(None),
            legacy_base_dir.join("workspace")
        );
        assert_eq!(
            path_manager.legacy_assistant_workspace_dir("demo", None),
            legacy_base_dir.join("workspace-demo")
        );
    }

    #[test]
    fn is_local_assistant_workspace_path_detects_personal_assistant_and_legacy() {
        let pm = PathManager::default();
        let base = pm.assistant_workspace_base_dir(None);
        let named = pm.assistant_workspace_dir("abc", None);
        assert!(pm.is_local_assistant_workspace_path(&named.to_string_lossy()));
        assert!(pm.is_local_assistant_workspace_path(&base.join("workspace").to_string_lossy()));
        let legacy = pm.legacy_assistant_workspace_dir("xyz", None);
        assert!(pm.is_local_assistant_workspace_path(&legacy.to_string_lossy()));
        assert!(!pm.is_local_assistant_workspace_path("/tmp/not-northhing"));
    }
}
