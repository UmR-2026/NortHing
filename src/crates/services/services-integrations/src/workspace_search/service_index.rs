//! Workspace search index lifecycle methods.
//!
//! Implements [`WorkspaceSearchService`] index/status methods by delegating
//! to the shared session opener in [`super::service_session`] and the
//! flashgrep protocol trait.

use super::flashgrep::{FlashgrepRepoSession, FLASHGREP_LOG_TARGET};
use super::service::WorkspaceSearchService;
use super::types::{IndexTaskHandle, WorkspaceIndexStatus};

impl WorkspaceSearchService {
    pub async fn open_repo(
        &self,
        repo_root: impl AsRef<std::path::Path>,
    ) -> super::service::WorkspaceSearchResult<WorkspaceIndexStatus> {
        let session = self.get_or_open_session(repo_root.as_ref()).await?;
        self.index_status_for_session(session).await
    }

    pub async fn get_index_status(
        &self,
        repo_root: impl AsRef<std::path::Path>,
    ) -> super::service::WorkspaceSearchResult<WorkspaceIndexStatus> {
        let session = self.get_or_open_session(repo_root.as_ref()).await?;
        self.index_status_for_session(session).await
    }

    pub async fn build_index(
        &self,
        repo_root: impl AsRef<std::path::Path>,
    ) -> super::service::WorkspaceSearchResult<IndexTaskHandle> {
        let session = self.get_or_open_session(repo_root.as_ref()).await?;
        let task = FlashgrepRepoSession::build_index(session.as_ref()).await.map_err(
            super::service_search::map_flashgrep_error("Failed to start index build"),
        )?;
        let repo_status = session
            .status()
            .await
            .map_err(super::service_search::map_flashgrep_error(
                "Failed to fetch repository status",
            ))?;
        tracing::info!(
            target: FLASHGREP_LOG_TARGET,
            "Workspace search build index requested: repo_root={}, task_id={}, phase={:?}",
            repo_root.as_ref().display(),
            task.task_id,
            repo_status.phase
        );
        Ok(IndexTaskHandle {
            task: task.into(),
            repo_status: repo_status.into(),
        })
    }

    pub async fn rebuild_index(
        &self,
        repo_root: impl AsRef<std::path::Path>,
    ) -> super::service::WorkspaceSearchResult<IndexTaskHandle> {
        let session = self.get_or_open_session(repo_root.as_ref()).await?;
        let task = FlashgrepRepoSession::rebuild_index(session.as_ref()).await.map_err(
            super::service_search::map_flashgrep_error("Failed to start index rebuild"),
        )?;
        let repo_status = session
            .status()
            .await
            .map_err(super::service_search::map_flashgrep_error(
                "Failed to fetch repository status",
            ))?;
        tracing::info!(
            target: FLASHGREP_LOG_TARGET,
            "Workspace search rebuild index requested: repo_root={}, task_id={}, phase={:?}",
            repo_root.as_ref().display(),
            task.task_id,
            repo_status.phase
        );
        Ok(IndexTaskHandle {
            task: task.into(),
            repo_status: repo_status.into(),
        })
    }

    pub(super) async fn index_status_for_session<S>(
        &self,
        session: std::sync::Arc<S>,
    ) -> super::service::WorkspaceSearchResult<WorkspaceIndexStatus>
    where
        S: FlashgrepRepoSession + ?Sized,
    {
        let repo_status = session
            .status()
            .await
            .map_err(super::service_search::map_flashgrep_error(
                "Failed to fetch repository status",
            ))?;
        let active_task = match repo_status.active_task_id.clone() {
            Some(task_id) => match session.task_status(task_id).await {
                Ok(task) => Some(task),
                Err(error) => {
                    tracing::warn!(
                        target: FLASHGREP_LOG_TARGET,
                        "Failed to fetch active flashgrep task status: {}",
                        error
                    );
                    None
                }
            },
            None => None,
        };

        Ok(WorkspaceIndexStatus {
            repo_status: repo_status.into(),
            active_task: active_task.map(Into::into),
        })
    }
}
