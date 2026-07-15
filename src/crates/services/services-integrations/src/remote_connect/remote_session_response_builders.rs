//! Remote-connect session response DTO builders + session/poll command
//! handlers (Round 11b split).
//!
//! Owns the `SessionInfo` DTO, the response builders consumed by the session
//! and poll command handlers, plus the three session/poll/initial-sync command
//! dispatchers. The session-state tracker lives in `remote_session_state.rs`
//! and exposes `RemoteSessionStateTracker` plus `ActiveTurnSnapshot` for
//! re-serialization here.
//!
//! Sub-domain split (R11b):
//! - `remote_session_response_builders.rs` (~700): DTOs + response builders +
//!   session/poll/initial_sync handlers + handler traits (this file)
//! - `remote_session_state.rs` (~700): state management
//!
//! Cross-sibling imports:
//! - `RemoteCommand` / `RemoteResponse` come from `remote_session_handlers`
//!   (the wire enum owner).
//! - `RemoteSessionStateTracker` / `ActiveTurnSnapshot` come from
//!   `remote_session_state`.

use std::path::{Path, PathBuf};

use northhing_runtime_ports::{RemoteSessionMetadata, RemoteWorkspaceFacts};

use super::remote_request_builders::{
    build_remote_session_create_request, ChatMessage, RemoteModelCatalog, RemoteModelCatalogPollDelta,
};
use super::remote_session_state::ActiveTurnSnapshot;
use super::remote_session_state::RemoteSessionStateTracker;
use super::remote_workspace_resolver::resolve_remote_agent_type;
use super::RemoteConnectSubmissionSource;
use crate::remote_connect::RemoteResponse;

/// Public session-list DTO used in `RemoteResponse::SessionList` /
/// `RemoteResponse::InitialSync`. Lives with the response builders because
/// every builder that emits a session list produces one of these.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub name: String,
    pub agent_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
}

pub fn remote_session_info(
    metadata: &RemoteSessionMetadata,
    workspace_path: Option<&str>,
    workspace_name: Option<&str>,
) -> SessionInfo {
    SessionInfo {
        session_id: metadata.session_id.clone(),
        name: metadata.name.clone(),
        agent_type: metadata.agent_type.clone(),
        created_at: (metadata.created_at_ms / 1000).to_string(),
        updated_at: (metadata.last_active_at_ms / 1000).to_string(),
        message_count: metadata.turn_count,
        workspace_path: workspace_path.map(ToOwned::to_owned),
        workspace_name: workspace_name.map(ToOwned::to_owned),
    }
}

pub fn remote_session_list_response(
    metadata: Vec<RemoteSessionMetadata>,
    workspace_path: Option<&str>,
    workspace_name: Option<&str>,
    limit: usize,
    offset: usize,
) -> RemoteResponse {
    let page_size = limit.min(100);
    let total = metadata.len();
    let has_more = offset.saturating_add(page_size) < total;
    let sessions = metadata
        .iter()
        .skip(offset)
        .take(page_size)
        .map(|session| remote_session_info(session, workspace_path, workspace_name))
        .collect();

    RemoteResponse::SessionList { sessions, has_more }
}

pub fn remote_initial_sync_response(
    workspace: Option<RemoteWorkspaceFacts>,
    metadata: Vec<RemoteSessionMetadata>,
    session_workspace_name: Option<&str>,
    has_more_sessions: bool,
    authenticated_user_id: Option<String>,
) -> RemoteResponse {
    let (has_workspace, path, project_name, git_branch, workspace_kind, assistant_id) = match workspace {
        Some(workspace) => (
            true,
            Some(workspace.path.clone()),
            Some(workspace.name.clone()),
            workspace.git_branch.clone(),
            Some(workspace.kind.as_wire_str().to_string()),
            workspace.assistant_id.clone(),
        ),
        None => (false, None, None, None, None, None),
    };
    let workspace_path = path.as_deref();
    let sessions = metadata
        .iter()
        .map(|session| remote_session_info(session, workspace_path, session_workspace_name))
        .collect();

    RemoteResponse::InitialSync {
        has_workspace,
        path,
        project_name,
        git_branch,
        workspace_kind,
        assistant_id,
        sessions,
        has_more_sessions,
        authenticated_user_id,
    }
}

pub fn remote_session_created_response(session_id: impl Into<String>) -> RemoteResponse {
    RemoteResponse::SessionCreated {
        session_id: session_id.into(),
    }
}

