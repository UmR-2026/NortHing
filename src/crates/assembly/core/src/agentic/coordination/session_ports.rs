//! Session-related runtime ports for `ConversationCoordinator`.

use super::coordinator::ConversationCoordinator;
use crate::agentic::core::MessageRole;
use northhing_runtime_ports::{
    AgentSessionDeleteRequest, AgentSessionListRequest, AgentSessionManagementPort, AgentSessionWorkspaceRequest,
    SessionTranscript, SessionTranscriptReader, SessionTranscriptRequest,
};
use std::path::Path;

#[async_trait::async_trait]
impl AgentSessionManagementPort for ConversationCoordinator {
    async fn list_sessions(
        &self,
        request: AgentSessionListRequest,
    ) -> northhing_runtime_ports::PortResult<Vec<northhing_runtime_ports::AgentSessionSummary>> {
        self.list_sessions(Path::new(&request.workspace_path))
            .await
            .map(|sessions| {
                sessions
                    .into_iter()
                    .map(super::subagent_ports::runtime_session_summary)
                    .collect::<Vec<_>>()
            })
            .map_err(|error| {
                northhing_runtime_ports::PortError::new(
                    northhing_runtime_ports::PortErrorKind::Backend,
                    error.to_string(),
                )
            })
    }

    async fn delete_session(&self, request: AgentSessionDeleteRequest) -> northhing_runtime_ports::PortResult<()> {
        self.delete_session(Path::new(&request.workspace_path), &request.session_id)
            .await
            .map_err(|error| {
                northhing_runtime_ports::PortError::new(
                    northhing_runtime_ports::PortErrorKind::Backend,
                    error.to_string(),
                )
            })
    }

    async fn resolve_session_workspace_path(
        &self,
        request: AgentSessionWorkspaceRequest,
    ) -> northhing_runtime_ports::PortResult<Option<String>> {
        Ok(self
            .resolve_session_workspace_path(&request.session_id)
            .await
            .map(|path| path.to_string_lossy().into_owned()))
    }
}

#[async_trait::async_trait]
impl SessionTranscriptReader for ConversationCoordinator {
    async fn read_session_transcript(
        &self,
        request: SessionTranscriptRequest,
    ) -> northhing_runtime_ports::PortResult<SessionTranscript> {
        let messages = self.get_messages(&request.session_id).await.map_err(|error| {
            northhing_runtime_ports::PortError::new(northhing_runtime_ports::PortErrorKind::Backend, error.to_string())
        })?;

        let messages = messages
            .into_iter()
            .filter(|message| match request.turn_id.as_ref() {
                Some(turn_id) => message.metadata.turn_id.as_ref() == Some(turn_id),
                None => true,
            })
            .map(|message| {
                let role = match message.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                    MessageRole::System => "system",
                }
                .to_string();

                northhing_runtime_ports::TranscriptMessage {
                    role,
                    turn_id: message.metadata.turn_id,
                    content: serde_json::to_value(message.content).unwrap_or_default(),
                }
            })
            .collect();

        Ok(SessionTranscript {
            session_id: request.session_id,
            messages,
        })
    }
}
