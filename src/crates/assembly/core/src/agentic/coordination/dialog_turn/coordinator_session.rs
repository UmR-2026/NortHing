//! Sub-domain: session.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains public session lifecycle methods on `ConversationCoordinator`:
//! create / update / delete / list / resolve / get_messages / subscribe / tool control.
//!
//! Thin wrappers that delegate to the helper methods in the `session` sibling.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;
use super::super::turn_outcome::TurnOutcome;

use crate::agentic::core::{Message, Session, SessionConfig, SessionSummary};
use crate::agentic::events::{AgenticEvent, EventSubscriber};
use crate::util::errors::{NortHingError, NortHingResult};
use std::collections::HashSet;
use std::path::Path;
use tracing::info;

impl ConversationCoordinator {
    /// Create a new session
    pub async fn create_session(
        &self,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
    ) -> NortHingResult<Session> {
        let workspace_path = config.workspace_path.clone().ok_or_else(|| {
            NortHingError::Validation("workspace_path is required when creating a session".to_string())
        })?;
        self.create_session_with_workspace_and_creator(None, session_name, agent_type, config, workspace_path, None)
            .await
    }

    /// Create a new session with optional session ID
    pub async fn create_session_with_id(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
    ) -> NortHingResult<Session> {
        let workspace_path = config.workspace_path.clone().ok_or_else(|| {
            NortHingError::Validation("workspace_path is required when creating a session".to_string())
        })?;
        self.create_session_with_workspace_and_creator(
            session_id,
            session_name,
            agent_type,
            config,
            workspace_path,
            None,
        )
        .await
    }

    /// Create a new session with optional session ID and workspace binding.
    /// `workspace_path` is forwarded in the `SessionCreated` event and also stored
    /// in the session's in-memory config so it can be retrieved without disk access.
    pub async fn create_session_with_workspace(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
        workspace_path: String,
    ) -> NortHingResult<Session> {
        self.create_session_with_workspace_and_creator(
            session_id,
            session_name,
            agent_type,
            config,
            workspace_path,
            None,
        )
        .await
    }

    pub async fn update_session_model(&self, session_id: &str, model_id: &str) -> NortHingResult<()> {
        let normalized_model_id = model_id.trim();
        let normalized_model_id = if normalized_model_id.is_empty() {
            "auto"
        } else {
            normalized_model_id
        };

        self.session_manager
            .update_session_model_id(session_id, normalized_model_id)
            .await?;

        info!(
            "Coordinator updated session model: session_id={}, model_id={}",
            session_id, normalized_model_id
        );

        Ok(())
    }

    /// Create a new session with explicit creator identity.
    pub async fn create_session_with_workspace_and_creator(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        mut config: SessionConfig,
        workspace_path: String,
        created_by: Option<String>,
    ) -> NortHingResult<Session> {
        // Persist the workspace binding inside the session config so execution can
        // consistently restore the correct workspace regardless of the entry point.
        config.workspace_path = Some(workspace_path.clone());
        config.workspace_id = Self::resolve_workspace_id_for_config(&config).await;
        let agent_type = Self::normalize_agent_type(&agent_type);
        let session = self
            .session_manager
            .create_session_with_id_and_creator(session_id, session_name, agent_type, config, created_by)
            .await?;

        Self::track_session_workspace_activity_best_effort(&session.config, "session_created").await;

        // SessionManager::create_session_with_id_and_creator already persists the
        // session into the effective workspace session storage path. Avoid writing
        // a second copy here using the raw workspace path, because remote workspaces
        // resolve to a different effective storage path and double-writing can leave
        // metadata/turn files split across two locations.

        self.emit_event(AgenticEvent::SessionCreated {
            session_id: session.session_id.clone(),
            session_name: session.session_name.clone(),
            agent_type: session.agent_type.clone(),
            workspace_path: Some(workspace_path),
            remote_connection_id: session.config.remote_connection_id.clone(),
            remote_ssh_host: session.config.remote_ssh_host.clone(),
        })
        .await;
        Ok(session)
    }

    /// Create a hidden internal subagent session that is persisted but excluded
    /// from normal user-facing session lists.
    pub async fn create_hidden_subagent_session_with_workspace(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        mut config: SessionConfig,
        workspace_path: String,
        created_by: Option<String>,
    ) -> NortHingResult<Session> {
        config.workspace_path = Some(workspace_path);
        config.workspace_id = Self::resolve_workspace_id_for_config(&config).await;
        let agent_type = Self::normalize_agent_type(&agent_type);
        self.create_hidden_subagent_session(session_id, session_name, agent_type, config, created_by)
            .await
    }

    /// Delete session
    pub async fn delete_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        self.delete_session_impl(workspace_path, session_id).await
    }

    pub async fn delete_hidden_subagent_sessions_for_parent_turns(
        &self,
        workspace_path: &Path,
        parent_session_id: &str,
        parent_dialog_turn_ids: &HashSet<String>,
    ) -> NortHingResult<Vec<String>> {
        self.delete_hidden_subagent_sessions_for_parent_turns_impl(
            workspace_path,
            parent_session_id,
            parent_dialog_turn_ids,
        )
        .await
    }

    /// List all sessions
    pub async fn list_sessions(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>> {
        self.list_sessions_impl(workspace_path).await
    }

    pub async fn resolve_session_workspace_path(&self, session_id: &str) -> Option<std::path::PathBuf> {
        self.resolve_session_workspace_path_impl(session_id).await
    }

    /// Get a best-effort message view for a session.
    pub async fn get_messages(&self, session_id: &str) -> NortHingResult<Vec<Message>> {
        self.get_messages_impl(session_id).await
    }

    /// Get a paginated best-effort message view for a session.
    pub async fn get_messages_paginated(
        &self,
        session_id: &str,
        limit: usize,
        before_message_id: Option<&str>,
    ) -> NortHingResult<(Vec<Message>, bool)> {
        self.get_messages_paginated_impl(session_id, limit, before_message_id)
            .await
    }

    /// Subscribe to internal events
    ///
    /// For internal systems to subscribe to events (e.g., logging, monitoring)
    pub fn subscribe_internal<H>(&self, subscriber_id: String, handler: H)
    where
        H: EventSubscriber + 'static,
    {
        self.subscribe_internal_impl(subscriber_id, handler);
    }

    /// Unsubscribe from internal events
    ///
    /// Remove subscriber previously added via subscribe_internal
    pub fn unsubscribe_internal(&self, subscriber_id: &str) {
        self.unsubscribe_internal_impl(subscriber_id);
    }

    /// Confirm tool execution
    pub async fn confirm_tool(&self, tool_id: &str, updated_input: Option<serde_json::Value>) -> NortHingResult<()> {
        self.confirm_tool_impl(tool_id, updated_input).await
    }

    /// Reject tool execution
    pub async fn reject_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        self.reject_tool_impl(tool_id, reason).await
    }

    /// Cancel tool execution
    pub async fn cancel_tool(&self, tool_id: &str, reason: String) -> NortHingResult<()> {
        self.cancel_tool_impl(tool_id, reason).await
    }
}
