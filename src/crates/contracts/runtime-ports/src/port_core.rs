//! R26 sibling 1/4: port_core — error/result type + base service port trait.
//!
//! Mavis take-over (interface crate, all items `pub`).

use serde::{Deserialize, Serialize};

pub type PortResult<T> = Result<T, PortError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortErrorKind {
    NotAvailable,
    NotFound,
    InvalidRequest,
    PermissionDenied,
    Cancelled,
    Timeout,
    Backend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortError {
    pub kind: PortErrorKind,
    pub message: String,
}

impl PortError {
    pub fn new(kind: PortErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PortError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceCapability {
    FileSystem,
    Workspace,
    SessionStore,
    Permission,
    Events,
    Clock,
    Terminal,
    Network,
    Git,
    McpCatalog,
    RemoteConnection,
    RemoteWorkspace,
    RemoteProjection,
    RemoteCapabilities,
}

impl RuntimeServiceCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileSystem => "filesystem",
            Self::Workspace => "workspace",
            Self::SessionStore => "session_store",
            Self::Permission => "permission",
            Self::Events => "events",
            Self::Clock => "clock",
            Self::Terminal => "terminal",
            Self::Network => "network",
            Self::Git => "git",
            Self::McpCatalog => "mcp_catalog",
            Self::RemoteConnection => "remote_connection",
            Self::RemoteWorkspace => "remote_workspace",
            Self::RemoteProjection => "remote_projection",
            Self::RemoteCapabilities => "remote_capabilities",
        }
    }
}

impl std::fmt::Display for RuntimeServiceCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub trait RuntimeServicePort: Send + Sync {
    fn capability(&self) -> RuntimeServiceCapability;
}

pub trait FileSystemPort: RuntimeServicePort {}
