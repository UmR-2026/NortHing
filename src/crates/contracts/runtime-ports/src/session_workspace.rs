//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp + remote-connection port traits.
//!
//! Mavis take-over (interface crate, all items `pub`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::port_core::{PortResult, RuntimeServicePort};

pub trait WorkspacePort: RuntimeServicePort {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStoragePathRequest {
    pub workspace_path: PathBuf,
    pub remote_connection_id: Option<String>,
    pub remote_ssh_host: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStorageKind {
    Local,
    Remote,
    UnresolvedRemote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStoragePathResolution {
    Local {
        workspace_path: PathBuf,
    },
    Remote {
        requested_workspace_path: PathBuf,
        effective_storage_path: PathBuf,
        remote_connection_id: Option<String>,
        remote_ssh_host: String,
    },
    UnresolvedRemote {
        requested_workspace_path: PathBuf,
        effective_storage_path: PathBuf,
        remote_connection_id: String,
    },
}

impl Serialize for SessionStoragePathResolution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = match self {
            Self::Local { .. } => serializer.serialize_struct("SessionStoragePathResolution", 3)?,
            Self::Remote { .. } => serializer.serialize_struct("SessionStoragePathResolution", 5)?,
            Self::UnresolvedRemote { .. } => serializer.serialize_struct("SessionStoragePathResolution", 5)?,
        };
        match self {
            Self::Local { workspace_path } => {
                state.serialize_field("requestedWorkspacePath", workspace_path)?;
                state.serialize_field("effectiveStoragePath", workspace_path)?;
                state.serialize_field("storageKind", &SessionStorageKind::Local)?;
            }
            Self::Remote {
                requested_workspace_path,
                effective_storage_path,
                remote_connection_id,
                remote_ssh_host,
            } => {
                state.serialize_field("requestedWorkspacePath", requested_workspace_path)?;
                state.serialize_field("effectiveStoragePath", effective_storage_path)?;
                state.serialize_field("storageKind", &SessionStorageKind::Remote)?;
                state.serialize_field("remoteConnectionId", remote_connection_id)?;
                state.serialize_field("remoteSshHost", remote_ssh_host)?;
            }
            Self::UnresolvedRemote {
                requested_workspace_path,
                effective_storage_path,
                remote_connection_id,
            } => {
                state.serialize_field("requestedWorkspacePath", requested_workspace_path)?;
                state.serialize_field("effectiveStoragePath", effective_storage_path)?;
                state.serialize_field("storageKind", &SessionStorageKind::UnresolvedRemote)?;
                state.serialize_field("remoteConnectionId", &Some(remote_connection_id))?;
                state.serialize_field("remoteSshHost", &None::<String>)?;
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for SessionStoragePathResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ResolutionVisitor;

        #[derive(Deserialize, Default)]
        struct ResolutionFields {
            #[serde(rename = "requestedWorkspacePath")]
            requested_workspace_path: Option<PathBuf>,
            #[serde(rename = "effectiveStoragePath")]
            effective_storage_path: Option<PathBuf>,
            #[serde(rename = "storageKind")]
            storage_kind: Option<SessionStorageKind>,
            #[serde(rename = "remoteConnectionId")]
            remote_connection_id: Option<String>,
            #[serde(rename = "remoteSshHost")]
            remote_ssh_host: Option<String>,
        }

        impl<'de> Visitor<'de> for ResolutionVisitor {
            type Value = SessionStoragePathResolution;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct SessionStoragePathResolution")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut fields = ResolutionFields::default();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "requestedWorkspacePath" => fields.requested_workspace_path = Some(map.next_value()?),
                        "effectiveStoragePath" => fields.effective_storage_path = Some(map.next_value()?),
                        "storageKind" => fields.storage_kind = Some(map.next_value()?),
                        "remoteConnectionId" => fields.remote_connection_id = Some(map.next_value()?),
                        "remoteSshHost" => fields.remote_ssh_host = Some(map.next_value()?),
                        _ => {
                            let _: serde_json::Value = map.next_value()?;
                        }
                    }
                }

                let storage_kind = fields
                    .storage_kind
                    .ok_or_else(|| de::Error::custom("missing field `storageKind`"))?;

                match storage_kind {
                    SessionStorageKind::Local => {
                        let path = fields
                            .effective_storage_path
                            .or(fields.requested_workspace_path)
                            .ok_or_else(|| de::Error::custom("missing workspace path for Local variant"))?;
                        Ok(SessionStoragePathResolution::Local { workspace_path: path })
                    }
                    SessionStorageKind::Remote => {
                        let requested = fields
                            .requested_workspace_path
                            .ok_or_else(|| de::Error::custom("missing field `requestedWorkspacePath`"))?;
                        let effective = fields
                            .effective_storage_path
                            .ok_or_else(|| de::Error::custom("missing field `effectiveStoragePath`"))?;
                        let ssh_host = fields
                            .remote_ssh_host
                            .ok_or_else(|| de::Error::custom("missing field `remoteSshHost` for Remote variant"))?;
                        Ok(SessionStoragePathResolution::Remote {
                            requested_workspace_path: requested,
                            effective_storage_path: effective,
                            remote_connection_id: fields.remote_connection_id,
                            remote_ssh_host: ssh_host,
                        })
                    }
                    SessionStorageKind::UnresolvedRemote => {
                        let requested = fields
                            .requested_workspace_path
                            .ok_or_else(|| de::Error::custom("missing field `requestedWorkspacePath`"))?;
                        let effective = fields
                            .effective_storage_path
                            .ok_or_else(|| de::Error::custom("missing field `effectiveStoragePath`"))?;
                        let conn_id = fields.remote_connection_id.ok_or_else(|| {
                            de::Error::custom("missing field `remoteConnectionId` for UnresolvedRemote variant")
                        })?;
                        Ok(SessionStoragePathResolution::UnresolvedRemote {
                            requested_workspace_path: requested,
                            effective_storage_path: effective,
                            remote_connection_id: conn_id,
                        })
                    }
                }
            }
        }

