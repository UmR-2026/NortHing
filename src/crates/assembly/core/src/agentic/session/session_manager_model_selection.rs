//! Round 9 split sibling: session_manager_model_selection
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
    pub(crate) async fn load_ai_config_for_model_resolution() -> Option<crate::service::config::types::AIConfig> {
        let config_service = get_global_config_service().await.ok()?;
        config_service.config(Some("ai")).await.ok()
    }

    pub(crate) fn is_auto_model_selector(model_id: &str) -> bool {
        let trimmed = model_id.trim();
        trimmed.is_empty() || trimmed == "auto" || trimmed == "default"
    }

    pub(crate) fn context_window_for_model_selection(
        ai_config: &crate::service::config::types::AIConfig,
        model_id: &str,
    ) -> Option<usize> {
        let trimmed = model_id.trim();
        if Self::is_auto_model_selector(trimmed) {
            return None;
        }

        let resolved_model_id = ai_config.resolve_model_selection(trimmed)?;
        ai_config
            .models
            .iter()
            .find(|model| model.id == resolved_model_id)
            .and_then(|model| model.context_window)
            .map(|tokens| tokens as usize)
    }

    pub(crate) fn session_context_window_from_ai_config(
        session: &Session,
        ai_config: &crate::service::config::types::AIConfig,
    ) -> Option<usize> {
        let configured_model_id = session
            .config
            .model_id
            .as_deref()
            .map(str::trim)
            .filter(|model_id| !model_id.is_empty())
            .unwrap_or("auto");

        if !Self::is_auto_model_selector(configured_model_id) {
            return Self::context_window_for_model_selection(ai_config, configured_model_id);
        }

        let agent_model_id = ai_config
            .agent_models
            .get(&session.agent_type)
            .map(String::as_str)
            .map(str::trim)
            .filter(|model_id| !Self::is_auto_model_selector(model_id));

        agent_model_id
            .and_then(|model_id| Self::context_window_for_model_selection(ai_config, model_id))
            .or_else(|| Self::context_window_for_model_selection(ai_config, "primary"))
    }

    pub(crate) fn sync_session_context_window_from_ai_config(
        session: &mut Session,
        ai_config: &crate::service::config::types::AIConfig,
    ) -> Option<usize> {
        let context_window = Self::session_context_window_from_ai_config(session, ai_config)?;
        session.config.max_context_tokens = context_window;
        Some(context_window)
    }
}