pub fn remote_session_model_updated_response(
    session_id: impl Into<String>,
    model_id: impl Into<String>,
) -> RemoteResponse {
    RemoteResponse::SessionModelUpdated {
        session_id: session_id.into(),
        model_id: model_id.into(),
    }
}

pub fn remote_messages_response(
    session_id: impl Into<String>,
    messages: Vec<ChatMessage>,
    has_more: bool,
) -> RemoteResponse {
    RemoteResponse::Messages {
        session_id: session_id.into(),
        messages,
        has_more,
    }
}

pub fn remote_session_deleted_response(session_id: impl Into<String>) -> RemoteResponse {
    RemoteResponse::SessionDeleted {
        session_id: session_id.into(),
    }
}

pub fn remote_model_catalog_poll_delta(
    current_model_catalog: Option<RemoteModelCatalog>,
    known_model_catalog_version: Option<u64>,
) -> RemoteModelCatalogPollDelta {
    let changed = should_send_remote_model_catalog(current_model_catalog.as_ref(), known_model_catalog_version);
    let catalog = if changed { current_model_catalog } else { None };

    RemoteModelCatalogPollDelta { changed, catalog }
}

pub fn should_send_remote_model_catalog(
    current_model_catalog: Option<&RemoteModelCatalog>,
    known_model_catalog_version: Option<u64>,
) -> bool {
    let current_version = current_model_catalog.map(|catalog| catalog.version).unwrap_or(0);
    known_model_catalog_version.unwrap_or(0) != current_version
}

pub fn remote_no_change_poll_response(version: u64) -> RemoteResponse {
    RemoteResponse::SessionPoll {
        version,
        changed: false,
        session_state: None,
        title: None,
        new_messages: None,
        total_msg_count: None,
        active_turn: None,
        model_catalog: Box::new(None),
    }
}

pub fn remote_snapshot_poll_response(
    tracker: &RemoteSessionStateTracker,
    version: u64,
    model_catalog: Option<RemoteModelCatalog>,
) -> RemoteResponse {
    let active_turn = tracker.snapshot_active_turn();
    let session_state = tracker.session_state();
    let title = tracker.title();
    RemoteResponse::SessionPoll {
        version,
        changed: true,
        session_state: Some(session_state),
        title: non_empty_title(title),
        new_messages: None,
        total_msg_count: None,
        active_turn,
        model_catalog: Box::new(model_catalog),
    }
}

pub fn remote_persisted_poll_response(
    tracker: &RemoteSessionStateTracker,
    version: u64,
    new_messages: Vec<ChatMessage>,
    total_msg_count: usize,
    model_catalog: Option<RemoteModelCatalog>,
) -> RemoteResponse {
    let turn_finished = tracker.is_turn_finished();
    let has_assistant_msg = new_messages.iter().any(|message| message.role == "assistant");

    let active_turn = if turn_finished && has_assistant_msg {
        tracker.finalize_completed_turn();
        None
    } else if turn_finished {
        let status = tracker.turn_status();
        if status == "completed" {
            tracker.snapshot_active_turn()
        } else {
            tracker.finalize_completed_turn();
            tracker.mark_persistence_clean();
            None
        }
    } else {
        tracker.snapshot_active_turn()
    };

    let (send_messages, send_total) = if turn_finished && !has_assistant_msg {
        (None, None)
    } else {
        if !new_messages.is_empty() {
            tracker.mark_persistence_clean();
        }
        (Some(new_messages), Some(total_msg_count))
    };

    let session_state = tracker.session_state();
    let title = tracker.title();
    RemoteResponse::SessionPoll {
        version,
        changed: true,
        session_state: Some(session_state),
        title: non_empty_title(title),
        new_messages: send_messages,
        total_msg_count: send_total,
        active_turn,
        model_catalog: Box::new(model_catalog),
    }
}