        deserializer.deserialize_map(ResolutionVisitor)
    }
}

impl SessionStoragePathResolution {
    pub fn local(workspace_path: PathBuf) -> Self {
        Self::Local { workspace_path }
    }

    pub fn remote(
        requested_workspace_path: PathBuf,
        effective_storage_path: PathBuf,
        remote_connection_id: Option<String>,
        remote_ssh_host: String,
    ) -> Self {
        Self::Remote {
            requested_workspace_path,
            effective_storage_path,
            remote_connection_id,
            remote_ssh_host,
        }
    }

    pub fn unresolved_remote(
        requested_workspace_path: PathBuf,
        effective_storage_path: PathBuf,
        remote_connection_id: String,
    ) -> Self {
        Self::UnresolvedRemote {
            requested_workspace_path,
            effective_storage_path,
            remote_connection_id,
        }
    }

    pub fn effective_storage_path(&self) -> &PathBuf {
        match self {
            Self::Local { workspace_path } => workspace_path,
            Self::Remote {
                effective_storage_path, ..
            } => effective_storage_path,
            Self::UnresolvedRemote {
                effective_storage_path, ..
            } => effective_storage_path,
        }
    }

    pub fn requested_workspace_path(&self) -> &PathBuf {
        match self {
            Self::Local { workspace_path } => workspace_path,
            Self::Remote {
                requested_workspace_path,
                ..
            } => requested_workspace_path,
            Self::UnresolvedRemote {
                requested_workspace_path,
                ..
            } => requested_workspace_path,
        }
    }

    pub fn storage_kind(&self) -> SessionStorageKind {
        match self {
            Self::Local { .. } => SessionStorageKind::Local,
            Self::Remote { .. } => SessionStorageKind::Remote,
            Self::UnresolvedRemote { .. } => SessionStorageKind::UnresolvedRemote,
        }
    }

