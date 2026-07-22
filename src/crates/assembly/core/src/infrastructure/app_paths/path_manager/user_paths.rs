//! Path accessors for user-level (cross-project) storage.
//!
//! R73-1 split: extracted from `path_manager.rs` (was lines 196-306).
//! All `~/.config/northhing/...` plus `~/.northhing/remote_ssh/...` path
//! resolution lives here. Project-scoped paths are in `project_paths.rs`.

use std::path::PathBuf;

use super::PathManager;

impl PathManager {
    /// Get user config directory: ~/.config/northhing/config/
    pub fn user_config_dir(&self) -> PathBuf {
        self.user_root.join("config")
    }

    /// Get app config file path: ~/.config/northhing/config/app.json
    pub fn app_config_file(&self) -> PathBuf {
        self.user_config_dir().join("app.json")
    }

    /// Get user agent directory: ~/.config/northhing/agents/
    pub fn user_agents_dir(&self) -> PathBuf {
        self.user_root.join("agents")
    }

    /// Get user skills directory:
    /// - Windows: C:\Users\xxx\AppData\Roaming\northhing\skills\
    /// - macOS: ~/Library/Application Support/northhing/skills/
    /// - Linux: ~/.local/share/northhing/skills/
    pub fn user_skills_dir(&self) -> PathBuf {
        self.user_root.join("skills")
    }

    /// Get northhing-managed built-in skills directory under the user skills root.
    pub fn builtin_skills_dir(&self) -> PathBuf {
        self.user_skills_dir().join(".system")
    }

    /// Get cache root directory: ~/.config/northhing/cache/
    pub fn cache_root(&self) -> PathBuf {
        self.user_root.join("cache")
    }

    /// Get managed runtimes root directory: ~/.config/northhing/runtimes/
    ///
    /// northhing-managed runtime components (e.g. node/python/office) are stored here.
    pub fn managed_runtimes_dir(&self) -> PathBuf {
        self.user_root.join("runtimes")
    }

    /// Get user data directory: ~/.config/northhing/data/
    pub fn user_data_dir(&self) -> PathBuf {
        self.user_root.join("data")
    }

    /// Root for per-host, per-remote-path workspace mirrors: `~/.northhing/remote_ssh/`.
    ///
    /// Session/chat persistence for SSH workspaces lives under
    /// `{this}/{sanitized_host}/{remote_path_segments}/sessions/`.
    pub fn remote_ssh_mirror_root() -> PathBuf {
        Self::new()
            .map(|pm| pm.northhing_home_dir().join("remote_ssh"))
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".northhing")
                    .join("remote_ssh")
            })
    }

    /// Get scheduled jobs directory: ~/.config/northhing/data/cron/
    pub fn user_cron_dir(&self) -> PathBuf {
        self.user_data_dir().join("cron")
    }

    /// Get scheduled jobs persistence file: ~/.config/northhing/data/cron/jobs.json
    pub fn cron_jobs_file(&self) -> PathBuf {
        self.user_cron_dir().join("jobs.json")
    }

    /// Get miniapps root directory: ~/.config/northhing/data/miniapps/
    pub fn miniapps_dir(&self) -> PathBuf {
        self.user_data_dir().join("miniapps")
    }

    /// Get directory for a specific miniapp: ~/.config/northhing/data/miniapps/{app_id}/
    pub fn miniapp_dir(&self, app_id: &str) -> PathBuf {
        self.miniapps_dir().join(app_id)
    }

    /// Get user-level rules directory: ~/.config/northhing/data/rules/
    pub fn user_rules_dir(&self) -> PathBuf {
        self.user_data_dir().join("rules")
    }

    /// Get logs directory: ~/.config/northhing/logs/
    pub fn logs_dir(&self) -> PathBuf {
        self.user_root.join("logs")
    }

    /// Get temp directory: ~/.config/northhing/temp/
    pub fn temp_dir(&self) -> PathBuf {
        self.user_root.join("temp")
    }
}