fn non_empty_title(title: String) -> Option<String> {
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

#[async_trait::async_trait]
pub trait RemoteSessionRuntimeHost: Send + Sync {
    async fn list_session_metadata(&self, workspace_path: &Path) -> Result<Vec<RemoteSessionMetadata>, String>;
    async fn resolve_default_assistant_workspace_path(&self) -> Result<String, String>;
    async fn create_session(
        &self,
        request: northhing_runtime_ports::AgentSessionCreateRequest,
    ) -> Result<String, String>;
    async fn load_model_catalog(&self, session_id: Option<&str>) -> Result<RemoteModelCatalog, String>;
    async fn update_session_model(&self, session_id: &str, model_id: &str) -> Result<String, String>;
    async fn ensure_session_loaded(&self, session_id: &str) -> Result<(), String>;
    async fn update_session_title(&self, session_id: &str, title: &str) -> Result<String, String>;
    async fn resolve_session_workspace_path(&self, session_id: &str) -> Option<PathBuf>;
    async fn load_remote_chat_messages(&self, workspace_path: &Path, session_id: &str) -> (Vec<ChatMessage>, bool);
    async fn delete_session(&self, workspace_path: &Path, session_id: &str) -> Result<(), String>;
    fn remove_tracker(&self, session_id: &str);
}

#[async_trait::async_trait]
pub trait RemotePollRuntimeHost: Send + Sync {
    fn ensure_tracker(&self, session_id: &str) -> std::sync::Arc<RemoteSessionStateTracker>;
    async fn load_model_catalog(&self, session_id: &str) -> Option<RemoteModelCatalog>;
    async fn resolve_session_workspace_path(&self, session_id: &str) -> Option<PathBuf>;
    async fn load_remote_chat_messages(&self, workspace_path: &Path, session_id: &str) -> (Vec<ChatMessage>, bool);
}

pub async fn handle_remote_session_command<H>(host: &H, command: &super::RemoteCommand) -> RemoteResponse
where
    H: RemoteSessionRuntimeHost + ?Sized,
{
    use super::RemoteCommand;
    match command {
        RemoteCommand::ListSessions {
            workspace_path,
            limit,
            offset,
            query,
        } => {
            let page_size = limit.unwrap_or(30).min(100);
            let page_offset = offset.unwrap_or(0);

            let Some(workspace_path) = workspace_path
                .as_deref()
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
            else {
                return RemoteResponse::Error {
                    message: "workspace_path is required for ListSessions".to_string(),
                };
            };

            let workspace_path_str = workspace_path.to_string_lossy().to_string();
            let workspace_name = workspace_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string());

            match host.list_session_metadata(&workspace_path).await {
                Ok(metadata) => {
                    let query = query
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_lowercase);
                    let sessions = metadata
                        .into_iter()
                        .filter(|session| {
                            query
                                .as_ref()
                                .is_none_or(|query| session.name.to_lowercase().contains(query))
                        })
                        .collect();
                    remote_session_list_response(
                        sessions,
                        Some(workspace_path_str.as_str()),
                        workspace_name.as_deref(),
                        page_size,
                        page_offset,
                    )
                }
                Err(message) => RemoteResponse::Error { message },
            }
        }
        RemoteCommand::CreateSession {
            agent_type,
            session_name,
            workspace_path,
        } => {
            let agent = resolve_remote_agent_type(agent_type.as_deref());
            let is_claw = agent == "Claw";
            let session_name = session_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or(match agent {
                    "Cowork" => "Remote Cowork Session",
                    "Claw" => "Remote Claw Session",
                    _ => "Remote Code Session",
                });

            let binding_workspace = if is_claw {
                match host.resolve_default_assistant_workspace_path().await {
                    Ok(path) => Some(path),
                    Err(message) => return RemoteResponse::Error { message },
                }
            } else {
                workspace_path
                    .as_deref()
                    .filter(|path| !path.is_empty())
                    .map(ToOwned::to_owned)
            };

            let Some(binding_workspace) = binding_workspace else {
                return RemoteResponse::Error {
                    message: if is_claw {
                        "Failed to get or create assistant workspace".to_string()
                    } else {
                        "workspace_path is required for CreateSession".to_string()
                    },
                };
            };

            let request = build_remote_session_create_request(
                session_name,
                agent,
                Some(binding_workspace),
                RemoteConnectSubmissionSource::Relay,
            );
            match host.create_session(request).await {
                Ok(session_id) => remote_session_created_response(session_id),
                Err(message) => RemoteResponse::Error { message },
            }
        }
        RemoteCommand::GetModelCatalog { session_id } => match host.load_model_catalog(session_id.as_deref()).await {
            Ok(catalog) => RemoteResponse::ModelCatalog { catalog },
            Err(message) => RemoteResponse::Error { message },
        },
        RemoteCommand::SetSessionModel { session_id, model_id } => match host
            .update_session_model(session_id, model_id)
            .await
        {
            Ok(normalized_model_id) => remote_session_model_updated_response(session_id.clone(), normalized_model_id),
            Err(message) => RemoteResponse::Error { message },
        },
        RemoteCommand::UpdateSessionTitle { session_id, title } => {
            if let Err(message) = host.ensure_session_loaded(session_id).await {
                return RemoteResponse::Error { message };
            }

            match host.update_session_title(session_id, title).await {
                Ok(normalized_title) => RemoteResponse::SessionTitleUpdated {
                    session_id: session_id.clone(),
                    title: normalized_title,
                },
                Err(message) => RemoteResponse::Error { message },
            }
        }
        RemoteCommand::GetSessionMessages {
            session_id,
            limit: _,
            before_message_id: _,
        } => {
            let Some(workspace_path) = host.resolve_session_workspace_path(session_id).await else {
                return RemoteResponse::Error {
                    message: format!("Workspace path not available for session: {}", session_id),
                };
            };
            let (chat_messages, has_more) = host.load_remote_chat_messages(&workspace_path, session_id).await;
            remote_messages_response(session_id.clone(), chat_messages, has_more)
        }
        RemoteCommand::DeleteSession { session_id } => {
            let Some(workspace_path) = host.resolve_session_workspace_path(session_id).await else {
                return RemoteResponse::Error {
                    message: format!("Workspace path not available for session: {}", session_id),
                };
            };

            match host.delete_session(&workspace_path, session_id).await {
                Ok(()) => {
                    host.remove_tracker(session_id);
                    remote_session_deleted_response(session_id.clone())
                }
                Err(message) => RemoteResponse::Error { message },
            }
        }
        _ => RemoteResponse::Error {
            message: "Unknown session command".into(),
        },
    }
}

