//! Remote-connect integration contracts (Round 11b split).
//!
//! This module owns remote-connect wire assembly, runtime-port request
//! construction, compatibility re-exports, and remote session tracker state.
//!
//! Round 11 split: 5 sibling files own domain-specific fns by prefix cluster.
//!
//! Round 11b split (QClaw R11 REQUIRED): the 2 over-cap files
//! `remote_command_handlers.rs` (1301) and `remote_session_tracker.rs` (1272)
//! are split into 5 new siblings, each ≤ 800 lines:
//! - `remote_session_state` (~700): `RemoteSessionStateTracker` + Registry +
//!   TrackerState + TrackerEvent + ActiveTurnSnapshot
//! - `remote_session_response_builders` (~600): `SessionInfo` + response
//!   builders + session/poll/initial-sync handlers + handler traits
//! - `remote_dialog_handlers` (~200): dialog submission types and fn
//! - `remote_cancel_handlers` (~110): cancel-task types and fn
//! - `remote_session_handlers` (~700): wire `RemoteCommand`/`RemoteResponse`
//!   re-exports + workspace/file/interaction sub-handlers + integration tests
//!
//! The wire enums + `RemoteCommandRuntimeHost` + top `handle_remote_command`
//! dispatcher live here in `mod.rs` because they are the cross-sibling
//! command routing hub — moving them out to any single sibling would create
//! an asymmetric import dependency that all five new siblings need.
//!
//! Pre-existing partial split (Round 5/6 era): device/encryption/pairing/
//! qr_generator/relay_client — unchanged.

pub mod device;
pub mod encryption;
pub mod pairing;
pub mod qr_generator;
pub mod relay_client;

pub mod remote_cancel_handlers;
pub mod remote_dialog_handlers;
pub mod remote_file_io;
pub mod remote_request_builders;
pub mod remote_session_handlers;
pub mod remote_session_response_builders;
pub mod remote_session_state;
pub mod remote_workspace_resolver;

// Re-export existing public types (preserves external API)
pub use device::DeviceIdentity;
pub use encryption::{decrypt_from_base64, encrypt_to_base64, KeyPair};
pub use pairing::{PairingChallenge, PairingProtocol, PairingResponse, PairingState, QrPayload};
pub use qr_generator::QrGenerator;
pub use relay_client::{ensure_rustls_crypto_provider, ConnectionState, RelayClient, RelayEvent, RelayMessage};

// Re-export runtime-port DTOs that the original `remote_connect.rs` re-exported
// as part of the cross-crate public API. Callers in `northhing-core` import
// these via `northhing_services_integrations::remote_connect::*`.
pub use northhing_runtime_ports::{
    RemoteAssistantWorkspaceFacts, RemoteControlStateSnapshot, RemoteFileChunkRange, RemoteInitialSyncRuntimeHost,
    RemoteProjectionPort, RemoteRecentWorkspaceFacts, RemoteSessionMetadata, RemoteWorkspaceFacts,
    RemoteWorkspaceFileChunk, RemoteWorkspaceFileContent, RemoteWorkspaceFileInfo, RemoteWorkspaceFileRuntimeHost,
    RemoteWorkspaceKind, RemoteWorkspacePort, RemoteWorkspaceRuntimeHost, RemoteWorkspaceUpdate,
};

// Re-export new sibling public types (preserve external API)
pub use remote_cancel_handlers::*;
pub use remote_dialog_handlers::*;
pub use remote_file_io::*;
pub use remote_request_builders::*;
pub use remote_session_handlers::*;
pub use remote_session_response_builders::*;
pub use remote_session_state::*;
pub use remote_workspace_resolver::*;

// Shared cross-sibling enum kept at mod.rs (used in many fn signatures).
use northhing_runtime_ports::AgentSubmissionSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteConnectSubmissionSource {
    Relay,
    Bot,
}

impl RemoteConnectSubmissionSource {
    pub const fn agent_submission_source(self) -> AgentSubmissionSource {
        match self {
            RemoteConnectSubmissionSource::Relay => AgentSubmissionSource::RemoteRelay,
            RemoteConnectSubmissionSource::Bot => AgentSubmissionSource::Bot,
        }
    }

    pub const fn metadata_source(self) -> &'static str {
        match self {
            RemoteConnectSubmissionSource::Relay => "remote_relay",
            RemoteConnectSubmissionSource::Bot => "bot",
        }
    }
}

// ----------------------------------------------------------------------------
// Wire enums + top-level command dispatcher (R11b: moved from
// `remote_command_handlers.rs` to keep `remote_session_handlers.rs` under the
// 800-line cap while preserving the public API path).
// ----------------------------------------------------------------------------

