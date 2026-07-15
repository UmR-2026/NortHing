//! R26 sibling 3/4: remote — remote workspace, projection, capability, runtime host traits.
//!
//! Mavis take-over (interface crate, all items `pub`).

use serde;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::port_core::RuntimeServicePort;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteWorkspaceKind {
    Normal,
    Assistant,
    Remote,
}

impl RemoteWorkspaceKind {
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Assistant => "assistant",
            Self::Remote => "remote",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteWorkspaceFacts {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    pub kind: RemoteWorkspaceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRecentWorkspaceFacts {
    pub path: String,
    pub name: String,
    pub last_opened: String,
    pub kind: RemoteWorkspaceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssistantWorkspaceFacts {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteWorkspaceUpdate {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionMetadata {
    pub session_id: String,
    pub name: String,
    pub agent_type: String,
    pub created_at_ms: u64,
    pub last_active_at_ms: u64,
    pub turn_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWorkspaceFileContent {
    pub name: String,
    pub bytes: Vec<u8>,
    pub mime_type: &'static str,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWorkspaceFileChunk {
    pub name: String,
    pub bytes: Vec<u8>,
    pub offset: u64,
    pub chunk_size: u64,
    pub total_size: u64,
    pub mime_type: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWorkspaceFileInfo {
    pub name: String,
    pub size: u64,
    pub mime_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteFileChunkRange {
    pub start: usize,
    pub end: usize,
    pub chunk_size: u64,
}

/// Old remote-connect host compatibility trait for workspace commands.
#[async_trait::async_trait]
pub trait RemoteWorkspaceRuntimeHost: Send + Sync {
    async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts>;
    async fn recent_workspaces(&self) -> Vec<RemoteRecentWorkspaceFacts>;
    async fn open_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String>;
    async fn assistant_workspaces(&self) -> Vec<RemoteAssistantWorkspaceFacts>;
    async fn open_assistant_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String>;
}

/// Typed registration boundary for remote workspace providers.
pub trait RemoteWorkspacePort: RuntimeServicePort + RemoteWorkspaceRuntimeHost {}

impl<T> RemoteWorkspacePort for T where T: RuntimeServicePort + RemoteWorkspaceRuntimeHost + ?Sized {}

/// Old remote-connect host compatibility trait for initial sync.
#[async_trait::async_trait]
pub trait RemoteInitialSyncRuntimeHost: Send + Sync {
    async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts>;
    async fn list_session_metadata(&self, workspace_path: &Path) -> Result<Vec<RemoteSessionMetadata>, String>;
}

/// Old remote-connect host compatibility trait for remote file projection.
#[async_trait::async_trait]
pub trait RemoteWorkspaceFileRuntimeHost: Send + Sync {
    async fn resolve_remote_file_workspace_root(&self, session_id: Option<&str>) -> Option<PathBuf>;
}

/// Typed registration boundary for remote filesystem/terminal/image projection providers.
pub trait RemoteProjectionPort: RuntimeServicePort + RemoteWorkspaceFileRuntimeHost {}

impl<T> RemoteProjectionPort for T where T: RuntimeServicePort + RemoteWorkspaceFileRuntimeHost + ?Sized {}

/// Typed registration boundary for remote host capability facts.
pub trait RemoteCapabilityPort: RuntimeServicePort {}