pub async fn handle_remote_poll_command<H>(host: &H, command: &super::RemoteCommand) -> RemoteResponse
where
    H: RemotePollRuntimeHost + ?Sized,
{
    use super::RemoteCommand;
    let RemoteCommand::PollSession {
        session_id,
        since_version,
        known_msg_count,
        known_model_catalog_version,
    } = command
    else {
        return RemoteResponse::Error {
            message: "expected poll_session".into(),
        };
    };

    let tracker = host.ensure_tracker(session_id);
    let current_version = tracker.version();
    let current_model_catalog = host.load_model_catalog(session_id).await;
    let model_catalog_delta = remote_model_catalog_poll_delta(current_model_catalog, *known_model_catalog_version);

    if *since_version == current_version && *since_version > 0 && !model_catalog_delta.changed {
        return remote_no_change_poll_response(current_version);
    }

    let needs_persistence = *since_version == 0 || tracker.is_persistence_dirty();
    if !needs_persistence {
        return remote_snapshot_poll_response(&tracker, current_version, model_catalog_delta.catalog);
    }

    let Some(workspace_path) = host.resolve_session_workspace_path(session_id).await else {
        return RemoteResponse::Error {
            message: format!("Workspace path not available for session: {}", session_id),
        };
    };
    let (all_chat_messages, _) = host.load_remote_chat_messages(&workspace_path, session_id).await;
    let total_msg_count = all_chat_messages.len();
    let new_messages = all_chat_messages.into_iter().skip(*known_msg_count).collect();

    remote_persisted_poll_response(
        &tracker,
        current_version,
        new_messages,
        total_msg_count,
        model_catalog_delta.catalog,
    )
}

pub async fn generate_remote_initial_sync<H>(host: &H, authenticated_user_id: Option<String>) -> RemoteResponse
where
    H: northhing_runtime_ports::RemoteInitialSyncRuntimeHost + ?Sized,
{
    let workspace = host.current_workspace().await;
    let workspace_path = workspace.as_ref().map(|workspace| PathBuf::from(&workspace.path));
    let workspace_name = workspace_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string());

    let (sessions, has_more) = if let Some(path) = workspace_path.as_deref() {
        match host.list_session_metadata(path).await {
            Ok(metadata) => {
                let total = metadata.len();
                let page_size = 100usize;
                (metadata.into_iter().take(page_size).collect(), total > page_size)
            }
            Err(_) => (Vec::new(), false),
        }
    } else {
        (Vec::new(), false)
    };

    remote_initial_sync_response(
        workspace,
        sessions,
        workspace_name.as_deref(),
        has_more,
        authenticated_user_id,
    )
}
