use crate::workspace_search::flashgrep::error::AppError;
use crate::workspace_search::flashgrep::{
    ClientCapabilities, ClientInfo, FlashgrepInitializeParams, FlashgrepRepoSession, GlobOutcome, GlobParams,
    GlobRequest, OpenRepoParams, ProtocolClient, QuerySpec, RepoRef, RepoStatus, Request, Response, SearchBackend,
    SearchOutcome, SearchParams, SearchRequest, SearchResults, TaskRef, TaskStatus,
};
use async_trait::async_trait;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub(super) struct RemoteStdioSessionEntry {
    pub(super) session: Arc<RemoteStdioRepoSession>,
    pub(super) activity_epoch: Arc<AtomicU64>,
}

pub(super) struct RemoteStdioRepoSession {
    pub(super) repo_id: String,
    pub(super) client: Arc<RemoteStdioDaemonClient>,
    pub(super) activity_epoch: Arc<AtomicU64>,
    pub(super) active_operations: Arc<AtomicU64>,
}

pub(super) struct RemoteStdioOperationLease {
    activity_epoch: Arc<AtomicU64>,
    active_operations: Arc<AtomicU64>,
}

impl Drop for RemoteStdioOperationLease {
    fn drop(&mut self) {
        self.active_operations.fetch_sub(1, Ordering::Relaxed);
        self.activity_epoch.fetch_add(1, Ordering::Relaxed);
    }
}

pub(super) struct RemoteStdioSessionLease {
    session: Arc<RemoteStdioRepoSession>,
    _operation: RemoteStdioOperationLease,
}

impl RemoteStdioSessionLease {
    pub(super) fn new(session: Arc<RemoteStdioRepoSession>) -> Self {
        let operation = session.acquire_operation();
        Self {
            session,
            _operation: operation,
        }
    }
}

impl Deref for RemoteStdioSessionLease {
    type Target = RemoteStdioRepoSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

pub(super) struct RemoteStdioDaemonClient {
    protocol: ProtocolClient,
}

impl RemoteStdioDaemonClient {
    pub(super) async fn spawn(
        provider: Arc<dyn super::protocol::RemoteWorkspaceSearchProvider>,
        connection_id: String,
        binary_path: String,
    ) -> Result<Arc<Self>, String> {
        let command = format!("{} serve --stdio", super::shell_escape(&binary_path));
        let (protocol, write_rx) = ProtocolClient::channel("remote flashgrep stdio daemon");
        let stdio_protocol = super::protocol::RemoteWorkspaceSearchStdioProtocol::new(protocol.clone());
        provider
            .spawn_stdio_daemon(&connection_id, &command, write_rx, stdio_protocol)
            .await?;

        let client = Arc::new(Self { protocol });
        client.initialize().await?;
        Ok(client)
    }

    pub(super) async fn initialize(&self) -> Result<(), String> {
        match self
            .protocol
            .send_request_with_timeout(
                Request::Initialize {
                    params: FlashgrepInitializeParams {
                        client_info: Some(ClientInfo {
                            name: "northhing-remote-workspace-search".to_string(),
                            version: Some(env!("CARGO_PKG_VERSION").to_string()),
                        }),
                        capabilities: ClientCapabilities::default(),
                    },
                },
                Some(Duration::from_secs(120)),
            )
            .await
            .map_err(|error| error.to_string())?
        {
            Response::InitializeResult { .. } => {
                self.protocol
                    .send_notification(Request::Initialized)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(())
            }
            other => Err(format!("Unexpected remote flashgrep initialize response: {other:?}")),
        }
    }

    pub(super) async fn open_repo(self: &Arc<Self>, params: OpenRepoParams) -> Result<RemoteStdioRepoSession, String> {
        match self.send_request(Request::OpenRepo { params }).await? {
            Response::RepoOpened { repo_id, .. } => Ok(RemoteStdioRepoSession {
                repo_id,
                client: self.clone(),
                activity_epoch: Arc::new(AtomicU64::new(1)),
                active_operations: Arc::new(AtomicU64::new(0)),
            }),
            other => Err(format!("Unexpected remote flashgrep open_repo response: {other:?}")),
        }
    }

    pub(super) async fn send_request(&self, request: Request) -> Result<Response, String> {
        self.protocol
            .send_request_with_timeout(request, Some(Duration::from_secs(120)))
            .await
            .map_err(|error| error.to_string())
    }

    pub(super) async fn shutdown(&self) {
        let _ = timeout(Duration::from_secs(2), self.send_request(Request::Shutdown)).await;
        self.protocol
            .close_with_message("remote flashgrep stdio daemon is shutting down")
            .await;
    }

    pub(super) fn is_closed(&self) -> bool {
        self.protocol.is_closed()
    }
}

impl RemoteStdioRepoSession {
    pub(super) fn acquire_operation(&self) -> RemoteStdioOperationLease {
        self.active_operations.fetch_add(1, Ordering::Relaxed);
        self.activity_epoch.fetch_add(1, Ordering::Relaxed);
        RemoteStdioOperationLease {
            activity_epoch: self.activity_epoch.clone(),
            active_operations: self.active_operations.clone(),
        }
    }

    pub(super) async fn status(&self) -> Result<RepoStatus, String> {
        let _lease = self.acquire_operation();
        self.status_without_activity_lease().await
    }

