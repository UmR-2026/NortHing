//! Round 9 split sibling: session_manager_auto_save_cleanup
//!
//! Auto-extracted from session_manager.rs (9 methods).
//! Methods declared `pub(crate)` so external callers and other modules can use them.

use super::session_manager::SessionManager;
use super::session_manager::{SessionAutoSaveSnapshot, SessionCleanupCandidate};

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
    pub(super) fn collect_auto_save_snapshots(sessions: &DashMap<String, Session>) -> Vec<SessionAutoSaveSnapshot> {
        sessions
            .iter()
            .filter_map(|entry| {
                let session = entry.value();
                if !Self::should_persist_session(session) {
                    return None;
                }
                Some(SessionAutoSaveSnapshot {
                    session_id: session.session_id.clone(),
                    updated_at: session.updated_at,
                    last_activity_at: session.last_activity_at,
                    session: session.clone(),
                })
            })
            .collect()
    }

    pub(super) fn auto_save_snapshot_is_current(
        sessions: &DashMap<String, Session>,
        snapshot: &SessionAutoSaveSnapshot,
    ) -> bool {
        sessions
            .get(&snapshot.session_id)
            .map(|session| Self::same_session_version(&session, snapshot.updated_at, snapshot.last_activity_at))
            .unwrap_or(false)
    }

    pub(crate) fn auto_save_interval(interval: Duration) -> time::Interval {
        time::interval_at(time::Instant::now() + interval, interval)
    }

    pub(crate) fn is_session_expired(session: &Session, now: SystemTime, timeout: Duration) -> bool {
        now.duration_since(session.last_activity_at)
            .map(|idle_duration| idle_duration > timeout)
            .unwrap_or(false)
    }

    pub(super) fn collect_expired_session_candidates(
        sessions: &DashMap<String, Session>,
        now: SystemTime,
        timeout: Duration,
    ) -> Vec<SessionCleanupCandidate> {
        sessions
            .iter()
            .filter_map(|entry| {
                let session = entry.value();
                if !Self::is_session_expired(session, now, timeout) {
                    return None;
                }
                Some(SessionCleanupCandidate {
                    session_id: session.session_id.clone(),
                    updated_at: session.updated_at,
                    last_activity_at: session.last_activity_at,
                })
            })
            .collect()
    }

    pub(super) fn cleanup_candidate_matches_session(
        session: &Session,
        candidate: &SessionCleanupCandidate,
        now: SystemTime,
        timeout: Duration,
    ) -> bool {
        Self::same_session_version(session, candidate.updated_at, candidate.last_activity_at)
            && Self::is_session_expired(session, now, timeout)
    }

    pub(super) fn cleanup_snapshot_for_candidate(
        sessions: &DashMap<String, Session>,
        candidate: &SessionCleanupCandidate,
        now: SystemTime,
        timeout: Duration,
    ) -> Option<Session> {
        sessions.get(&candidate.session_id).and_then(|session| {
            Self::cleanup_candidate_matches_session(&session, candidate, now, timeout).then(|| session.clone())
        })
    }

    pub(crate) fn spawn_auto_save_task(&self) {
        let sessions = self.sessions.clone();
        let persistence = self.persistence_manager.clone();
        let interval = self.config.auto_save_interval;

        tokio::spawn(async move {
            let mut ticker = Self::auto_save_interval(interval);

            loop {
                ticker.tick().await;

                for snapshot in Self::collect_auto_save_snapshots(&sessions) {
                    if !Self::auto_save_snapshot_is_current(&sessions, &snapshot) {
                        continue;
                    }
                    if let Some(workspace_path) =
                        Self::effective_workspace_path_from_config(&snapshot.session.config).await
                    {
                        if !Self::auto_save_snapshot_is_current(&sessions, &snapshot) {
                            continue;
                        }
                        if let Err(e) = persistence.save_session(&workspace_path, &snapshot.session).await {
                            error!(
                                "Failed to auto-save session: session_id={}, error={}",
                                snapshot.session_id, e
                            );
                        }
                    }
                }
            }
        });

        debug!("Auto-save task started");
    }

    pub(crate) fn spawn_cleanup_task(&self) {
        let sessions = self.sessions.clone();
        let timeout = self.config.session_idle_timeout;
        let persistence = self.persistence_manager.clone();
        let enable_persistence = self.config.enable_persistence;
        let context_store = self.context_store.clone();
        let prompt_cache_store = self.prompt_cache_store.clone();
        let turn_skill_agent_snapshot_store = self.turn_skill_agent_snapshot_store.clone();
        let skill_agent_baseline_override_snapshot_store = self.skill_agent_baseline_override_snapshot_store.clone();
        let file_read_state_store = self.file_read_state_store.clone();

        tokio::spawn(async move {
            let mut ticker = time::interval(Duration::from_secs(60));

            loop {
                ticker.tick().await;

                let now = SystemTime::now();
                let candidates = Self::collect_expired_session_candidates(&sessions, now, timeout);

                for candidate in candidates {
                    debug!("Cleaning up expired session: session_id={}", candidate.session_id);

                    let cleanup_now = SystemTime::now();
                    let Some(session) =
                        Self::cleanup_snapshot_for_candidate(&sessions, &candidate, cleanup_now, timeout)
                    else {
                        continue;
                    };

                    if enable_persistence && Self::should_persist_session(&session) {
                        if let Some(workspace_path) = Self::effective_workspace_path_from_config(&session.config).await
                        {
                            if Self::cleanup_snapshot_for_candidate(&sessions, &candidate, SystemTime::now(), timeout)
                                .is_some()
                            {
                                if let Err(e) = persistence.save_session(&workspace_path, &session).await {
                                    warn!(
                                        target: "session::auto_save",
                                        "background auto-save failed: session_id={}, workspace_path={}, error={}",
                                        session.session_id,
                                        workspace_path.display(),
                                        e,
                                    );
                                }
                            }
                        }
                    }

                    let removal_now = SystemTime::now();
                    if sessions
                        .remove_if(&candidate.session_id, |_, session| {
                            Self::cleanup_candidate_matches_session(session, &candidate, removal_now, timeout)
                        })
                        .is_some()
                    {
                        context_store.delete_session(&candidate.session_id);
                        prompt_cache_store.delete_session(&candidate.session_id);
                        turn_skill_agent_snapshot_store.delete_session(&candidate.session_id);
                        skill_agent_baseline_override_snapshot_store.remove(&candidate.session_id);
                        file_read_state_store.delete_session(&candidate.session_id);
                    }
                }
            }
        });

        debug!("Cleanup task started");
    }
}
