//! Remote-connect workspace/file/interaction sub-handlers (Round 11b split).
//!
//! Owns the workspace / file / interaction sub-handlers, the workspace response
//! builders, and the integration tests. The wire `RemoteCommand` /
//! `RemoteResponse` enums + `RemoteCommandRuntimeHost` trait + top-level
//! `handle_remote_command` dispatcher live in `mod.rs` (they are the
//! cross-sibling command routing hub).
//!
//! Sub-domain split (R11b):
//! - `remote_session_handlers.rs` (~700, this file): workspace/file/interaction
//!   sub-handlers + workspace response builders + integration tests
//! - `remote_session_response_builders.rs` (~600): DTOs + response builders +
//!   session/poll/initial-sync handlers
//! - `remote_dialog_handlers.rs` (~200): dialog submission types and fn
//! - `remote_cancel_handlers.rs` (~110): cancel-task types and fn
//! - `remote_session_state.rs` (~720): state management

use northhing_runtime_ports::{
    RemoteAssistantWorkspaceFacts, RemoteRecentWorkspaceFacts, RemoteWorkspaceFacts, RemoteWorkspaceFileRuntimeHost,
    RemoteWorkspaceRuntimeHost, RemoteWorkspaceUpdate,
};

use super::remote_file_io::{
    read_remote_workspace_file, read_remote_workspace_file_chunk, read_remote_workspace_file_info,
    remote_file_chunk_response, remote_file_content_response, remote_file_info_response,
};
use super::remote_workspace_resolver::REMOTE_FILE_MAX_READ_BYTES;

