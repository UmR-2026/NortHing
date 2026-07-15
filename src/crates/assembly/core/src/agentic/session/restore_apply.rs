//! R49b split sibling: full session restore family
//!
//! Contains restore_session*, restore_session_with_turns* thin wrappers,
//! plus the monolithic restore_session_with_turns_internal that loads
//! session+turns from persistence, rebuilds runtime context, and inserts
//! the session into the in-memory coordinator state.

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
use northhing_runtime_ports::{SessionStoragePathRequest, SessionStorePort};
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
    /// Restore session (from persistent storage)
    pub(crate) async fn restore_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<Session> {
        self.restore_session_internal(workspace_path, session_id, false).await
    }

    pub(crate) async fn restore_internal_session(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Session> {
        self.restore_session_internal(workspace_path, session_id, true).await
    }

    pub(crate) async fn restore_session_internal(
        &self,
        workspace_path: &Path,
        session_id: &str,
        include_internal: bool,
    ) -> NortHingResult<Session> {
        let (session, _) = self
            .restore_session_with_turns_internal(workspace_path, session_id, include_internal)
            .await?;
        Ok(session)
    }

    /// Restore session and return the persisted turns read during restore.
    pub(crate) async fn restore_session_with_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_session_with_turns_internal(workspace_path, session_id, false)
            .await
    }

    pub(crate) async fn restore_internal_session_with_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.restore_session_with_turns_internal(workspace_path, session_id, true)
            .await
    }

    pub(crate) async fn restore_session_with_turns_internal(
        &self,
        workspace_path: &Path,
        session_id: &str,
        include_internal: bool,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        let restore_started_at = Instant::now();
        // Check if session is already in memory
        let session_already_in_memory = self.sessions.contains_key(session_id);

        let storage_path_started_at = Instant::now();
        let session_storage_path = {
            let ws = workspace_path.to_string_lossy().to_string();
            let tmp_config = SessionConfig {
                workspace_path: Some(ws),
                ..Default::default()
            };
            Self::effective_workspace_path_from_config(&tmp_config)
                .await
                .unwrap_or_else(|| workspace_path.to_path_buf())
        };
        debug!(
            "Session restore phase completed: session_id={}, phase=resolve_storage_path, duration_ms={}",
            session_id,
            elapsed_ms_u64(storage_path_started_at)
        );

        let metadata_started_at = Instant::now();
        let session_metadata = self
            .persistence_manager
            .load_session_metadata(&session_storage_path, session_id)
            .await?;
        if session_metadata
            .as_ref()
            .is_some_and(|metadata| !include_internal && metadata.should_hide_from_user_lists())
        {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        }
        let listing_baseline_rebuild_turn_index =
            Self::listing_baseline_rebuild_turn_index_from_metadata(session_metadata.as_ref());
        debug!(
            "Session restore phase completed: session_id={}, phase=load_metadata, duration_ms={}",
            session_id,
            elapsed_ms_u64(metadata_started_at)
        );

        // 1. Load session and turns from storage in one pass
        let session_started_at = Instant::now();
        let (mut session, persisted_turns) = self
            .persistence_manager
            .load_session_with_turns(&session_storage_path, session_id)
            .await?;
        debug!(
            "Session restore phase completed: session_id={}, phase=load_session_with_turns, turn_count={}, duration_ms={}",
            session_id,
            persisted_turns.len(),
            elapsed_ms_u64(session_started_at)
        );

        let ai_config_for_restore = Self::load_ai_config_for_model_resolution().await;
        let mut should_persist_restored_session = false;

        // Lazy migration: if the persisted model_id is no longer usable
        // (model deleted or disabled while the session was on disk), repoint
        // it to "auto" before the session re-enters memory. The next request
        // will pick a model via the normal auto/agent/default pipeline.
        if let Some(persisted_model_id) = session.config.model_id.as_deref() {
            let trimmed = persisted_model_id.trim();
            let needs_migration = if trimmed.is_empty() {
                false
            } else if let Some(ai_config) = ai_config_for_restore.as_ref() {
                !Self::is_session_model_id_usable(ai_config, trimmed)
            } else {
                false
            };

            if needs_migration {
                warn!(
                    "Session restore detected stale model_id; migrating to auto: session_id={}, previous_model_id={}",
                    session_id, trimmed
                );
                let previous_model_id = trimmed.to_string();
                session.config.model_id = Some("auto".to_string());
                should_persist_restored_session = true;

                if let Some(coordinator) = crate::agentic::coordination::global_coordinator() {
                    coordinator
                        .emit_session_model_auto_migrated(
                            session_id,
                            &previous_model_id,
                            "auto",
                            "model_unavailable_on_restore",
                        )
                        .await;
                }
            }
        }

        if let Some(ai_config) = ai_config_for_restore.as_ref() {
            let previous_max_context_tokens = session.config.max_context_tokens;
            if let Some(context_window) = Self::sync_session_context_window_from_ai_config(&mut session, ai_config) {
                if context_window != previous_max_context_tokens {
                    should_persist_restored_session = true;
                    debug!(
                        "Session context window refreshed during restore: session_id={}, previous={}, resolved={}",
                        session_id, previous_max_context_tokens, context_window
                    );
                }
            }
        }

        // Reset session state to Idle
        // After application restart, previous Processing state is invalid and must be reset
        let previous_state_was_not_idle = !matches!(session.state, SessionState::Idle);
        if previous_state_was_not_idle {
            let old_state = session.state.clone();
            session.state = SessionState::Idle;
            debug!(
                "Resetting session state during restore: session_id={}, state={:?} -> Idle",
                session_id, old_state
            );
        }

        // 2. Restore runtime context with snapshot-first semantics.
        // If the latest snapshot lags behind turn persistence, append the missing turn delta
        // instead of truncating session history.
        //
        // This compensates for the fact that persistence is not transactional across
        // `session.json`, `turns/*.json`, and `snapshots/context-*.json`.
        let persisted_turn_ids: Vec<String> = persisted_turns.iter().map(|turn| turn.turn_id.clone()).collect();
        session.last_user_dialog_agent_type =
            Self::derive_last_user_dialog_agent_type_from_turns(&persisted_turns, Some(session.agent_type.as_str()));
        let mut latest_turn_index: Option<usize> = None;
        let context_snapshot_started_at = Instant::now();
        let mut messages = match self
            .persistence_manager
            .load_latest_turn_context_snapshot(&session_storage_path, session_id)
            .await?
        {
            Some((turn_index, msgs)) => {
                latest_turn_index = Some(turn_index);
                self.sanitize_listing_diff_context_snapshot_if_needed(
                    &session_storage_path,
                    session_id,
                    turn_index,
                    msgs,
                    listing_baseline_rebuild_turn_index,
                    "restore_pre_listing_baseline_rebuild_snapshot",
                )
                .await
            }
            None => Self::build_messages_from_turns(&persisted_turns),
        };
        debug!(
            "Session restore phase completed: session_id={}, phase=load_context_snapshot, snapshot_turn_index={:?}, message_count={}, duration_ms={}",
            session_id,
            latest_turn_index,
            messages.len(),
            elapsed_ms_u64(context_snapshot_started_at)
        );

        if let Some(snapshot_turn_index) = latest_turn_index {
            let delta_start = snapshot_turn_index.saturating_add(1);
            if delta_start < persisted_turns.len() {
                warn!(
                    "Context snapshot is behind persisted turns, rebuilding delta: session_id={}, snapshot_turn_index={}, persisted_turn_count={}",
                    session_id,
                    snapshot_turn_index,
                    persisted_turns.len()
                );
                messages.extend(Self::build_messages_from_turns(&persisted_turns[delta_start..]));
            }
        };

        if messages.is_empty() {
            debug!(
                "Session {} has empty persisted messages (may be new session)",
                session_id
            );
        }

        // 3. Restore the in-memory context cache from the recovered messages.
        // If session already exists, delete old one first then create (ensure clean state)
        if session_already_in_memory {
            self.context_store.delete_session(session_id);
            self.prompt_cache_store.delete_session(session_id);
            self.turn_skill_agent_snapshot_store.delete_session(session_id);
            self.skill_agent_baseline_override_snapshot_store.remove(session_id);
            self.file_read_state_store.delete_session(session_id);
        }

        let context_replace_started_at = Instant::now();
        self.context_store.replace_context(session_id, messages.clone());
        debug!(
            "Session restore phase completed: session_id={}, phase=replace_context, message_count={}, duration_ms={}",
            session_id,
            messages.len(),
            elapsed_ms_u64(context_replace_started_at)
        );

        let recoverable_turn_count = latest_turn_index
            .map(|turn_index| turn_index + 1)
            .unwrap_or(0)
            .max(persisted_turns.len());

        if session.dialog_turn_ids.len() < persisted_turns.len() {
            warn!(
                "Session metadata is behind persisted turns, rebuilding dialog_turn_ids: session_id={}, session_turn_count={}, persisted_turn_count={}",
                session_id,
                session.dialog_turn_ids.len(),
                persisted_turns.len()
            );
            session.dialog_turn_ids = persisted_turn_ids;
        } else if session.dialog_turn_ids.len() > recoverable_turn_count {
            warn!(
                "Session metadata exceeds recoverable history, truncating: session_id={}, session_turn_count={}, recoverable_turn_count={}",
                session_id,
                session.dialog_turn_ids.len(),
                recoverable_turn_count
            );
            session.dialog_turn_ids.truncate(recoverable_turn_count);
        } else if persisted_turns.len() == session.dialog_turn_ids.len()
            && session.dialog_turn_ids != persisted_turn_ids
        {
            warn!(
                "Session metadata turn ids diverge from persisted turns, normalizing order: session_id={}",
                session_id
            );
            session.dialog_turn_ids = persisted_turn_ids;
        }

        if recoverable_turn_count == 0 && !session.dialog_turn_ids.is_empty() && messages.is_empty() {
            warn!(
                "Session has no available context snapshot and messages are empty, clearing turns: session_id={}",
                session_id
            );
            session.dialog_turn_ids.clear();
        }

        let context_msg_count = self.context_store.get_context_messages(session_id).len();

        debug!(
            "Session restored: session_id={}, session_name={}, messages={}, context_messages={}, turn_count={}, total_duration_ms={}",
            session_id,
            session.session_name,
            messages.len(),
            context_msg_count,
            persisted_turns.len(),
            elapsed_ms_u64(restore_started_at)
        );

        // Do not infer unread completion from persisted runtime state during restore.
        // Older IDE versions could leave sessions in non-idle states on disk; treating those
        // as completed would surface misleading unread indicators after an upgrade.
        // Unread completion is now written only by runtime completion/persist paths.

        if should_persist_restored_session && self.should_persist_session_id(session_id) {
            self.persistence_manager
                .save_session(&session_storage_path, &session)
                .await?;
        }

        // 4. Add to memory (will overwrite if already exists)
        self.sessions.insert(session_id.to_string(), session.clone());
        self.session_workspace_index
            .insert(session_id.to_string(), session_storage_path.clone());

        Ok((session, persisted_turns))
    }
}