    pub fn remote_connection_id(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::Remote {
                remote_connection_id, ..
            } => remote_connection_id.as_deref(),
            Self::UnresolvedRemote {
                remote_connection_id, ..
            } => Some(remote_connection_id.as_str()),
        }
    }

    pub fn remote_ssh_host(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::Remote { remote_ssh_host, .. } => Some(remote_ssh_host.as_str()),
            Self::UnresolvedRemote { .. } => None,
        }
    }

    pub fn is_remote_storage(&self) -> bool {
        matches!(self, Self::Remote { .. } | Self::UnresolvedRemote { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionViewRestoreRequest {
    pub workspace_path: PathBuf,
    pub session_id: String,
    pub include_internal: bool,
    pub tail_turn_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTurnLoadRequest {
    pub workspace_path: PathBuf,
    pub session_id: String,
    pub tail_turn_count: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTurnLoadTiming {
    pub requested_tail_turn_count: Option<usize>,
    pub loaded_turn_count: usize,
    pub total_turn_count: usize,
    pub turn_file_count: usize,
    pub missing_turn_file_count: usize,
    pub fast_path: bool,
    pub metadata_duration_ms: u64,
    pub state_duration_ms: u64,
    pub scan_duration_ms: u64,
    pub read_duration_ms: u64,
    pub max_turn_read_duration_ms: u64,
    pub build_session_duration_ms: u64,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionViewRestoreTiming {
    pub resolve_storage_path_duration_ms: u64,
    pub visibility_metadata_duration_ms: u64,
    pub load_session_with_turns_duration_ms: u64,
    pub normalize_turn_ids_duration_ms: u64,
    pub total_duration_ms: u64,
    pub turn_load: SessionTurnLoadTiming,
}

#[async_trait::async_trait]
pub trait SessionStorePort: RuntimeServicePort {
    async fn resolve_session_storage_path(
        &self,
        request: SessionStoragePathRequest,
    ) -> PortResult<SessionStoragePathResolution>;
}

/// One row from [`WorkspaceFileSystem::read_dir`] (POSIX paths when the backend is remote SSH).
#[derive(Debug, Clone)]
pub struct WorkspaceDirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// Unified file system operations that work for both local and remote workspaces.
#[async_trait::async_trait]
pub trait WorkspaceFileSystem: Send + Sync {
    async fn read_file(&self, path: &str) -> anyhow::Result<Vec<u8>>;
    async fn read_file_text(&self, path: &str) -> anyhow::Result<String>;
    async fn write_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()>;
    async fn exists(&self, path: &str) -> anyhow::Result<bool>;
    async fn is_file(&self, path: &str) -> anyhow::Result<bool>;
    async fn is_dir(&self, path: &str) -> anyhow::Result<bool>;
    /// List immediate children (non-recursive). Symlinks may be included; callers often skip them.
    async fn read_dir(&self, path: &str) -> anyhow::Result<Vec<WorkspaceDirEntry>>;
}

/// Unified shell execution options for local and remote workspaces.
#[derive(Clone, Default)]
pub struct WorkspaceCommandOptions {
    pub timeout_ms: Option<u64>,
    pub cancellation_token: Option<CancellationToken>,
}

impl std::fmt::Debug for WorkspaceCommandOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceCommandOptions")
            .field("timeout_ms", &self.timeout_ms)
            .field(
                "cancellation_token",
                &self.cancellation_token.as_ref().map(|_| "<CancellationToken>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub interrupted: bool,
    pub timed_out: bool,
}

impl WorkspaceCommandResult {
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Unified shell execution for both local and remote workspaces.
#[async_trait::async_trait]
pub trait WorkspaceShell: Send + Sync {
    /// Execute a command and return a structured result.
    async fn exec_with_options(
        &self,
        command: &str,
        options: WorkspaceCommandOptions,
    ) -> anyhow::Result<WorkspaceCommandResult>;

    /// Execute a command and return (stdout, stderr, exit_code).
    async fn exec(&self, command: &str, timeout_ms: Option<u64>) -> anyhow::Result<(String, String, i32)> {
        let result = self
            .exec_with_options(
                command,
                WorkspaceCommandOptions {
                    timeout_ms,
                    ..Default::default()
                },
            )
            .await?;

        if result.timed_out {
            anyhow::bail!("Command timed out after {}ms", timeout_ms.unwrap_or_default());
        }
        if result.interrupted {
            anyhow::bail!("Command was cancelled");
        }

        Ok((result.stdout, result.stderr, result.exit_code))
    }
}

/// Bundle of workspace I/O services injected into tool runtime context.
pub struct WorkspaceServices {
    pub fs: Arc<dyn WorkspaceFileSystem>,
    pub shell: Arc<dyn WorkspaceShell>,
}

impl Clone for WorkspaceServices {
    fn clone(&self) -> Self {
        Self {
            fs: Arc::clone(&self.fs),
            shell: Arc::clone(&self.shell),
        }
    }
}

impl std::fmt::Debug for WorkspaceServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceServices")
            .field("fs", &"<dyn WorkspaceFileSystem>")
            .field("shell", &"<dyn WorkspaceShell>")
            .finish()
    }
}

/// Runtime handles injected into tool execution contexts.
///
/// This bundle is intentionally handle-only. Concrete local or remote
/// implementations are still assembled by product/runtime owners outside this
/// crate.
#[derive(Clone, Default)]
pub struct ToolRuntimeHandles {
    workspace_services: Option<WorkspaceServices>,
    cancellation_token: Option<CancellationToken>,
}

impl ToolRuntimeHandles {
    pub fn new(workspace_services: Option<WorkspaceServices>, cancellation_token: Option<CancellationToken>) -> Self {
        Self {
            workspace_services,
            cancellation_token,
        }
    }

    pub fn workspace_services(&self) -> Option<&WorkspaceServices> {
        self.workspace_services.as_ref()
    }

    pub fn cancellation_token(&self) -> Option<&CancellationToken> {
        self.cancellation_token.as_ref()
    }
}

impl std::fmt::Debug for ToolRuntimeHandles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRuntimeHandles")
            .field(
                "workspace_services",
                &self.workspace_services.as_ref().map(|_| "<WorkspaceServices>"),
            )
            .field(
                "cancellation_token",
                &self.cancellation_token.as_ref().map(|_| "<CancellationToken>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub scope: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
}

#[async_trait::async_trait]
pub trait PermissionPort: RuntimeServicePort {
    async fn request_permission(&self, request: PermissionRequest) -> PortResult<PermissionDecision>;
}

pub trait ClockPort: RuntimeServicePort {
    fn now_unix_millis(&self) -> i64;
}

pub trait TerminalPort: RuntimeServicePort {}

pub trait NetworkPort: RuntimeServicePort {}

pub trait GitPort: RuntimeServicePort {}

/// Marker: any `McpCatalogPort` (the rich async trait in `mcp.rs`) is
/// also a `RuntimeServicePort` for registration through the
/// `RuntimeServicesBuilder`. Kept as a separate marker so the rich
/// port trait stays narrow (single async method) while the runtime
/// services registry can still use the standard builder pattern.
pub trait McpCatalogPort: RuntimeServicePort {}

/// Typed registration boundary for remote connection providers.
///
/// PR1 intentionally keeps this trait handle-free; PR2 adds owner-specific
/// lifecycle methods once behavior-equivalence tests are in place.
pub trait RemoteConnectionPort: RuntimeServicePort {}
