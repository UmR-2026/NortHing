//! Session Manager
//!
//! Responsible for session CRUD, lifecycle management, and resource association

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

/// Session manager configuration
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    pub max_active_sessions: usize,
    pub session_idle_timeout: Duration,
    pub auto_save_interval: Duration,
    pub enable_persistence: bool,
    pub prompt_cache_policy: PromptCachePolicy,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600), // 1 hour
            auto_save_interval: Duration::from_secs(300),    // 5 minutes
            enable_persistence: true,
            prompt_cache_policy: PromptCachePolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTitleMethod {
    Ai,
    Fallback,
}

impl SessionTitleMethod {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSessionTitle {
    pub title: String,
    pub method: SessionTitleMethod,
}

// When a full skill/agent listing baseline is rebuilt at turn R, snapshots whose
// turn_index < R still contain now-redundant listing diff reminders. We do not
// eagerly rewrite all historical snapshots; instead restore/rollback sanitize those
// older snapshots lazily based on this persisted cutoff.
pub(super) const LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY: &str = "listingBaselineRebuildTurnIndex";

/// Session manager
pub struct SessionManager {
    /// Active sessions in memory
    pub(crate) sessions: Arc<DashMap<String, Session>>,

    /// Runtime cache of session_id -> effective workspace path.
    /// Populated on session create/restore and used to restore evicted sessions
    /// or resolve workspace-bound operations that only receive a session_id.
    /// This cache is intentionally retained across memory eviction, but should
    /// be cleared when a session is explicitly deleted.
    pub(crate) session_workspace_index: Arc<DashMap<String, PathBuf>>,

    /// Sub-components
    pub(crate) context_store: Arc<SessionContextStore>,
    pub(crate) prompt_cache_store: Arc<SessionPromptCacheStore>,
    pub(crate) turn_skill_agent_snapshot_store: Arc<TurnSkillAgentSnapshotStore>,
    pub(crate) skill_agent_baseline_override_snapshot_store: Arc<DashMap<String, TurnSkillAgentSnapshot>>,
    pub(crate) file_read_state_store: Arc<FileReadStateStore>,
    pub(crate) evidence_ledger: Arc<SessionEvidenceLedger>,
    pub(crate) persistence_manager: Arc<PersistenceManager>,

    /// Configuration
    pub(crate) config: SessionManagerConfig,
}

/// Snapshot used by the auto-save background task to compare in-memory session
/// state against the last persisted version. Promoted to `pub(super)` so the
/// `session_manager_auto_save_cleanup` sibling can construct and inspect these
/// snapshots while iterating the sessions map.
#[derive(Clone)]
pub(super) struct SessionAutoSaveSnapshot {
    pub(super) session_id: String,
    pub(super) updated_at: SystemTime,
    pub(super) last_activity_at: SystemTime,
    pub(super) session: Session,
}

/// Cleanup candidate record used by the cleanup background task. Promoted to
/// `pub(super)` so the `session_manager_auto_save_cleanup` sibling can build
/// candidates when scanning for expired sessions.
#[derive(Clone)]
pub(super) struct SessionCleanupCandidate {
    pub(super) session_id: String,
    pub(super) updated_at: SystemTime,
    pub(super) last_activity_at: SystemTime,
}