use tracing::info;

// Items brought into scope via the `pub use` re-exports above; the explicit
// `use` lines are only needed for sibling-local helpers that are NOT in the
// glob re-export (e.g. fns whose only consumer is this file).

/// Commands that remote clients can send to the desktop runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum RemoteCommand {
    GetWorkspaceInfo,
    ListRecentWorkspaces,
    SetWorkspace {
        path: String,
    },
    ListAssistants,
    SetAssistant {
        path: String,
    },
    ListSessions {
        workspace_path: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
        query: Option<String>,
    },
    CreateSession {
        agent_type: Option<String>,
        session_name: Option<String>,
        workspace_path: Option<String>,
    },
    GetModelCatalog {
        session_id: Option<String>,
    },
    SetSessionModel {
        session_id: String,
        model_id: String,
    },
    UpdateSessionTitle {
        session_id: String,
        title: String,
    },
    GetSessionMessages {
        session_id: String,
        limit: Option<usize>,
        before_message_id: Option<String>,
    },
    SendMessage {
        session_id: String,
        content: String,
        agent_type: Option<String>,
        images: Option<Vec<ImageAttachment>>,
        image_contexts: Option<Vec<RemoteImageContext>>,
    },
    CancelTask {
        session_id: String,
        turn_id: Option<String>,
    },
    DeleteSession {
        session_id: String,
    },
    ConfirmTool {
        tool_id: String,
        updated_input: Option<serde_json::Value>,
    },
    RejectTool {
        tool_id: String,
        reason: Option<String>,
    },
    CancelTool {
        tool_id: String,
        reason: Option<String>,
    },
    AnswerQuestion {
        tool_id: String,
        answers: serde_json::Value,
    },
    PollSession {
        session_id: String,
        since_version: u64,
        known_msg_count: usize,
        known_model_catalog_version: Option<u64>,
    },
    ReadFile {
        path: String,
        session_id: Option<String>,
    },
    ReadFileChunk {
        path: String,
        session_id: Option<String>,
        offset: u64,
        limit: u64,
    },
    GetFileInfo {
        path: String,
        session_id: Option<String>,
    },
    Ping,
}

/// Responses sent from desktop back to remote clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "resp", rename_all = "snake_case")]
pub enum RemoteResponse {
    WorkspaceInfo {
        has_workspace: bool,
        path: Option<String>,
        project_name: Option<String>,
        git_branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        assistant_id: Option<String>,
    },
    RecentWorkspaces {
        workspaces: Vec<RecentWorkspaceEntry>,
    },
    WorkspaceUpdated {
        success: bool,
        path: Option<String>,
        project_name: Option<String>,
        error: Option<String>,
    },
    AssistantList {
        assistants: Vec<AssistantEntry>,
    },
    AssistantUpdated {
        success: bool,
        path: Option<String>,
        name: Option<String>,
        error: Option<String>,
    },
    SessionList {
        sessions: Vec<self::remote_session_response_builders::SessionInfo>,
        has_more: bool,
    },
    SessionCreated {
        session_id: String,
    },
    ModelCatalog {
        catalog: RemoteModelCatalog,
    },
    SessionModelUpdated {
        session_id: String,
        model_id: String,
    },
    SessionTitleUpdated {
        session_id: String,
        title: String,
    },
    Messages {
        session_id: String,
        messages: Vec<self::remote_request_builders::ChatMessage>,
        has_more: bool,
    },
    MessageSent {
        session_id: String,
        turn_id: String,
    },
    TaskCancelled {
        session_id: String,
    },
    SessionDeleted {
        session_id: String,
    },
    InitialSync {
        has_workspace: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        project_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        assistant_id: Option<String>,
        sessions: Vec<self::remote_session_response_builders::SessionInfo>,
        has_more_sessions: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        authenticated_user_id: Option<String>,
    },
    SessionPoll {
        version: u64,
        changed: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_state: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        new_messages: Option<Vec<self::remote_request_builders::ChatMessage>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_msg_count: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        active_turn: Option<self::remote_session_state::ActiveTurnSnapshot>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_catalog: Box<Option<RemoteModelCatalog>>,
    },
    AnswerAccepted,
    InteractionAccepted {
        action: String,
        target_id: String,
    },
    FileContent {
        name: String,
        content_base64: String,
        mime_type: String,
        size: u64,
    },
    FileChunk {
        name: String,
        chunk_base64: String,
        offset: u64,
        chunk_size: u64,
        total_size: u64,
        mime_type: String,
    },
    FileInfo {
        name: String,
        size: u64,
        mime_type: String,
    },
    Pong,
    Error {
        message: String,
    },
}

