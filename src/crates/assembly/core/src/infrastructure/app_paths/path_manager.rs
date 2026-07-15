//! Unified path management module
//!
//! Provides unified management for all app storage paths, supporting user, project, and temporary levels.
//!
//! R73-1 split: this file is the entry and contains only the core types,
//! env helpers, default/test constructors, and the global singleton.
//! Path accessors live in dedicated submodules under `path_manager/`:
//! - `assistant_workspace` — `~/.northhing/personal_assistant/workspace-*` (incl. legacy)
//! - `user_paths`          — `~/.config/northhing/{config,agents,skills,cache,runtimes,data,cron,rules,miniapps,logs,temp}`
//!                           and `~/.northhing/remote_ssh/`
//! - `project_paths`       — `{workspace}/.northhing/...` and `~/.northhing/projects/<slug>/...`
//!                           + project runtime slug cache
//! - `init`                — `ensure_dir` + `initialize_user_directories`

use crate::util::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::error;

mod assistant_workspace;
mod init;
mod project_paths;
mod user_paths;

/// Storage level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageLevel {
    /// User: global configuration and data
    User,
    /// Project: configuration for a specific project
    Project,
    /// Session: temporary data for the current session
    Session,
    /// Temporary: cache that can be cleaned
    Temporary,
}

