use super::types::AnnouncementState;
use crate::infrastructure::app_paths::PathManager;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;

pub struct AnnouncementStateStore {
    inner: northhing_services_integrations::announcement::AnnouncementStateStore,
}

impl AnnouncementStateStore {
    pub fn new(path_manager: &Arc<PathManager>) -> Self {
        Self {
            inner: northhing_services_integrations::announcement::AnnouncementStateStore::new(
                path_manager.user_config_dir(),
            ),
        }
    }

    /// Load state from disk.  Returns a default state if the file does not exist.
    pub async fn load(&self) -> NortHingResult<AnnouncementState> {
        self.inner.load().await.map_err(map_state_store_error)
    }

    /// Persist state to disk.
    pub async fn save(&self, state: &AnnouncementState) -> NortHingResult<()> {
        self.inner.save(state).await.map_err(map_state_store_error)
    }
}

fn map_state_store_error(
    err: northhing_services_integrations::announcement::AnnouncementStateStoreError,
) -> NortHingError {
    match err {
        northhing_services_integrations::announcement::AnnouncementStateStoreError::Io(err) => NortHingError::Io(err),
        northhing_services_integrations::announcement::AnnouncementStateStoreError::Serialization(err) => {
            NortHingError::Serialization(err)
        }
    }
}
