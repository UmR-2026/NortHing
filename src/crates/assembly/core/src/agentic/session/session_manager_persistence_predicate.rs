//! Round 9 split sibling: session_manager_persistence_predicate
//!
//! Auto-extracted from session_manager.rs (5 methods).
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
    pub(crate) fn session_workspace_from_config(config: &SessionConfig) -> Option<PathBuf> {
        config.workspace_path.as_ref().map(PathBuf::from)
    }

    pub(crate) fn should_persist_session_kind(kind: SessionKind) -> bool {
        match kind {
            SessionKind::Standard | SessionKind::Subagent => true,
            SessionKind::EphemeralChild => false,
        }
    }

    pub(crate) fn should_persist_session(session: &Session) -> bool {
        Self::should_persist_session_kind(session.kind)
    }

    pub(crate) fn same_session_version(
        session: &Session,
        updated_at: SystemTime,
        last_activity_at: SystemTime,
    ) -> bool {
        session.updated_at == updated_at && session.last_activity_at == last_activity_at
    }

    pub(crate) fn should_persist_session_id(&self, session_id: &str) -> bool {
        self.config.enable_persistence
            && self
                .sessions
                .get(session_id)
                .map(|session| Self::should_persist_session(&session))
                .unwrap_or(true)
    }
}