/// Path manager
///
/// Manages all app storage paths consistently across platforms
#[derive(Debug, Clone)]
pub struct PathManager {
    /// User config root directory
    pub(super) user_root: PathBuf,
    /// Optional override for the northhing home directory, used by tests to avoid
    /// touching the real user home.
    pub(super) northhing_home_override: Option<PathBuf>,
    /// Cache of runtime slugs keyed by the original and canonical workspace paths.
    ///
    /// `pub(super)` because `project_paths::project_runtime_slug` and friends
    /// (in the same `path_manager` module subtree) need to read/write it
    /// directly to avoid an extra accessor on every lookup.
    pub(super) project_runtime_slug_cache: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl PathManager {
    /// Create a new path manager
    pub fn new() -> NortHingResult<Self> {
        Self::validate_e2e_storage_guard()?;
        let user_root = Self::get_user_config_root()?;
        let northhing_home_override = Self::get_northhing_home_override();

        Ok(Self {
            user_root,
            northhing_home_override,
            project_runtime_slug_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn env_path(name: &str) -> Option<PathBuf> {
        env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
    }

    fn env_flag_enabled(name: &str) -> bool {
        matches!(env::var(name).ok().as_deref(), Some("1") | Some("true") | Some("TRUE"))
    }

    fn validate_e2e_storage_guard() -> NortHingResult<()> {
        if !Self::env_flag_enabled("northhing_E2E_STORAGE_GUARD") {
            return Ok(());
        }

        let has_user_root =
            Self::env_path("northhing_USER_ROOT").is_some() || Self::env_path("northhing_E2E_USER_ROOT").is_some();
        let has_home_root =
            Self::env_path("northhing_HOME").is_some() || Self::env_path("northhing_E2E_HOME").is_some();

        if has_user_root && has_home_root {
            return Ok(());
        }

        Err(NortHingError::config(
            "northhing_E2E_STORAGE_GUARD requires isolated northhing_E2E_USER_ROOT and northhing_E2E_HOME storage roots",
        ))
    }

    /// Get user config root directory
    ///
    /// - Windows: %APPDATA%\northhing\
    /// - macOS: ~/Library/Application Support/northhing/
    /// - Linux: ~/.config/northhing/
    fn get_user_config_root() -> NortHingResult<PathBuf> {
        if let Some(path) = Self::env_path("northhing_USER_ROOT").or_else(|| Self::env_path("northhing_E2E_USER_ROOT"))
        {
            return Ok(path);
        }

        let config_dir =
            dirs::config_dir().ok_or_else(|| NortHingError::config("Failed to get config directory".to_string()))?;

        Ok(config_dir.join("northhing"))
    }

    fn get_northhing_home_override() -> Option<PathBuf> {
        Self::env_path("northhing_HOME").or_else(|| Self::env_path("northhing_E2E_HOME"))
    }
}

impl Default for PathManager {
    fn default() -> Self {
        match Self::new() {
            Ok(manager) => manager,
            Err(e) => {
                error!(
                    "Failed to create PathManager from system config directory, using temp fallback: {}",
                    e
                );
                Self {
                    user_root: std::env::temp_dir().join("northhing"),
                    northhing_home_override: Self::get_northhing_home_override(),
                    project_runtime_slug_cache: Arc::new(Mutex::new(HashMap::new())),
                }
            }
        }
    }
}

#[cfg(test)]
impl PathManager {
    /// Test-only constructor that bypasses the real user home by injecting an isolated
    /// `user_root` directory. This avoids touching the actual user profile during tests.
    /// `northhing_home_override` is set to `user_root.parent().join("home").join(".northhing")`.
    pub(crate) fn with_user_root_for_tests(user_root: PathBuf) -> Self {
        let base = user_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| user_root.clone());
        Self {
            user_root,
            northhing_home_override: Some(base.join("home").join(".northhing")),
            project_runtime_slug_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Global PathManager instance
static GLOBAL_PATH_MANAGER: OnceLock<Arc<PathManager>> = OnceLock::new();

fn init_global_path_manager() -> NortHingResult<Arc<PathManager>> {
    PathManager::new().map(Arc::new)
}

/// Get the global PathManager instance (Arc)
///
/// Return a shared Arc to the global PathManager instance
pub fn path_manager_arc() -> Arc<PathManager> {
    GLOBAL_PATH_MANAGER
        .get_or_init(|| match init_global_path_manager() {
            Ok(manager) => manager,
            Err(e) => {
                error!(
                    "Failed to create global PathManager from config directory, using fallback: {}",
                    e
                );
                Arc::new(PathManager::default())
            }
        })
        .clone()
}

/// Try to get the global PathManager instance (Arc)
pub fn try_get_path_manager_arc() -> NortHingResult<Arc<PathManager>> {
    if let Some(manager) = GLOBAL_PATH_MANAGER.get() {
        return Ok(Arc::clone(manager));
    }

    let manager = init_global_path_manager()?;
    match GLOBAL_PATH_MANAGER.set(Arc::clone(&manager)) {
        Ok(()) => Ok(manager),
        Err(_) => Ok(Arc::clone(
            GLOBAL_PATH_MANAGER
                .get()
                .expect("GLOBAL_PATH_MANAGER should be initialized after set failure"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::PathManager;
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn env_overrides_keep_e2e_storage_out_of_real_user_profile() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let _env_guard = EnvVarGuard::capture([
            "northhing_USER_ROOT",
            "northhing_E2E_USER_ROOT",
            "northhing_HOME",
            "northhing_E2E_HOME",
            "northhing_E2E_STORAGE_GUARD",
        ]);
        let temp_root = std::env::temp_dir().join("northhing-e2e-path-manager-test");
        let user_root = temp_root.join("user-root");
        let home_root = temp_root.join("home");

        std::env::remove_var("northhing_USER_ROOT");
        std::env::set_var("northhing_E2E_USER_ROOT", &user_root);
        std::env::remove_var("northhing_HOME");
        std::env::set_var("northhing_E2E_HOME", &home_root);

        let pm = PathManager::new().expect("path manager should use env overrides");
        assert_eq!(pm.user_config_dir(), user_root.join("config"));
        assert_eq!(pm.user_data_dir(), user_root.join("data"));
        assert_eq!(pm.northhing_home_dir(), home_root);
    }

    #[test]
    fn e2e_storage_guard_rejects_missing_isolated_roots() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let _env_guard = EnvVarGuard::capture([
            "northhing_USER_ROOT",
            "northhing_E2E_USER_ROOT",
            "northhing_HOME",
            "northhing_E2E_HOME",
            "northhing_E2E_STORAGE_GUARD",
        ]);

        std::env::remove_var("northhing_USER_ROOT");
        std::env::remove_var("northhing_E2E_USER_ROOT");
        std::env::remove_var("northhing_HOME");
        std::env::remove_var("northhing_E2E_HOME");
        std::env::set_var("northhing_E2E_STORAGE_GUARD", "1");

        let error = PathManager::new().expect_err("guard should reject real-profile storage");
        let message = error.to_string();
        assert!(message.contains("northhing_E2E_STORAGE_GUARD"));
        assert!(message.contains("northhing_E2E_USER_ROOT"));
    }

    struct EnvVarGuard {
        values: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvVarGuard {
        fn capture(names: impl IntoIterator<Item = &'static str>) -> Self {
            Self {
                values: names.into_iter().map(|name| (name, std::env::var_os(name))).collect(),
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for (name, value) in self.values.drain(..) {
                restore_env(name, value);
            }
        }
    }

    fn restore_env(name: &str, value: Option<OsString>) {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }
}
