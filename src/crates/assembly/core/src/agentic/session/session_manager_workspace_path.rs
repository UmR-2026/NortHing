//! Round 9 split sibling: session_manager_workspace_path
//!
//! Auto-extracted from session_manager.rs (4 methods).
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
    pub(crate) async fn effective_workspace_path_from_config(config: &SessionConfig) -> Option<PathBuf> {
        CoreSessionStorePort::resolve_storage_path_for_config(config)
            .await
            .map(|resolution| resolution.effective_storage_path().clone())
    }

    pub(crate) fn session_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        self.sessions
            .get(session_id)
            .and_then(|session| Self::session_workspace_from_config(&session.config))
    }

    pub(crate) async fn effective_session_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        let config = self.sessions.get(session_id)?.config.clone();
        Self::effective_workspace_path_from_config(&config).await
    }

    pub(crate) async fn resolve_session_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        if let Some(workspace_path) = self
            .get_session(session_id)
            .and_then(|session| session.config.workspace_path)
            .filter(|path| !path.is_empty())
        {
            return Some(PathBuf::from(workspace_path));
        }

        let indexed_workspace_path = self.session_workspace_index.get(session_id).map(|entry| entry.clone());
        if let Some(workspace_path) = indexed_workspace_path {
            match self
                .persistence_manager
                .load_session_metadata(&workspace_path, session_id)
                .await
            {
                Ok(Some(metadata)) => {
                    if let Some(bound_workspace) = metadata.workspace_path.filter(|path| !path.is_empty()) {
                        return Some(PathBuf::from(bound_workspace));
                    }
                    return Some(workspace_path);
                }
                Ok(None) => {}
                Err(err) => {
                    debug!(
                        "Failed to load indexed session metadata while resolving workspace: session_id={} workspace={} error={}",
                        session_id,
                        workspace_path.display(),
                        err
                    );
                }
            }
        }

        let workspace_service = global_workspace_service()?;
        let mut workspaces = workspace_service.list_workspace_infos().await;
        workspaces.sort_by(|left, right| right.last_accessed.cmp(&left.last_accessed));
        let candidates: Vec<PathBuf> = workspaces.into_iter().map(|workspace| workspace.root_path).collect();

        for workspace_path in candidates {
            let workspace_path = workspace_path.clone();
            match self
                .persistence_manager
                .load_session_metadata(&workspace_path, session_id)
                .await
            {
                Ok(Some(metadata)) => {
                    if let Some(bound_workspace) = metadata.workspace_path.filter(|path| !path.is_empty()) {
                        return Some(PathBuf::from(bound_workspace));
                    }
                    return Some(workspace_path);
                }
                Ok(None) => {}
                Err(err) => {
                    debug!(
                        "Failed to load session metadata while resolving workspace: session_id={} workspace={} error={}",
                        session_id,
                        workspace_path.display(),
                        err
                    );
                }
            }
        }

        None
    }
}
