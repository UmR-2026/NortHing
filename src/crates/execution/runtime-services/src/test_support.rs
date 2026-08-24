use std::sync::Arc;

use northhing_runtime_ports::{
    ClockPort, FileSystemPort, GitPort, McpCatalogPort, NetworkPort, PermissionDecision, PermissionPort,
    PermissionRequest, PortResult, RuntimeEventEnvelope, RuntimeEventSink, RuntimeServiceCapability,
    RuntimeServicePort, SessionStoragePathRequest, SessionStoragePathResolution, SessionStorePort, TerminalPort,
    WorkspacePort,
};

use crate::{RuntimeServices, RuntimeServicesBuilder, RuntimeServicesError, RuntimeServicesProvider};

#[derive(Debug)]
pub struct FakeRuntimePort {
    capability: RuntimeServiceCapability,
}

impl FakeRuntimePort {
    pub fn new(capability: RuntimeServiceCapability) -> Self {
        Self { capability }
    }
}

impl RuntimeServicePort for FakeRuntimePort {
    fn capability(&self) -> RuntimeServiceCapability {
        self.capability
    }
}

impl FileSystemPort for FakeRuntimePort {}
impl WorkspacePort for FakeRuntimePort {}
#[async_trait::async_trait]
impl SessionStorePort for FakeRuntimePort {
    async fn resolve_session_storage_path(
        &self,
        request: SessionStoragePathRequest,
    ) -> PortResult<SessionStoragePathResolution> {
        Ok(SessionStoragePathResolution::local(request.workspace_path))
    }
}
impl TerminalPort for FakeRuntimePort {}
impl NetworkPort for FakeRuntimePort {}
impl GitPort for FakeRuntimePort {}
impl McpCatalogPort for FakeRuntimePort {}

#[async_trait::async_trait]
impl PermissionPort for FakeRuntimePort {
    async fn request_permission(&self, _request: PermissionRequest) -> PortResult<PermissionDecision> {
        Ok(PermissionDecision::Allow)
    }
}

impl ClockPort for FakeRuntimePort {
    fn now_unix_millis(&self) -> i64 {
        0
    }
}

#[derive(Debug, Default)]
pub struct FakeRuntimeEventSink;

#[async_trait::async_trait]
impl RuntimeEventSink for FakeRuntimeEventSink {
    async fn publish_runtime_event(&self, _event: RuntimeEventEnvelope) -> PortResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FakeRuntimeServicesProvider;

impl FakeRuntimeServicesProvider {
    pub fn with_all_required() -> Self {
        Self
    }

    pub fn build_services(self) -> Result<RuntimeServices, RuntimeServicesError> {
        self.register(RuntimeServicesBuilder::new()).build()
    }
}

impl RuntimeServicesProvider for FakeRuntimeServicesProvider {
    fn register(&self, builder: RuntimeServicesBuilder) -> RuntimeServicesBuilder {
        let filesystem: Arc<dyn FileSystemPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::FileSystem));
        let workspace: Arc<dyn WorkspacePort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Workspace));
        let session_store: Arc<dyn SessionStorePort> =
            Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::SessionStore));
        let permission: Arc<dyn PermissionPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Permission));
        let events: Arc<dyn RuntimeEventSink> = Arc::new(FakeRuntimeEventSink);
        let clock: Arc<dyn ClockPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Clock));

        builder
            .with_filesystem(filesystem)
            .with_workspace(workspace)
            .with_session_store(session_store)
            .with_permission(permission)
            .with_events(events)
            .with_clock(clock)
    }
}
