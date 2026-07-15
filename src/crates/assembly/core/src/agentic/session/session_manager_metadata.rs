//! Round 9 split sibling: session_manager_metadata
//!
//! Auto-extracted from session_manager.rs (29 methods).
//! Methods declared `pub(crate)` so external callers and other modules can use them.

use super::session_manager::SessionManager;

use crate::agentic::core::{
    new_turn_id, CompressionContract, CompressionState, InternalReminderKind, Message, MessageSemanticKind,
    ProcessingPhase, Session, SessionConfig, SessionKind, SessionState, SessionSummary, TurnStats,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::session_store_port::CoreSessionStorePort;
use crate::agentic::session::{
    CachedSystemPrompt, CachedUserContext, EvidenceLedgerCheckpoint, EvidenceLedgerEvent, EvidenceLedgerEventStatus,
    EvidenceLedgerSummary, EvidenceLedgerTargetKind, FileReadState, FileReadStateStore, PromptCacheLookup,
    PromptCachePolicy, PromptCacheScope, SessionContextStore, SessionEvidenceLedger, SessionPromptCache,
    SessionPromptCacheStore, SystemPromptCacheIdentity, TurnSkillAgentSnapshotStore, UserContextCacheIdentity,
};
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::{
    get_app_language_code, get_global_config_service, short_model_user_language_instruction, subscribe_config_updates,
    ConfigUpdateEvent,
};
use crate::service::session::{
    DialogTurnData, DialogTurnKind, ModelRoundData, SessionMetadata, SessionRelationship, TextItemData, TurnStatus,
    UserMessageData,
};
use crate::service::snapshot::ensure_snapshot_manager_for_workspace;
use crate::service::workspace::global_workspace_service;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::sanitize_plain_model_output;
use crate::util::timing::elapsed_ms_u64;
use dashmap::DashMap;
pub use northhing_runtime_ports::SessionViewRestoreTiming;
use northhing_runtime_ports::{SessionStoragePathRequest, SessionStorePort, SessionViewRestoreRequest};
use northhing_services_core::session::{
    apply_session_lineage, collect_hidden_subagent_cascade as collect_hidden_subagent_cascade_ids,
    merge_session_custom_metadata as merge_session_custom_metadata_value, set_deep_review_run_manifest,
    set_session_relationship,
};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::time::{Duration, SystemTime};
use tokio::time;
use tracing::{debug, error, info, warn};

impl SessionManager {
    pub(crate) async fn update_session_title(&self, session_id: &str, title: &str) -> NortHingResult<()> {
        let normalized_title = Self::normalize_session_title_input(title)?;
        let workspace_path = self.effective_session_workspace_path(session_id).await;

        {
            let Some(mut session) = self.sessions.get_mut(session_id) else {
                return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
            };
            session.session_name = normalized_title.clone();
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
        }

        if self.should_persist_session_id(session_id) {
            let Some(workspace_path) = workspace_path.as_ref() else {
                return Err(NortHingError::Session(format!(
                    "Workspace path is unavailable for session {}",
                    session_id
                )));
            };
            // Clone the session data out of the DashMap guard before awaiting I/O.
            let session_snapshot = {
                let Some(session) = self.sessions.get(session_id) else {
                    return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
                };
                session.clone()
            };
            // Ref guard released -- DashMap shard lock is free.
            self.persistence_manager
                .save_session(workspace_path, &session_snapshot)
                .await?;
        }

        info!(
            "Session title updated: session_id={}, title={}",
            session_id, normalized_title
        );

        Ok(())
    }

    pub(crate) async fn update_session_title_if_current(
        &self,
        session_id: &str,
        expected_current_title: &str,
        title: &str,
    ) -> NortHingResult<bool> {
        let Some(session) = self.sessions.get(session_id) else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        };

        if session.session_name != expected_current_title {
            debug!(
                "Skipping auto-generated title because current title changed: session_id={}, expected_title={}, current_title={}",
                session_id,
                expected_current_title,
                session.session_name
            );
            return Ok(false);
        }
        drop(session);

        self.update_session_title(session_id, title).await?;
        Ok(true)
    }

    pub(crate) async fn update_session_agent_type(&self, session_id: &str, agent_type: &str) -> NortHingResult<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.agent_type = agent_type.to_string();
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        }

        if self.should_persist_session_id(session_id) {
            let effective_path = self.effective_session_workspace_path(session_id).await;
            let session_snapshot = self.sessions.get(session_id).map(|s| s.clone());
            // Ref guard released -- DashMap shard lock is free.
            if let (Some(workspace_path), Some(session)) = (effective_path, session_snapshot) {
                self.persistence_manager.save_session(&workspace_path, &session).await?;
            }
        }

        debug!(
            "Session agent type updated: session_id={}, agent_type={}",
            session_id, agent_type
        );

        Ok(())
    }

    pub(crate) async fn update_last_submitted_agent_type(
        &self,
        session_id: &str,
        agent_type: &str,
    ) -> NortHingResult<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.last_submitted_agent_type = Some(agent_type.to_string());
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        }

        if self.should_persist_session_id(session_id) {
            let effective_path = self.effective_session_workspace_path(session_id).await;
            let session_snapshot = self.sessions.get(session_id).map(|s| s.clone());
            if let (Some(workspace_path), Some(session)) = (effective_path, session_snapshot) {
                self.persistence_manager.save_session(&workspace_path, &session).await?;
            }
        }

        debug!(
            "Session last submitted agent type updated: session_id={}, agent_type={}",
            session_id, agent_type
        );

        Ok(())
    }

    pub(crate) fn derive_last_user_dialog_agent_type_from_turns(
        turns: &[DialogTurnData],
        fallback_agent_type: Option<&str>,
    ) -> Option<String> {
        // New turns persist their mode on the turn itself. For older persisted
        // sessions that predate this field, fall back to the session default
        // only when at least one surviving user dialog turn exists.
        turns
            .iter()
            .rev()
            .find(|turn| turn.kind == DialogTurnKind::UserDialog)
            .and_then(|turn| {
                turn.agent_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
            .or_else(|| {
                if turns.iter().any(|turn| turn.kind == DialogTurnKind::UserDialog) {
                    fallback_agent_type
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                } else {
                    None
                }
            })
    }

    pub(crate) async fn update_session_model_id(&self, session_id: &str, model_id: &str) -> NortHingResult<()> {
        let ai_config = Self::load_ai_config_for_model_resolution().await;
        let mut resolved_context_window = None;

        // If the session was evicted from memory (idle > 1h), try to restore it
        // using the workspace path recorded when it was first created/restored.
        if !self.sessions.contains_key(session_id) && self.config.enable_persistence {
            let workspace_path = self.session_workspace_index.get(session_id).map(|entry| entry.clone());
            if let Some(workspace_path) = workspace_path {
                debug!(
                    "Session evicted from memory, restoring for model update: session_id={}",
                    session_id
                );
                if let Err(e) = self.restore_session(&workspace_path, session_id).await {
                    warn!(
                        target: "session::metadata",
                        "session restore on eviction failed: session_id={}, workspace_path={}, error={}",
                        session_id,
                        workspace_path.display(),
                        e,
                    );
                }
            }
        }

        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.config.model_id = Some(model_id.to_string());
            if let Some(ai_config) = ai_config.as_ref() {
                resolved_context_window = Self::sync_session_context_window_from_ai_config(&mut session, ai_config);
            }
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        }

        if self.should_persist_session_id(session_id) {
            let effective_path = self.effective_session_workspace_path(session_id).await;
            let session_snapshot = self.sessions.get(session_id).map(|s| s.clone());
            // Ref guard released -- DashMap shard lock is free.
            if let (Some(workspace_path), Some(session)) = (effective_path, session_snapshot) {
                self.persistence_manager.save_session(&workspace_path, &session).await?;
            }
        }

        debug!(
            "Session model id updated: session_id={}, model_id={}, max_context_tokens={:?}",
            session_id, model_id, resolved_context_window
        );

        Ok(())
    }

    pub(crate) async fn refresh_session_context_window(&self, session_id: &str) -> NortHingResult<()> {
        if let Some(ai_config) = Self::load_ai_config_for_model_resolution().await {
            if let Some(mut session) = self.sessions.get_mut(session_id) {
                let previous = session.config.max_context_tokens;
                Self::sync_session_context_window_from_ai_config(&mut session, &ai_config);
                let updated = session.config.max_context_tokens;
                if updated != previous {
                    debug!(
                        "Refreshed session context window: session_id={}, previous={}, updated={}",
                        session_id, previous, updated
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) fn paginate_messages(
        messages: &[Message],
        limit: usize,
        before_message_id: Option<&str>,
    ) -> (Vec<Message>, bool) {
        if messages.is_empty() {
            return (vec![], false);
        }

        let end_idx = if let Some(before_id) = before_message_id {
            messages.iter().position(|m| m.id == before_id).unwrap_or(0)
        } else {
            messages.len()
        };

        if end_idx == 0 {
            return (vec![], false);
        }

        let start_idx = end_idx.saturating_sub(limit);
        let has_more = start_idx > 0;

        (messages[start_idx..end_idx].to_vec(), has_more)
    }

    pub(crate) async fn load_session_metadata(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<SessionMetadata>> {
        self.persistence_manager
            .load_session_metadata(workspace_path, session_id)
            .await
    }

    pub(crate) async fn save_session_metadata(
        &self,
        workspace_path: &Path,
        metadata: &SessionMetadata,
    ) -> NortHingResult<()> {
        self.persistence_manager
            .save_session_metadata(workspace_path, metadata)
            .await
    }

    pub(crate) async fn metadata_workspace_path_for_update(&self, session_id: &str) -> NortHingResult<PathBuf> {
        if !self.should_persist_session_id(session_id) {
            return Err(NortHingError::Validation(format!(
                "Session persistence is disabled: {}",
                session_id
            )));
        }

        self.effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))
    }

    pub(crate) async fn load_or_persist_session_metadata(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<SessionMetadata> {
        match self
            .persistence_manager
            .load_session_metadata(workspace_path, session_id)
            .await?
        {
            Some(metadata) => Ok(metadata),
            None => {
                let session = self
                    .sessions
                    .get(session_id)
                    .map(|value| value.clone())
                    .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;
                self.persistence_manager.save_session(workspace_path, &session).await?;
                self.persistence_manager
                    .load_session_metadata(workspace_path, session_id)
                    .await?
                    .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))
            }
        }
    }

    pub(crate) async fn update_session_metadata_at_workspace(
        &self,
        workspace_path: &Path,
        session_id: &str,
        update: impl FnOnce(&mut SessionMetadata),
    ) -> NortHingResult<()> {
        let mut metadata = self
            .load_or_persist_session_metadata(workspace_path, session_id)
            .await?;
        update(&mut metadata);
        self.persistence_manager
            .save_session_metadata(workspace_path, &metadata)
            .await
    }

    pub(crate) async fn update_persisted_session_metadata(
        &self,
        session_id: &str,
        update: impl FnOnce(&mut SessionMetadata),
    ) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            return Ok(());
        }

        let workspace_path = self.metadata_workspace_path_for_update(session_id).await?;
        self.update_session_metadata_at_workspace(&workspace_path, session_id, update)
            .await
    }

    pub(crate) async fn merge_session_custom_metadata(
        &self,
        session_id: &str,
        patch: serde_json::Value,
    ) -> NortHingResult<()> {
        self.update_persisted_session_metadata(session_id, |metadata| {
            merge_session_custom_metadata_value(metadata, patch)
        })
        .await
    }

    pub(crate) async fn merge_session_relationship(
        &self,
        session_id: &str,
        relationship: SessionRelationship,
    ) -> NortHingResult<()> {
        self.update_persisted_session_metadata(session_id, |metadata| set_session_relationship(metadata, relationship))
            .await
    }

    pub(crate) async fn persist_session_lineage(
        &self,
        session_id: &str,
        relationship: SessionRelationship,
    ) -> NortHingResult<()> {
        self.update_persisted_session_metadata(session_id, |metadata| apply_session_lineage(metadata, relationship))
            .await
    }

    pub(crate) async fn collect_hidden_subagent_cascade_for_parent_turns(
        &self,
        workspace_path: &Path,
        parent_session_id: &str,
        parent_dialog_turn_ids: &HashSet<String>,
    ) -> NortHingResult<Vec<String>> {
        if parent_session_id.trim().is_empty() || parent_dialog_turn_ids.is_empty() {
            return Ok(Vec::new());
        }

        let metadata_list = self
            .persistence_manager
            .list_session_metadata_including_internal(workspace_path)
            .await?;
        Ok(collect_hidden_subagent_cascade_ids(
            metadata_list,
            parent_session_id,
            parent_dialog_turn_ids,
        ))
    }

    pub(crate) async fn set_session_deep_review_run_manifest(
        &self,
        session_id: &str,
        deep_review_run_manifest: Option<serde_json::Value>,
    ) -> NortHingResult<()> {
        self.update_persisted_session_metadata(session_id, |metadata| {
            set_deep_review_run_manifest(metadata, deep_review_run_manifest)
        })
        .await
    }

    pub(crate) async fn get_messages(&self, session_id: &str) -> NortHingResult<Vec<Message>> {
        if self.config.enable_persistence {
            if let Some(workspace_path) = self.effective_session_workspace_path(session_id).await {
                let messages = self.rebuild_messages_from_turns(&workspace_path, session_id).await?;
                if !messages.is_empty() {
                    return Ok(messages);
                }
            }
        }

        Ok(self.context_store.get_context_messages(session_id))
    }

    pub(crate) async fn get_messages_paginated(
        &self,
        session_id: &str,
        limit: usize,
        before_message_id: Option<&str>,
    ) -> NortHingResult<(Vec<Message>, bool)> {
        let messages = self.get_messages(session_id).await?;
        Ok(Self::paginate_messages(&messages, limit, before_message_id))
    }

    pub(crate) async fn get_context_messages(&self, session_id: &str) -> NortHingResult<Vec<Message>> {
        let context_messages = self.context_store.get_context_messages(session_id);

        Ok(context_messages)
    }

    pub(crate) async fn add_message(&self, session_id: &str, message: Message) -> NortHingResult<()> {
        self.context_store.add_message(session_id, message);
        self.persist_current_turn_context_snapshot_best_effort(session_id, "context_message_added")
            .await;
        Ok(())
    }

    pub(crate) async fn replace_context_messages(&self, session_id: &str, messages: Vec<Message>) {
        self.context_store.replace_context(session_id, messages);
        self.file_read_state_store.clear_session(session_id);
        self.persist_current_turn_context_snapshot_best_effort(session_id, "context_replaced")
            .await;
    }

    pub(crate) fn set_file_read_state(&self, session_id: &str, logical_path: &str, state: FileReadState) {
        self.file_read_state_store.set(session_id, logical_path, state);
    }

    pub(crate) fn get_file_read_state(&self, session_id: &str, logical_path: &str) -> Option<FileReadState> {
        self.file_read_state_store.get(session_id, logical_path)
    }

    pub(crate) fn get_turn_count(&self, session_id: &str) -> usize {
        self.sessions
            .get(session_id)
            .map(|s| s.dialog_turn_ids.len())
            .unwrap_or(0)
    }

    pub(crate) fn get_compression_state(&self, session_id: &str) -> Option<CompressionState> {
        self.sessions.get(session_id).map(|s| s.compression_state.clone())
    }

    pub(crate) async fn update_compression_state(
        &self,
        session_id: &str,
        compression_state: CompressionState,
    ) -> NortHingResult<()> {
        let effective_path = self.effective_session_workspace_path(session_id).await;

        // IMPORTANT: keep the DashMap guard scope short -- do NOT hold it across .await.
        let session_snapshot = if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.compression_state = compression_state;
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
            if self.config.enable_persistence && Self::should_persist_session(&session) {
                Some(session.clone())
            } else {
                None
            }
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        };
        // RefMut guard released here -- DashMap shard lock is free.

        if let Some(session) = session_snapshot {
            if let Some(ref workspace_path) = effective_path {
                self.persistence_manager.save_session(workspace_path, &session).await?;
            }
        }

        Ok(())
    }
}
