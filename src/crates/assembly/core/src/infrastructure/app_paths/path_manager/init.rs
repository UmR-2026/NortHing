//! Bulk directory initialization for `PathManager`.
//!
//! R73-1 split: extracted from `path_manager.rs` (was lines 442-475).
//! `ensure_dir` is a single-directory helper; `initialize_user_directories`
//! creates the full user-level layout on first launch.

use std::path::Path;

use super::PathManager;
use crate::util::errors::{NortHingError, NortHingResult};

impl PathManager {
    /// Ensure directory exists
    pub async fn ensure_dir(&self, path: &Path) -> NortHingResult<()> {
        if !path.exists() {
            tokio::fs::create_dir_all(path)
                .await
                .map_err(|e| NortHingError::service(format!("Failed to create directory {:?}: {}", path, e)))?;
        }
        Ok(())
    }

    /// Initialize user-level directory structure
    pub async fn initialize_user_directories(&self) -> NortHingResult<()> {
        let dirs = vec![
            self.northhing_home_dir(),
            self.projects_root(),
            self.assistant_workspace_base_dir(None),
            self.user_config_dir(),
            self.user_agents_dir(),
            self.cache_root(),
            self.user_data_dir(),
            self.user_cron_dir(),
            self.user_rules_dir(),
            self.miniapps_dir(),
            self.logs_dir(),
            self.temp_dir(),
        ];

        for dir in dirs {
            self.ensure_dir(&dir).await?;
        }

        tracing::debug!("User-level directories initialized");
        Ok(())
    }
}
