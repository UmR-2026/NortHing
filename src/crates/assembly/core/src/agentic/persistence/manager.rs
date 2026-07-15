//! Persistence Manager facade
//!
//! Round 10a split: implementation is split across 6 sub-domain siblings
//! (session_subhandlers, turn_subhandlers, transcript_subhandlers,
//! metadata_subhandlers, skill_snapshot_subhandlers, paths_utilities).
//! This file keeps the struct definition and the 3 public constructors.
//! See `session_branch.rs` (Round 3b partial split) for the original
//! branch-session handler and the multi-impl pattern reference.

use crate::agentic::core::{
    strip_prompt_markup, CompressionState, InMemoryRelationship, Message, MessageContent, Session, SessionConfig,
    SessionState, SessionSummary,
};
use crate::agentic::session::{SessionPromptCache, PROMPT_CACHE_SCHEMA_VERSION};
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::infrastructure::PathManager;
use crate::service::remote_ssh::workspace_state::{resolve_workspace_session_identity, LOCAL_WORKSPACE_SSH_HOST};
use crate::service::session::{
    DialogTurnData, SessionMetadata, SessionTranscriptExport, SessionTranscriptExportOptions,
    SessionTranscriptIndexEntry, ToolItemData, TranscriptLineRange, SESSION_STORAGE_SCHEMA_VERSION,
};
use crate::service::workspace_runtime::WorkspaceRuntimeService;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use futures::{stream, StreamExt};
use northhing_runtime_ports::{SessionTurnLoadRequest, SessionTurnLoadTiming};
pub use northhing_services_core::session::SessionMetadataPage;
use northhing_services_core::{
    json_store::{JsonFileStore, JsonFileStoreError},
    session::{
        build_session_metadata as build_persisted_session_metadata, empty_session_metadata_page,
        refresh_session_metadata_from_turns, try_refresh_session_metadata_for_saved_turn, SessionMetadataBuildFacts,
        SessionMetadataStore, SessionMetadataStoreError, SessionStorageLayout,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

pub struct PersistenceManager {
    pub(super) path_manager: Arc<PathManager>,
    pub(super) runtime_service: Arc<WorkspaceRuntimeService>,
}

impl PersistenceManager {
    pub fn new(path_manager: Arc<PathManager>) -> NortHingResult<Self> {
        Ok(Self {
            runtime_service: Arc::new(WorkspaceRuntimeService::new(path_manager.clone())),
            path_manager,
        })
    }

    pub fn path_manager(&self) -> &Arc<PathManager> {
        &self.path_manager
    }

    pub fn runtime_service(&self) -> &Arc<WorkspaceRuntimeService> {
        &self.runtime_service
    }
}