use crate::remote_connect::RemoteResponse;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecentWorkspaceEntry {
    pub path: String,
    pub name: String,
    pub last_opened: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssistantEntry {
    pub path: String,
    pub name: String,
    pub assistant_id: Option<String>,
}

pub fn remote_workspace_info_response(workspace: Option<RemoteWorkspaceFacts>) -> RemoteResponse {
    match workspace {
        Some(workspace) => RemoteResponse::WorkspaceInfo {
            has_workspace: true,
            path: Some(workspace.path),
            project_name: Some(workspace.name),
            git_branch: workspace.git_branch,
            workspace_kind: Some(workspace.kind.as_wire_str().to_string()),
            assistant_id: workspace.assistant_id,
        },
        None => RemoteResponse::WorkspaceInfo {
            has_workspace: false,
            path: None,
            project_name: None,
            git_branch: None,
            workspace_kind: None,
            assistant_id: None,
        },
    }
}

pub fn remote_recent_workspaces_response(workspaces: Vec<RemoteRecentWorkspaceFacts>) -> RemoteResponse {
    RemoteResponse::RecentWorkspaces {
        workspaces: workspaces
            .into_iter()
            .map(|workspace| RecentWorkspaceEntry {
                path: workspace.path,
                name: workspace.name,
                last_opened: workspace.last_opened,
                workspace_kind: Some(workspace.kind.as_wire_str().to_string()),
            })
            .collect(),
    }
}

pub fn remote_assistant_list_response(assistants: Vec<RemoteAssistantWorkspaceFacts>) -> RemoteResponse {
    RemoteResponse::AssistantList {
        assistants: assistants
            .into_iter()
            .map(|assistant| AssistantEntry {
                path: assistant.path,
                name: assistant.name,
                assistant_id: assistant.assistant_id,
            })
            .collect(),
    }
}

pub fn remote_workspace_updated_response(result: Result<RemoteWorkspaceUpdate, String>) -> RemoteResponse {
    match result {
        Ok(update) => RemoteResponse::WorkspaceUpdated {
            success: true,
            path: Some(update.path),
            project_name: Some(update.name),
            error: None,
        },
        Err(message) => RemoteResponse::WorkspaceUpdated {
            success: false,
            path: None,
            project_name: None,
            error: Some(message),
        },
    }
}

pub fn remote_assistant_updated_response(result: Result<RemoteWorkspaceUpdate, String>) -> RemoteResponse {
    match result {
        Ok(update) => RemoteResponse::AssistantUpdated {
            success: true,
            path: Some(update.path),
            name: Some(update.name),
            error: None,
        },
        Err(message) => RemoteResponse::AssistantUpdated {
            success: false,
            path: None,
            name: None,
            error: Some(message),
        },
    }
}

#[async_trait::async_trait]
pub trait RemoteInteractionRuntimeHost: Send + Sync {
    async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> Result<(), String>;
    async fn reject_tool(&self, tool_id: &str, reason: String) -> Result<(), String>;
    async fn cancel_tool(&self, tool_id: &str, reason: String) -> Result<(), String>;
    fn answer_question(&self, tool_id: &str, answers: serde_json::Value) -> Result<(), String>;
}

pub fn remote_interaction_accepted_response(
    action: impl Into<String>,
    target_id: impl Into<String>,
    result: Result<(), String>,
) -> RemoteResponse {
    match result {
        Ok(()) => RemoteResponse::InteractionAccepted {
            action: action.into(),
            target_id: target_id.into(),
        },
        Err(message) => RemoteResponse::Error { message },
    }
}

pub fn remote_answer_question_response(result: Result<(), String>) -> RemoteResponse {
    match result {
        Ok(()) => RemoteResponse::AnswerAccepted,
        Err(message) => RemoteResponse::Error { message },
    }
}

pub async fn handle_remote_interaction_command<H>(
    host: &H,
    command: &crate::remote_connect::RemoteCommand,
) -> RemoteResponse
where
    H: RemoteInteractionRuntimeHost + ?Sized,
{
    use super::RemoteCommand;
    match command {
        RemoteCommand::ConfirmTool { tool_id, updated_input } => remote_interaction_accepted_response(
            "confirm_tool",
            tool_id.clone(),
            host.confirm_tool(tool_id, updated_input.clone()).await,
        ),
        RemoteCommand::RejectTool { tool_id, reason } => {
            let reject_reason = reason.clone().unwrap_or_else(|| "User rejected".to_string());
            remote_interaction_accepted_response(
                "reject_tool",
                tool_id.clone(),
                host.reject_tool(tool_id, reject_reason).await,
            )
        }
        RemoteCommand::CancelTool { tool_id, reason } => {
            let cancel_reason = reason.clone().unwrap_or_else(|| "User cancelled".to_string());
            remote_interaction_accepted_response(
                "cancel_tool",
                tool_id.clone(),
                host.cancel_tool(tool_id, cancel_reason).await,
            )
        }
        RemoteCommand::AnswerQuestion { tool_id, answers } => {
            remote_answer_question_response(host.answer_question(tool_id, answers.clone()))
        }
        _ => RemoteResponse::Error {
            message: "Unknown execution command".to_string(),
        },
    }
}

pub async fn handle_remote_workspace_command<H>(
    host: &H,
    command: &crate::remote_connect::RemoteCommand,
) -> RemoteResponse
where
    H: RemoteWorkspaceRuntimeHost + ?Sized,
{
    use super::RemoteCommand;
    match command {
        RemoteCommand::GetWorkspaceInfo => remote_workspace_info_response(host.current_workspace().await),
        RemoteCommand::ListRecentWorkspaces => remote_recent_workspaces_response(host.recent_workspaces().await),
        RemoteCommand::SetWorkspace { path } => remote_workspace_updated_response(host.open_workspace(path).await),
        RemoteCommand::ListAssistants => remote_assistant_list_response(host.assistant_workspaces().await),
        RemoteCommand::SetAssistant { path } => {
            remote_assistant_updated_response(host.open_assistant_workspace(path).await)
        }
        _ => RemoteResponse::Error {
            message: "Unknown workspace command".into(),
        },
    }
}

pub async fn handle_remote_workspace_file_command<H>(
    host: &H,
    command: &crate::remote_connect::RemoteCommand,
) -> RemoteResponse
where
    H: RemoteWorkspaceFileRuntimeHost + ?Sized,
{
    use super::RemoteCommand;
    match command {
        RemoteCommand::ReadFile { path, session_id } => {
            let workspace_root = host.resolve_remote_file_workspace_root(session_id.as_deref()).await;
            remote_file_content_response(
                read_remote_workspace_file(path, REMOTE_FILE_MAX_READ_BYTES, workspace_root.as_deref()).await,
            )
        }
        RemoteCommand::ReadFileChunk {
            path,
            session_id,
            offset,
            limit,
        } => {
            let workspace_root = host.resolve_remote_file_workspace_root(session_id.as_deref()).await;
            remote_file_chunk_response(
                read_remote_workspace_file_chunk(path, workspace_root.as_deref(), *offset, *limit).await,
            )
        }
        RemoteCommand::GetFileInfo { path, session_id } => {
            let workspace_root = host.resolve_remote_file_workspace_root(session_id.as_deref()).await;
            remote_file_info_response(read_remote_workspace_file_info(path, workspace_root.as_deref()).await)
        }
        _ => RemoteResponse::Error {
            message: "Unsupported remote workspace file command".to_string(),
        },
    }
}

// `handle_remote_session_command`, `handle_remote_poll_command`, and
// `generate_remote_initial_sync` are owned by `remote_session_response_builders`
// (they are tightly coupled with the session response DTOs and the
// `RemoteSessionRuntimeHost` / `RemotePollRuntimeHost` traits). The
// integration tests below import them from that sibling.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use northhing_runtime_ports::RemoteSessionMetadata as RuntimeRemoteSessionMetadata;
    use northhing_runtime_ports::{
        AgentSessionCreateRequest, RemoteWorkspaceFacts, RemoteWorkspaceKind, RemoteWorkspaceRuntimeHost,
        RemoteWorkspaceUpdate,
    };

    use crate::remote_connect::remote_request_builders::{ChatMessage, RemoteModelCatalog};
    use crate::remote_connect::remote_session_response_builders::{
        handle_remote_poll_command, handle_remote_session_command, RemotePollRuntimeHost, RemoteSessionRuntimeHost,
    };
    use crate::remote_connect::remote_session_state::RemoteSessionStateTracker;
    use crate::remote_connect::RemoteCommand;

    struct FakeWorkspaceHost;

    #[async_trait::async_trait]
    impl RemoteWorkspaceRuntimeHost for FakeWorkspaceHost {
        async fn current_workspace(&self) -> Option<RemoteWorkspaceFacts> {
            Some(RemoteWorkspaceFacts {
                path: "/workspace/project".to_string(),
                name: "project".to_string(),
                git_branch: Some("main".to_string()),
                kind: RemoteWorkspaceKind::Normal,
                assistant_id: None,
            })
        }

        async fn recent_workspaces(&self) -> Vec<RemoteRecentWorkspaceFacts> {
            vec![RemoteRecentWorkspaceFacts {
                path: "/workspace/project".to_string(),
                name: "project".to_string(),
                last_opened: "2026-05-29T00:00:00Z".to_string(),
                kind: RemoteWorkspaceKind::Normal,
            }]
        }

        async fn open_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
            Ok(RemoteWorkspaceUpdate {
                path: path.to_string(),
                name: "opened".to_string(),
            })
        }

        async fn assistant_workspaces(&self) -> Vec<RemoteAssistantWorkspaceFacts> {
            vec![RemoteAssistantWorkspaceFacts {
                path: "/workspace/assistant".to_string(),
                name: "assistant".to_string(),
                assistant_id: None,
            }]
        }

        async fn open_assistant_workspace(&self, path: &str) -> Result<RemoteWorkspaceUpdate, String> {
            Ok(RemoteWorkspaceUpdate {
                path: path.to_string(),
                name: "assistant".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn remote_workspace_handler_preserves_response_shapes() {
        let host = FakeWorkspaceHost;

        assert_eq!(
            handle_remote_workspace_command(&host, &RemoteCommand::GetWorkspaceInfo).await,
            RemoteResponse::WorkspaceInfo {
                has_workspace: true,
                path: Some("/workspace/project".to_string()),
                project_name: Some("project".to_string()),
                git_branch: Some("main".to_string()),
                workspace_kind: Some("normal".to_string()),
                assistant_id: None,
            }
        );

        assert_eq!(
            handle_remote_workspace_command(
                &host,
                &RemoteCommand::SetWorkspace {
                    path: "/workspace/next".to_string(),
                },
            )
            .await,
            RemoteResponse::WorkspaceUpdated {
                success: true,
                path: Some("/workspace/next".to_string()),
                project_name: Some("opened".to_string()),
                error: None,
            }
        );
    }

    #[derive(Default)]
    struct FakeSessionHost {
        created_requests: Mutex<Vec<AgentSessionCreateRequest>>,
        removed_trackers: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl RemoteSessionRuntimeHost for FakeSessionHost {
        async fn list_session_metadata(
            &self,
            _workspace_path: &Path,
        ) -> Result<Vec<RuntimeRemoteSessionMetadata>, String> {
            Ok(vec![
                RuntimeRemoteSessionMetadata {
                    session_id: "session-a".to_string(),
                    name: "keep me".to_string(),
                    agent_type: "agentic".to_string(),
                    created_at_ms: 1_000,
                    last_active_at_ms: 2_000,
                    turn_count: 3,
                },
                RuntimeRemoteSessionMetadata {
                    session_id: "session-b".to_string(),
                    name: "other".to_string(),
                    agent_type: "agentic".to_string(),
                    created_at_ms: 1_000,
                    last_active_at_ms: 2_000,
                    turn_count: 1,
                },
            ])
        }

        async fn resolve_default_assistant_workspace_path(&self) -> Result<String, String> {
            Ok("/workspace/assistant".to_string())
        }

        async fn create_session(&self, request: AgentSessionCreateRequest) -> Result<String, String> {
            self.created_requests.lock().unwrap().push(request);
            Ok("created-session".to_string())
        }

        async fn load_model_catalog(&self, _session_id: Option<&str>) -> Result<RemoteModelCatalog, String> {
            Ok(RemoteModelCatalog {
                version: 1,
                models: Vec::new(),
                default_models: crate::remote_connect::remote_request_builders::RemoteDefaultModelsConfig::default(),
                session_model_id: None,
            })
        }

        async fn update_session_model(&self, _session_id: &str, model_id: &str) -> Result<String, String> {
            Ok(model_id.to_string())
        }

        async fn ensure_session_loaded(&self, _session_id: &str) -> Result<(), String> {
            Ok(())
        }

        async fn update_session_title(&self, _session_id: &str, title: &str) -> Result<String, String> {
            Ok(title.trim().to_string())
        }

        async fn resolve_session_workspace_path(&self, _session_id: &str) -> Option<PathBuf> {
            Some(PathBuf::from("/workspace/project"))
        }

        async fn load_remote_chat_messages(
            &self,
            _workspace_path: &Path,
            _session_id: &str,
        ) -> (Vec<ChatMessage>, bool) {
            (
                vec![ChatMessage {
                    id: "message-1".to_string(),
                    role: "user".to_string(),
                    content: "hello".to_string(),
                    timestamp: "1".to_string(),
                    metadata: None,
                    images: None,
                    thinking: None,
                    tools: None,
                    items: None,
                }],
                false,
            )
        }

        async fn delete_session(&self, _workspace_path: &Path, _session_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn remove_tracker(&self, session_id: &str) {
            self.removed_trackers.lock().unwrap().push(session_id.to_string());
        }
    }

    #[tokio::test]
    async fn remote_session_handler_preserves_list_and_create_policy() {
        let host = FakeSessionHost::default();

        let list = handle_remote_session_command(
            &host,
            &RemoteCommand::ListSessions {
                workspace_path: Some("/workspace/project".to_string()),
                limit: Some(20),
                offset: Some(0),
                query: Some("keep".to_string()),
            },
        )
        .await;
        let RemoteResponse::SessionList { sessions, has_more } = list else {
            panic!("expected session list");
        };
        assert!(!has_more);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "session-a");
        assert_eq!(sessions[0].workspace_path.as_deref(), Some("/workspace/project"));

        let created = handle_remote_session_command(
            &host,
            &RemoteCommand::CreateSession {
                agent_type: Some("Cowork".to_string()),
                session_name: None,
                workspace_path: Some("/workspace/project".to_string()),
            },
        )
        .await;
        assert_eq!(
            created,
            RemoteResponse::SessionCreated {
                session_id: "created-session".to_string(),
            }
        );
        let created_requests = host.created_requests.lock().unwrap();
        assert_eq!(created_requests[0].session_name, "Remote Cowork Session");
        assert_eq!(created_requests[0].agent_type, "Cowork");
        assert_eq!(
            created_requests[0].workspace_path.as_deref(),
            Some("/workspace/project")
        );
    }

    #[tokio::test]
    async fn remote_session_handler_removes_tracker_after_delete_success() {
        let host = FakeSessionHost::default();

        let deleted = handle_remote_session_command(
            &host,
            &RemoteCommand::DeleteSession {
                session_id: "session-a".to_string(),
            },
        )
        .await;

        assert_eq!(
            deleted,
            RemoteResponse::SessionDeleted {
                session_id: "session-a".to_string(),
            }
        );
        assert_eq!(host.removed_trackers.lock().unwrap().as_slice(), ["session-a"]);
    }

    struct FakePollHost {
        tracker: Arc<RemoteSessionStateTracker>,
    }

    #[async_trait::async_trait]
    impl RemotePollRuntimeHost for FakePollHost {
        fn ensure_tracker(&self, _session_id: &str) -> Arc<RemoteSessionStateTracker> {
            self.tracker.clone()
        }

        async fn load_model_catalog(&self, _session_id: &str) -> Option<RemoteModelCatalog> {
            None
        }

        async fn resolve_session_workspace_path(&self, _session_id: &str) -> Option<PathBuf> {
            None
        }

        async fn load_remote_chat_messages(
            &self,
            _workspace_path: &Path,
            _session_id: &str,
        ) -> (Vec<ChatMessage>, bool) {
            (Vec::new(), false)
        }
    }

    #[tokio::test]
    async fn remote_poll_handler_preserves_missing_workspace_error() {
        let host = FakePollHost {
            tracker: Arc::new(RemoteSessionStateTracker::new("session-a".to_string())),
        };

        let response = handle_remote_poll_command(
            &host,
            &RemoteCommand::PollSession {
                session_id: "session-a".to_string(),
                since_version: 0,
                known_msg_count: 0,
                known_model_catalog_version: None,
            },
        )
        .await;

        assert_eq!(
            response,
            RemoteResponse::Error {
                message: "Workspace path not available for session: session-a".to_string(),
            }
        );
    }

    #[derive(Default)]
    struct FakeInteractionHost {
        rejected: Mutex<Vec<(String, String)>>,
    }

    #[async_trait::async_trait]
    impl RemoteInteractionRuntimeHost for FakeInteractionHost {
        async fn confirm_tool(&self, _tool_id: &str, _updated_input: Option<serde_json::Value>) -> Result<(), String> {
            Ok(())
        }

        async fn reject_tool(&self, tool_id: &str, reason: String) -> Result<(), String> {
            self.rejected.lock().unwrap().push((tool_id.to_string(), reason));
            Ok(())
        }

        async fn cancel_tool(&self, _tool_id: &str, _reason: String) -> Result<(), String> {
            Ok(())
        }

        fn answer_question(&self, _tool_id: &str, _answers: serde_json::Value) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn remote_interaction_handler_preserves_default_reject_reason() {
        let host = FakeInteractionHost::default();

        let response = handle_remote_interaction_command(
            &host,
            &RemoteCommand::RejectTool {
                tool_id: "tool-1".to_string(),
                reason: None,
            },
        )
        .await;

        assert_eq!(
            response,
            RemoteResponse::InteractionAccepted {
                action: "reject_tool".to_string(),
                target_id: "tool-1".to_string(),
            }
        );
        assert_eq!(
            host.rejected.lock().unwrap().as_slice(),
            [("tool-1".to_string(), "User rejected".to_string())]
        );
    }
}