/// Host callbacks required by full remote-connect command routing.
#[async_trait::async_trait]
pub trait RemoteCommandRuntimeHost: Send + Sync {
    type ImageContext: Send + Sync + 'static;

    async fn handle_workspace_command(&self, command: &RemoteCommand) -> RemoteResponse;
    async fn handle_session_command(&self, command: &RemoteCommand) -> RemoteResponse;
    async fn handle_poll_command(&self, command: &RemoteCommand) -> RemoteResponse;
    async fn handle_workspace_file_command(&self, command: &RemoteCommand) -> RemoteResponse;
    async fn handle_interaction_command(&self, command: &RemoteCommand) -> RemoteResponse;

    async fn submit_dialog(
        &self,
        request: RemoteDialogSubmissionRequest<Self::ImageContext>,
    ) -> Result<self::remote_dialog_handlers::RemoteDialogSubmitOutcome, String>;

    async fn cancel_task(&self, request: self::remote_cancel_handlers::RemoteCancelTaskRequest) -> Result<(), String>;

    fn legacy_image_contexts(&self, images: Option<&[ImageAttachment]>) -> Vec<Self::ImageContext>;

    fn explicit_image_contexts(&self, contexts: Vec<RemoteImageContext>) -> Vec<Self::ImageContext>;
}

/// Top-level remote-connect command dispatcher. Delegates workspace / session /
/// poll / file / interaction commands to host methods, and handles dialog +
/// cancel commands inline via the dialog + cancel sibling fns.
pub async fn handle_remote_command<H>(
    host: &H,
    command: &RemoteCommand,
    source: RemoteConnectSubmissionSource,
) -> RemoteResponse
where
    H: RemoteCommandRuntimeHost + ?Sized,
{
    use self::remote_cancel_handlers::remote_task_cancel_response;
    use self::remote_cancel_handlers::RemoteCancelTaskRequest;
    use self::remote_dialog_handlers::remote_dialog_submit_response;

    match command {
        RemoteCommand::Ping => RemoteResponse::Pong,

        RemoteCommand::GetWorkspaceInfo
        | RemoteCommand::ListRecentWorkspaces
        | RemoteCommand::SetWorkspace { .. }
        | RemoteCommand::ListAssistants
        | RemoteCommand::SetAssistant { .. } => host.handle_workspace_command(command).await,

        RemoteCommand::ListSessions { .. }
        | RemoteCommand::CreateSession { .. }
        | RemoteCommand::GetModelCatalog { .. }
        | RemoteCommand::SetSessionModel { .. }
        | RemoteCommand::UpdateSessionTitle { .. }
        | RemoteCommand::GetSessionMessages { .. }
        | RemoteCommand::DeleteSession { .. } => host.handle_session_command(command).await,

        RemoteCommand::PollSession { .. } => host.handle_poll_command(command).await,

        RemoteCommand::ReadFile { .. } | RemoteCommand::ReadFileChunk { .. } | RemoteCommand::GetFileInfo { .. } => {
            host.handle_workspace_file_command(command).await
        }

        RemoteCommand::ConfirmTool { .. }
        | RemoteCommand::RejectTool { .. }
        | RemoteCommand::CancelTool { .. }
        | RemoteCommand::AnswerQuestion { .. } => host.handle_interaction_command(command).await,

        RemoteCommand::SendMessage {
            session_id,
            content,
            agent_type,
            images,
            image_contexts,
        } => {
            let resolved_contexts = resolve_remote_execution_image_contexts(
                images.as_ref().map(Vec::as_slice),
                image_contexts
                    .clone()
                    .map(|contexts| host.explicit_image_contexts(contexts)),
                |images| host.legacy_image_contexts(images),
            );
            info!(
                "Remote send_message: session={session_id}, agent_type={}, image_contexts={}",
                agent_type.as_deref().unwrap_or("agentic"),
                resolved_contexts.len()
            );
            remote_dialog_submit_response(
                host.submit_dialog(RemoteDialogSubmissionRequest {
                    session_id: session_id.clone(),
                    content: content.clone(),
                    agent_type: agent_type.clone(),
                    image_contexts: resolved_contexts,
                    policy: RemoteDialogSubmissionPolicy::for_source(source),
                    turn_id: None,
                })
                .await,
            )
        }

        RemoteCommand::CancelTask { session_id, turn_id } => remote_task_cancel_response(
            session_id.clone(),
            host.cancel_task(RemoteCancelTaskRequest {
                session_id: session_id.clone(),
                requested_turn_id: turn_id.clone(),
            })
            .await,
        ),
    }
}
