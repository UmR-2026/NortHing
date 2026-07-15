use crate::agentic::core::{
    new_turn_id, CompressionContract, CompressionState, InternalReminderKind, Message, MessageSemanticKind,
    ProcessingPhase, Session, SessionConfig, SessionKind, SessionState, SessionSummary, TurnStats,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::session::session_store_port::CoreSessionStorePort;
use crate::agentic::session::SessionManager;
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_persisted_turn(
        &self,
        session_id: &str,
        kind: DialogTurnKind,
        agent_type: Option<String>,
        user_input: String,
        turn_id: Option<String>,
        context_messages: Vec<Message>,
        processing_phase: ProcessingPhase,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<String> {
        let session = self
            .get_session(session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;
        let workspace_path = Self::effective_workspace_path_from_config(&session.config)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;

        let turn_index = session.dialog_turn_ids.len();
        let turn_id = new_turn_id(turn_id);

        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.dialog_turn_ids.push(turn_id.clone());
            if kind == DialogTurnKind::UserDialog {
                session.last_user_dialog_agent_type = agent_type.clone();
            }
            session.state = SessionState::Processing {
                current_turn_id: turn_id.clone(),
                phase: processing_phase,
            };
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();
        }

        for message in context_messages {
            self.context_store
                .add_message(session_id, message.with_turn_id(turn_id.clone()));
        }

        if self.should_persist_session_id(session_id) {
            let turn_data = DialogTurnData::new_with_kind(
                kind,
                turn_id.clone(),
                turn_index,
                session_id.to_string(),
                if kind == DialogTurnKind::UserDialog {
                    agent_type.clone()
                } else {
                    None
                },
                UserMessageData {
                    id: format!("{}-user", turn_id),
                    content: user_input,
                    timestamp: SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    metadata: user_message_metadata,
                },
            );

            // Clone the session data out of the DashMap guard before awaiting I/O.
            let session_snapshot = self.sessions.get(session_id).map(|s| s.clone());
            // Ref guard released -- DashMap shard lock is free.
            if let Some(session) = session_snapshot {
                self.persistence_manager.save_session(&workspace_path, &session).await?;
            }
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn_data)
                .await?;
        }

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn_index, "turn_started")
            .await;

        Ok(turn_id)
    }

    /// Start a new dialog turn
    /// turn_id: Optional frontend-specified ID, if None then backend generates
    /// Returns: turn_id
    pub(crate) async fn start_dialog_turn(
        &self,
        session_id: &str,
        agent_type: String,
        user_input: String,
        turn_id: Option<String>,
        image_contexts: Option<Vec<ImageContextData>>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<String> {
        let user_message = if let Some(images) = image_contexts.as_ref().filter(|v| !v.is_empty()).cloned() {
            Message::user_multimodal(user_input.clone(), images)
                .with_semantic_kind(MessageSemanticKind::ActualUserInput)
        } else {
            Message::user(user_input.clone()).with_semantic_kind(MessageSemanticKind::ActualUserInput)
        };

        let turn_id = self
            .start_persisted_turn(
                session_id,
                DialogTurnKind::UserDialog,
                Some(agent_type),
                user_input,
                turn_id,
                vec![user_message],
                ProcessingPhase::Starting,
                user_message_metadata,
            )
            .await?;

        debug!("Starting dialog turn: turn_id={}", turn_id);

        Ok(turn_id)
    }

    pub(crate) async fn start_dialog_turn_with_prepended_messages(
        &self,
        session_id: &str,
        agent_type: String,
        user_input: String,
        turn_id: Option<String>,
        image_contexts: Option<Vec<ImageContextData>>,
        prepended_messages: Vec<Message>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<String> {
        let user_message = if let Some(images) = image_contexts.as_ref().filter(|v| !v.is_empty()).cloned() {
            Message::user_multimodal(user_input.clone(), images)
                .with_semantic_kind(MessageSemanticKind::ActualUserInput)
        } else {
            Message::user(user_input.clone()).with_semantic_kind(MessageSemanticKind::ActualUserInput)
        };

        let mut context_messages = prepended_messages;
        context_messages.push(user_message);

        let turn_id = self
            .start_persisted_turn(
                session_id,
                DialogTurnKind::UserDialog,
                Some(agent_type),
                user_input,
                turn_id,
                context_messages,
                ProcessingPhase::Starting,
                user_message_metadata,
            )
            .await?;

        debug!("Starting dialog turn with prepended messages: turn_id={}", turn_id);

        Ok(turn_id)
    }

    /// Start a new dialog turn when the model-visible user message has already
    /// been inserted into runtime context by the caller.
    ///
    /// This is used by forked/hidden subagent flows that seed inherited context
    /// before they acquire a concrete dialog turn id. The turn still needs the
    /// normal persisted lifecycle (turn record, active turn bookkeeping, and
    /// context snapshot), but must not append a duplicate user message into the
    /// runtime context cache.
    pub(crate) async fn start_dialog_turn_with_existing_context(
        &self,
        session_id: &str,
        agent_type: String,
        user_input: String,
        turn_id: Option<String>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<String> {
        let turn_id = self
            .start_persisted_turn(
                session_id,
                DialogTurnKind::UserDialog,
                Some(agent_type),
                user_input,
                turn_id,
                Vec::new(),
                ProcessingPhase::Starting,
                user_message_metadata,
            )
            .await?;

        debug!("Starting dialog turn with existing context: turn_id={}", turn_id);

        Ok(turn_id)
    }

    /// Start a persisted maintenance turn that should not enter model-visible context.
    pub(crate) async fn start_maintenance_turn(
        &self,
        session_id: &str,
        display_message: String,
        turn_id: Option<String>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<String> {
        let turn_id = self
            .start_persisted_turn(
                session_id,
                DialogTurnKind::ManualCompaction,
                None,
                display_message,
                turn_id,
                Vec::new(),
                ProcessingPhase::Compacting,
                user_message_metadata,
            )
            .await?;

        debug!("Starting maintenance turn: turn_id={}", turn_id);

        Ok(turn_id)
    }

    /// Append a completed local command turn that should be persisted in user-facing
    /// history without entering model-visible runtime context.
    pub async fn append_completed_local_command_turn(
        &self,
        session_id: &str,
        content: String,
        turn_id: Option<String>,
        timestamp_ms: Option<u64>,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<DialogTurnData> {
        let session = self
            .get_session(session_id)
            .ok_or_else(|| NortHingError::NotFound(format!("Session not found: {}", session_id)))?;
        let workspace_path = Self::effective_workspace_path_from_config(&session.config)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;

        let turn_id = new_turn_id(turn_id);
        let turn_index = session
            .dialog_turn_ids
            .iter()
            .position(|existing| existing == &turn_id)
            .unwrap_or(session.dialog_turn_ids.len());
        let timestamp = timestamp_ms.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        });
        let mut turn = DialogTurnData::new_with_kind(
            DialogTurnKind::LocalCommand,
            turn_id.clone(),
            turn_index,
            session_id.to_string(),
            None,
            UserMessageData {
                id: format!("{}-user", turn_id),
                content,
                timestamp,
                metadata: user_message_metadata,
            },
        );
        turn.timestamp = timestamp;
        turn.start_time = timestamp;
        turn.end_time = Some(timestamp);
        turn.duration_ms = Some(0);
        turn.status = TurnStatus::Completed;

        if self.config.enable_persistence && Self::should_persist_session(&session) {
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn)
                .await?;
        }

        let session_snapshot = if let Some(mut session) = self.sessions.get_mut(session_id) {
            if !session.dialog_turn_ids.iter().any(|existing| existing == &turn_id) {
                session.dialog_turn_ids.push(turn_id);
            }
            session.state = SessionState::Idle;
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();

            if self.config.enable_persistence && Self::should_persist_session(&session) {
                Some(session.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(session) = session_snapshot {
            self.persistence_manager.save_session(&workspace_path, &session).await?;
        }

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn_index, "local_command_turn_persisted")
            .await;

        Ok(turn)
    }

    /// Complete dialog turn
    pub(crate) async fn complete_dialog_turn(
        &self,
        session_id: &str,
        turn_id: &str,
        final_response: String,
        stats: TurnStats,
    ) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            debug!(
                "Skipping dialog turn persistence for transient session completion: session_id={}, turn_id={}, response_len={}, rounds={}",
                session_id,
                turn_id,
                final_response.len(),
                stats.total_rounds
            );
            return Ok(());
        }

        let workspace_path = self
            .effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;
        let turn_index = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.iter().position(|id| id == turn_id))
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;
        let mut turn = self
            .persistence_manager
            .load_dialog_turn(&workspace_path, session_id, turn_index)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;

        // Update state
        let completion_timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let has_assistant_text = turn
            .model_rounds
            .iter()
            .any(|round| round.text_items.iter().any(|item| !item.content.trim().is_empty()));
        if !has_assistant_text && !final_response.trim().is_empty() {
            let round_index = turn.model_rounds.len();
            turn.model_rounds.push(ModelRoundData {
                id: format!("{}-final-round", turn.turn_id),
                turn_id: turn.turn_id.clone(),
                round_index,
                timestamp: completion_timestamp,
                text_items: vec![TextItemData {
                    id: format!("{}-final-text", turn.turn_id),
                    content: final_response.clone(),
                    is_streaming: false,
                    timestamp: completion_timestamp,
                    is_markdown: true,
                    order_index: Some(0),
                    is_subagent_item: None,
                    parent_task_tool_id: None,
                    subagent_session_id: None,
                    status: Some("completed".to_string()),
                }],
                tool_items: Vec::new(),
                thinking_items: Vec::new(),
                start_time: completion_timestamp,
                end_time: Some(completion_timestamp),
                duration_ms: Some(0),
                provider_id: None,
                model_id: None,
                model_alias: None,
                first_chunk_ms: None,
                first_visible_output_ms: None,
                stream_duration_ms: None,
                attempt_count: None,
                failure_category: None,
                token_details: None,
                status: "completed".to_string(),
            });
        }
        turn.status = TurnStatus::Completed;
        turn.duration_ms = Some(stats.duration_ms);
        turn.end_time = Some(completion_timestamp);

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn.turn_index, "turn_completed")
            .await;

        // Persist
        if self.should_persist_session_id(session_id) {
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn)
                .await?;
        }

        debug!(
            "Dialog turn completed: turn_id={}, rounds={}, tools={}",
            turn_id, stats.total_rounds, stats.total_tools
        );

        Ok(())
    }

    /// Mark a dialog turn as failed and persist it.
    /// Unlike `complete_dialog_turn`, this sets the state to `Failed` with an error message.
    pub(crate) async fn fail_dialog_turn(&self, session_id: &str, turn_id: &str, error: String) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            debug!(
                "Skipping dialog turn persistence for transient session failure: session_id={}, turn_id={}, error={}",
                session_id, turn_id, error
            );
            return Ok(());
        }

        let workspace_path = self
            .effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;
        let turn_index = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.iter().position(|id| id == turn_id))
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;
        let mut turn = self
            .persistence_manager
            .load_dialog_turn(&workspace_path, session_id, turn_index)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;

        turn.status = TurnStatus::Error;
        turn.end_time = Some(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn.turn_index, "turn_failed")
            .await;
        if self.should_persist_session_id(session_id) {
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn)
                .await?;
        }

        debug!(
            "Dialog turn marked as failed: turn_id={}, turn_index={}, error={}",
            turn_id, turn.turn_index, error
        );

        Ok(())
    }

    /// Mark a dialog turn as cancelled and persist it. Unlike
    /// `complete_dialog_turn`, this writes `TurnStatus::Cancelled` so the
    /// frontend / persistence layer can distinguish a user-cancelled turn
    /// from a fully-completed one. Any partial assistant content that was
    /// already streamed is preserved in `model_rounds`.
    pub(crate) async fn cancel_dialog_turn(&self, session_id: &str, turn_id: &str) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            debug!(
                "Skipping dialog turn persistence for transient session cancellation: session_id={}, turn_id={}",
                session_id, turn_id
            );
            return Ok(());
        }

        let workspace_path = self
            .effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;
        let turn_index = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.iter().position(|id| id == turn_id))
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;
        let mut turn = self
            .persistence_manager
            .load_dialog_turn(&workspace_path, session_id, turn_index)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;

        turn.status = TurnStatus::Cancelled;
        turn.end_time = Some(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn.turn_index, "turn_cancelled")
            .await;

        self.persistence_manager
            .save_dialog_turn(&workspace_path, &turn)
            .await?;

        debug!(
            "Dialog turn marked as cancelled: turn_id={}, turn_index={}",
            turn_id, turn.turn_index
        );

        Ok(())
    }

    /// Complete a maintenance turn and persist its synthetic model round payload.
    pub(crate) async fn complete_maintenance_turn(
        &self,
        session_id: &str,
        turn_id: &str,
        model_rounds: Vec<ModelRoundData>,
        duration_ms: u64,
    ) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            debug!(
                "Skipping maintenance turn persistence for transient session completion: session_id={}, turn_id={}, rounds={}, duration_ms={}",
                session_id,
                turn_id,
                model_rounds.len(),
                duration_ms
            );
            return Ok(());
        }

        let workspace_path = self
            .effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;
        let turn_index = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.iter().position(|id| id == turn_id))
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;
        let mut turn = self
            .persistence_manager
            .load_dialog_turn(&workspace_path, session_id, turn_index)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;

        let completion_timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        turn.model_rounds = model_rounds;
        turn.status = TurnStatus::Completed;
        turn.duration_ms = Some(duration_ms);
        turn.end_time = Some(completion_timestamp);

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn.turn_index, "maintenance_turn_completed")
            .await;

        if self.should_persist_session_id(session_id) {
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn)
                .await?;
        }

        Ok(())
    }

    /// Mark a maintenance turn as failed while preserving its synthetic tool state.
    pub(crate) async fn fail_maintenance_turn(
        &self,
        session_id: &str,
        turn_id: &str,
        error: String,
        model_rounds: Vec<ModelRoundData>,
    ) -> NortHingResult<()> {
        if !self.should_persist_session_id(session_id) {
            debug!(
                "Skipping maintenance turn persistence for transient session failure: session_id={}, turn_id={}, rounds={}, error={}",
                session_id,
                turn_id,
                model_rounds.len(),
                error
            );
            return Ok(());
        }

        let workspace_path = self
            .effective_session_workspace_path(session_id)
            .await
            .ok_or_else(|| NortHingError::Validation(format!("Session workspace_path is missing: {}", session_id)))?;
        let turn_index = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.iter().position(|id| id == turn_id))
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;
        let mut turn = self
            .persistence_manager
            .load_dialog_turn(&workspace_path, session_id, turn_index)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Dialog turn not found: {}", turn_id)))?;

        let completion_timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        turn.model_rounds = model_rounds;
        turn.status = TurnStatus::Error;
        turn.duration_ms = Some(completion_timestamp.saturating_sub(turn.start_time));
        turn.end_time = Some(completion_timestamp);

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn.turn_index, "maintenance_turn_failed")
            .await;

        if self.should_persist_session_id(session_id) {
            self.persistence_manager
                .save_dialog_turn(&workspace_path, &turn)
                .await?;
        }

        debug!(
            "Maintenance turn marked as failed: turn_id={}, turn_index={}, error={}",
            turn_id, turn.turn_index, error
        );

        Ok(())
    }
}
