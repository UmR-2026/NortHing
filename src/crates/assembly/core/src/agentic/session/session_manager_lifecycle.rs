//! Round 9 split sibling: session_manager_lifecycle
//!
//! Auto-extracted from session_manager.rs (11 methods).
//! Methods declared `pub(crate)` so external callers and other modules can use them.

use super::session_manager::SessionManager;
use super::session_manager::SessionManagerConfig;

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
    pub(crate) fn new(
        context_store: Arc<SessionContextStore>,
        persistence_manager: Arc<PersistenceManager>,
        config: SessionManagerConfig,
    ) -> Self {
        let enable_persistence = config.enable_persistence;

        let manager = Self {
            sessions: Arc::new(DashMap::new()),
            session_workspace_index: Arc::new(DashMap::new()),
            context_store,
            prompt_cache_store: Arc::new(SessionPromptCacheStore::new()),
            turn_skill_agent_snapshot_store: Arc::new(TurnSkillAgentSnapshotStore::new()),
            skill_agent_baseline_override_snapshot_store: Arc::new(DashMap::new()),
            file_read_state_store: Arc::new(FileReadStateStore::new()),
            evidence_ledger: Arc::new(SessionEvidenceLedger::new()),
            persistence_manager,
            config,
        };

        // Start background tasks
        if enable_persistence {
            manager.spawn_auto_save_task();
        }
        manager.spawn_cleanup_task();
        manager.spawn_model_reconciliation_listener();

        manager
    }

    pub(crate) async fn create_session(
        &self,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
    ) -> NortHingResult<Session> {
        self.create_session_with_id_and_details(None, session_name, agent_type, config, None, SessionKind::Standard)
            .await
    }

    pub(crate) async fn create_session_with_id(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
    ) -> NortHingResult<Session> {
        self.create_session_with_id_and_details(
            session_id,
            session_name,
            agent_type,
            config,
            None,
            SessionKind::Standard,
        )
        .await
    }

    pub(crate) async fn create_session_with_id_and_creator(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
        created_by: Option<String>,
    ) -> NortHingResult<Session> {
        self.create_session_with_id_and_details(
            session_id,
            session_name,
            agent_type,
            config,
            created_by,
            SessionKind::Standard,
        )
        .await
    }

    pub(crate) async fn create_session_with_id_and_details(
        &self,
        session_id: Option<String>,
        session_name: String,
        agent_type: String,
        config: SessionConfig,
        created_by: Option<String>,
        kind: SessionKind,
    ) -> NortHingResult<Session> {
        let _workspace_path = Self::session_workspace_from_config(&config)
            .ok_or_else(|| NortHingError::Validation("Session workspace_path is required".to_string()))?;

        let session_storage_path = Self::effective_workspace_path_from_config(&config)
            .await
            .ok_or_else(|| NortHingError::Validation("Session workspace_path is required".to_string()))?;

        // Check session count limit
        if self.sessions.len() >= self.config.max_active_sessions {
            return Err(NortHingError::Validation(format!(
                "Exceeded maximum session limit: {}",
                self.config.max_active_sessions
            )));
        }

        let mut session = if let Some(id) = session_id {
            Session::new_with_id(id, session_name, agent_type.clone(), config)
        } else {
            Session::new(session_name, agent_type.clone(), config)
        };
        session.created_by = created_by;
        session.kind = kind;
        let session_id = session.session_id.clone();

        // 1. Add to memory
        self.sessions.insert(session_id.clone(), session.clone());
        self.session_workspace_index
            .insert(session_id.clone(), session_storage_path.clone());

        // 2. Initialize the in-memory context cache.
        self.context_store.create_session(&session_id);
        self.turn_skill_agent_snapshot_store.create_session(&session_id);
        self.file_read_state_store.create_session(&session_id);

        // 3. Persist to local path (handles remote workspaces correctly)
        // Use the local `session` directly -- no need to re-fetch from DashMap,
        // which would hold a Ref guard across the async save_session call.
        if self.config.enable_persistence && Self::should_persist_session(&session) {
            self.persistence_manager
                .save_session(&session_storage_path, &session)
                .await?;
        }

        info!("Session created: session_name={}", session.session_name);

        Ok(session)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub(crate) async fn update_session_state(&self, session_id: &str, new_state: SessionState) -> NortHingResult<()> {
        let effective_path = self.effective_session_workspace_path(session_id).await;

        // IMPORTANT: keep the DashMap guard scope short -- do NOT hold it across .await.
        // Collect the data needed for persistence, then release the guard before doing I/O.
        let should_persist = if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.state = new_state.clone();
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();

            self.config.enable_persistence && Self::should_persist_session(&session)
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        };
        // RefMut guard released here -- DashMap shard lock is free.

        // Persist state changes outside the guard scope.
        if should_persist {
            if let Some(ref workspace_path) = effective_path {
                self.persistence_manager
                    .save_session_state(workspace_path, session_id, &new_state)
                    .await?;
            }
        }

        debug!(
            "Updated session state: session_id={}, state={:?}",
            session_id, new_state
        );

        Ok(())
    }

    pub(crate) async fn update_session_state_for_turn_if_processing(
        &self,
        session_id: &str,
        expected_turn_id: &str,
        new_state: SessionState,
    ) -> NortHingResult<bool> {
        let effective_path = self.effective_session_workspace_path(session_id).await;

        let should_persist = if let Some(mut session) = self.sessions.get_mut(session_id) {
            let owns_processing_turn = matches!(
                &session.state,
                SessionState::Processing {
                    current_turn_id,
                    ..
                } if current_turn_id == expected_turn_id
            );

            if !owns_processing_turn {
                debug!(
                    "Skipped session state update for stale turn: session_id={}, expected_turn_id={}, current_state={:?}",
                    session_id, expected_turn_id, session.state
                );
                return Ok(false);
            }

            session.state = new_state.clone();
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();

            self.config.enable_persistence && Self::should_persist_session(&session)
        } else {
            return Err(NortHingError::NotFound(format!("Session not found: {}", session_id)));
        };

        if should_persist {
            if let Some(ref workspace_path) = effective_path {
                self.persistence_manager
                    .save_session_state(workspace_path, session_id, &new_state)
                    .await?;
            }
        }

        debug!(
            "Updated session state for turn: session_id={}, turn_id={}, state={:?}",
            session_id, expected_turn_id, new_state
        );

        Ok(true)
    }

    pub(crate) fn touch_session(&self, session_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.last_activity_at = SystemTime::now();
        }
    }

    pub(crate) async fn delete_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        let delete_started_at = Instant::now();
        debug!(
            "Session deletion started: session_id={}, workspace_path={}, persistence_enabled={}",
            session_id,
            workspace_path.display(),
            self.config.enable_persistence
        );

        // 1. Clean up snapshot system resources (including physical snapshot files)
        let snapshot_stage_started_at = Instant::now();
        debug!(
            "Session deletion stage starting: session_id={}, stage=snapshot_cleanup",
            session_id
        );
        if let Ok(snapshot_manager) = ensure_snapshot_manager_for_workspace(workspace_path) {
            let snapshot_service = snapshot_manager.snapshot_service();
            let snapshot_service = snapshot_service.read().await;
            if let Err(e) = snapshot_service.accept_session(session_id).await {
                warn!("Failed to cleanup snapshot system resources: {}", e);
            } else {
                debug!("Snapshot system resources cleaned up: session_id={}", session_id);
            }
        }
        debug!(
            "Session deletion stage completed: session_id={}, stage=snapshot_cleanup, duration_ms={}",
            session_id,
            elapsed_ms_u64(snapshot_stage_started_at)
        );

        let context_stage_started_at = Instant::now();
        debug!(
            "Session deletion stage starting: session_id={}, stage=context_store_delete",
            session_id
        );
        self.context_store.delete_session(session_id);
        self.prompt_cache_store.delete_session(session_id);
        self.turn_skill_agent_snapshot_store.delete_session(session_id);
        self.skill_agent_baseline_override_snapshot_store.remove(session_id);
        self.file_read_state_store.delete_session(session_id);
        debug!(
            "Session deletion stage completed: session_id={}, stage=context_store_delete, duration_ms={}",
            session_id,
            elapsed_ms_u64(context_stage_started_at)
        );

        // 2. Delete persisted data
        if self.config.enable_persistence {
            let persistence_stage_started_at = Instant::now();
            debug!(
                "Session deletion stage starting: session_id={}, stage=persistence_delete",
                session_id
            );
            self.persistence_manager
                .delete_session(workspace_path, session_id)
                .await?;
            debug!(
                "Session deletion stage completed: session_id={}, stage=persistence_delete, duration_ms={}",
                session_id,
                elapsed_ms_u64(persistence_stage_started_at)
            );
        }

        if let Some(cron) = crate::service::cron::global_cron_service() {
            let cron_stage_started_at = Instant::now();
            debug!(
                "Session deletion stage starting: session_id={}, stage=cron_cleanup",
                session_id
            );
            match cron.delete_jobs_for_session(session_id).await {
                Ok(removed) if removed > 0 => {
                    info!(
                        "Removed {} scheduled job(s) for deleted session_id={}",
                        removed, session_id
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "Failed to remove scheduled jobs for deleted session_id={}: {}",
                        session_id, e
                    );
                }
            }
            debug!(
                "Session deletion stage completed: session_id={}, stage=cron_cleanup, duration_ms={}",
                session_id,
                elapsed_ms_u64(cron_stage_started_at)
            );
        }

        // 3. Clean up associated Terminal session
        use crate::service::terminal::TerminalApi;
        if let Ok(terminal_api) = TerminalApi::from_singleton() {
            let binding = terminal_api.session_manager().binding();
            let terminal_stage_started_at = Instant::now();
            debug!(
                "Session deletion stage starting: session_id={}, stage=terminal_binding_cleanup, has_binding={}",
                session_id,
                binding.has(session_id)
            );
            if binding.has(session_id) {
                if let Err(e) = binding.remove(session_id).await {
                    warn!("Failed to cleanup associated Terminal session: {}", e);
                } else {
                    debug!("Associated Terminal session cleaned up: session_id={}", session_id);
                }
            }
            debug!(
                "Session deletion stage completed: session_id={}, stage=terminal_binding_cleanup, duration_ms={}",
                session_id,
                elapsed_ms_u64(terminal_stage_started_at)
            );
        }

        // 4. Remove from memory
        let memory_stage_started_at = Instant::now();
        debug!(
            "Session deletion stage starting: session_id={}, stage=in_memory_remove",
            session_id
        );
        self.sessions.remove(session_id);
        debug!(
            "Session deletion stage completed: session_id={}, stage=in_memory_remove, duration_ms={}",
            session_id,
            elapsed_ms_u64(memory_stage_started_at)
        );
        self.session_workspace_index.remove(session_id);

        info!(
            "Session deletion completed: session_id={}, workspace_path={}, duration_ms={}",
            session_id,
            workspace_path.display(),
            elapsed_ms_u64(delete_started_at)
        );

        Ok(())
    }

    pub(crate) async fn list_sessions(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>> {
        if self.config.enable_persistence {
            self.persistence_manager.list_sessions(workspace_path).await
        } else {
            let summaries: Vec<_> = self
                .sessions
                .iter()
                .map(|entry| {
                    let session = entry.value();
                    SessionSummary {
                        session_id: session.session_id.clone(),
                        session_name: session.session_name.clone(),
                        agent_type: session.agent_type.clone(),
                        last_user_dialog_agent_type: session.last_user_dialog_agent_type.clone(),
                        last_submitted_agent_type: session.last_submitted_agent_type.clone(),
                        created_by: session.created_by.clone(),
                        kind: session.kind,
                        turn_count: session.dialog_turn_ids.len(),
                        created_at: session.created_at,
                        last_activity_at: session.last_activity_at,
                        state: session.state.clone(),
                        // Phase D.2: project `parent_session_id` from
                        // the in-memory `Session::relationship` field.
                        // Today this is always `None` because no caller
                        // populates the in-memory relationship — the
                        // disk-backed persistence path remains the source
                        // of truth. The field is in place so the moment a
                        // caller writes `session.relationship = Some(...)`
                        // (or a future disk-load step populates it), the
                        // tree view starts surfacing the hierarchy.
                        parent_session_id: session.relationship.as_ref().and_then(|r| r.parent_session_id.clone()),
                    }
                })
                .filter(|summary| !matches!(summary.kind, SessionKind::Subagent | SessionKind::EphemeralChild))
                .collect();
            Ok(summaries)
        }
    }
}