    pub(super) async fn status_without_activity_lease(&self) -> Result<RepoStatus, String> {
        match self
            .client
            .send_request(Request::GetRepoStatus {
                params: self.repo_ref(),
            })
            .await?
        {
            Response::RepoStatus { status } => Ok(status),
            other => Err(format!(
                "Unexpected remote flashgrep get_repo_status response: {other:?}"
            )),
        }
    }

    pub(super) async fn task_status(&self, task_id: impl Into<String>) -> Result<TaskStatus, String> {
        let _lease = self.acquire_operation();
        match self
            .client
            .send_request(Request::TaskStatus {
                params: TaskRef {
                    task_id: task_id.into(),
                },
            })
            .await?
        {
            Response::TaskStatus { task } => Ok(task),
            other => Err(format!("Unexpected remote flashgrep task/status response: {other:?}")),
        }
    }

    pub(super) async fn build_index(&self) -> Result<TaskStatus, String> {
        let _lease = self.acquire_operation();
        match self
            .client
            .send_request(Request::BaseSnapshotBuild {
                params: self.repo_ref(),
            })
            .await?
        {
            Response::TaskStarted { task } => Ok(task),
            other => Err(format!("Unexpected remote flashgrep build response: {other:?}")),
        }
    }

    pub(super) async fn rebuild_index(&self) -> Result<TaskStatus, String> {
        let _lease = self.acquire_operation();
        match self
            .client
            .send_request(Request::BaseSnapshotRebuild {
                params: self.repo_ref(),
            })
            .await?
        {
            Response::TaskStarted { task } => Ok(task),
            other => Err(format!("Unexpected remote flashgrep rebuild response: {other:?}")),
        }
    }

    pub(super) async fn search(
        &self,
        query: QuerySpec,
        scope: crate::workspace_search::flashgrep::PathScope,
    ) -> Result<(SearchBackend, RepoStatus, SearchResults), String> {
        let _lease = self.acquire_operation();
        match self
            .client
            .send_request(Request::Search {
                params: SearchParams {
                    repo_id: self.repo_id.clone(),
                    query,
                    scope,
                    consistency: crate::workspace_search::flashgrep::ConsistencyMode::WorkspaceEventual,
                    allow_scan_fallback: true,
                },
            })
            .await?
        {
            Response::SearchCompleted {
                backend,
                status,
                results,
                ..
            } => Ok((backend, status, results)),
            other => Err(format!("Unexpected remote flashgrep search response: {other:?}")),
        }
    }

    pub(super) async fn glob(
        &self,
        scope: crate::workspace_search::flashgrep::PathScope,
    ) -> Result<(RepoStatus, Vec<String>), String> {
        let _lease = self.acquire_operation();
        match self
            .client
            .send_request(Request::Glob {
                params: GlobParams {
                    repo_id: self.repo_id.clone(),
                    scope,
                },
            })
            .await?
        {
            Response::GlobCompleted { status, paths, .. } => Ok((status, paths)),
            other => Err(format!("Unexpected remote flashgrep glob response: {other:?}")),
        }
    }

    pub(super) async fn close(&self) {
        let _ = self
            .client
            .send_request(Request::CloseRepo {
                params: self.repo_ref(),
            })
            .await;
    }

    pub(super) fn repo_ref(&self) -> RepoRef {
        RepoRef {
            repo_id: self.repo_id.clone(),
        }
    }
}

#[async_trait]
impl FlashgrepRepoSession for RemoteStdioRepoSession {
    async fn status(&self) -> crate::workspace_search::flashgrep::error::Result<RepoStatus> {
        RemoteStdioRepoSession::status(self).await.map_err(AppError::Protocol)
    }

    async fn task_status(&self, task_id: String) -> crate::workspace_search::flashgrep::error::Result<TaskStatus> {
        RemoteStdioRepoSession::task_status(self, task_id)
            .await
            .map_err(AppError::Protocol)
    }

    async fn build_index(&self) -> crate::workspace_search::flashgrep::error::Result<TaskStatus> {
        RemoteStdioRepoSession::build_index(self)
            .await
            .map_err(AppError::Protocol)
    }

    async fn rebuild_index(&self) -> crate::workspace_search::flashgrep::error::Result<TaskStatus> {
        RemoteStdioRepoSession::rebuild_index(self)
            .await
            .map_err(AppError::Protocol)
    }

    async fn search(&self, request: SearchRequest) -> crate::workspace_search::flashgrep::error::Result<SearchOutcome> {
        let (backend, status, results) = RemoteStdioRepoSession::search(self, request.query, request.scope)
            .await
            .map_err(AppError::Protocol)?;
        Ok(SearchOutcome {
            backend,
            status,
            results,
        })
    }

    async fn glob(&self, request: GlobRequest) -> crate::workspace_search::flashgrep::error::Result<GlobOutcome> {
        let (status, paths) = RemoteStdioRepoSession::glob(self, request.scope)
            .await
            .map_err(AppError::Protocol)?;
        Ok(GlobOutcome { status, paths })
    }

    async fn close(&self) -> crate::workspace_search::flashgrep::error::Result<()> {
        RemoteStdioRepoSession::close(self).await;
        Ok(())
    }
}
