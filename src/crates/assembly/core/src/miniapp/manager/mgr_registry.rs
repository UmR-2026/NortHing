//! Read-side lookups on [`MiniAppManager`]: list metadata, load full app by id,
//! list version numbers, and the small `draft_dir` helpers used by the
//! lifecycle and draft modules.

use super::mgr_types::map_miniapp_port_error;
use super::MiniAppManager;
use crate::miniapp::types::MiniAppMeta;
use crate::util::errors::NortHingResult;
use std::path::PathBuf;

impl MiniAppManager {
    /// List all MiniApp metadata.
    pub async fn list(&self) -> NortHingResult<Vec<MiniAppMeta>> {
        self.runtime_facade()
            .list_metadata()
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Get full MiniApp by id.
    pub async fn get(&self, app_id: &str) -> NortHingResult<crate::miniapp::types::MiniApp> {
        self.runtime_facade()
            .load_app_ensuring_runtime_state(app_id.to_string())
            .await
            .map_err(map_miniapp_port_error)
    }

    /// List version numbers for an app.
    pub async fn list_versions(&self, app_id: &str) -> NortHingResult<Vec<u32>> {
        self.storage.list_versions(app_id).await
    }

    pub fn draft_dir(&self, app_id: &str, draft_id: &str) -> PathBuf {
        self.storage.draft_dir(app_id, draft_id)
    }

    pub(super) fn draft_root_string(&self, app_id: &str, draft_id: &str) -> String {
        self.storage.draft_dir(app_id, draft_id).to_string_lossy().to_string()
    }
}
