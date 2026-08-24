//! Core product-full runtime service adapters.
//!
//! This file registers existing core concrete adapters into typed runtime
//! service builders. It does not create new runtime behavior.

use std::sync::Arc;

use northhing_runtime_ports::{
    GitPort, McpCatalogPort, NetworkPort, RuntimeServiceCapability, RuntimeServicePort, SessionStorePort, TerminalPort,
};
use northhing_runtime_services::{RuntimeServicesBuilder, RuntimeServicesProvider};

use crate::agentic::session::CoreSessionStorePort;

#[derive(Debug, Clone, Copy, Default)]
pub struct CoreRuntimeServicesProvider;

impl CoreRuntimeServicesProvider {
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeServicesProvider for CoreRuntimeServicesProvider {
    fn register(&self, builder: RuntimeServicesBuilder) -> RuntimeServicesBuilder {
        let session_store: Arc<dyn SessionStorePort> = Arc::new(CoreSessionStorePort);
        let terminal: Arc<dyn TerminalPort> =
            Arc::new(CoreRuntimeServiceMarkerPort::new(RuntimeServiceCapability::Terminal));
        let network: Arc<dyn NetworkPort> =
            Arc::new(CoreRuntimeServiceMarkerPort::new(RuntimeServiceCapability::Network));
        let git: Arc<dyn GitPort> = Arc::new(CoreRuntimeServiceMarkerPort::new(RuntimeServiceCapability::Git));
        let mcp_catalog: Arc<dyn McpCatalogPort> =
            Arc::new(CoreRuntimeServiceMarkerPort::new(RuntimeServiceCapability::McpCatalog));
        builder
            .with_session_store(session_store)
            .with_optional_terminal(Some(terminal))
            .with_optional_network(Some(network))
            .with_optional_git(Some(git))
            .with_optional_mcp_catalog(Some(mcp_catalog))
    }
}

#[derive(Debug)]
struct CoreRuntimeServiceMarkerPort {
    capability: RuntimeServiceCapability,
}

impl CoreRuntimeServiceMarkerPort {
    const fn new(capability: RuntimeServiceCapability) -> Self {
        Self { capability }
    }
}

impl RuntimeServicePort for CoreRuntimeServiceMarkerPort {
    fn capability(&self) -> RuntimeServiceCapability {
        self.capability
    }
}

impl TerminalPort for CoreRuntimeServiceMarkerPort {}
impl NetworkPort for CoreRuntimeServiceMarkerPort {}
impl GitPort for CoreRuntimeServiceMarkerPort {}
impl McpCatalogPort for CoreRuntimeServiceMarkerPort {}
