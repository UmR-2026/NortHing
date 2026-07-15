//! Path Helpers & Message Sanitization sub-handlers (Round 10a split)
//!
//! Path resolution, ensure_dirs, JSON read/write helpers, prompt cache, message sanitization, and turn-status labels.
//!
//! This file owns the path helpers & message sanitization-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

use super::manager::PersistenceManager;
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

static SESSION_METADATA_UPDATE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredSessionPromptCacheFile {
    schema_version: u32,
    #[serde(flatten)]
    cache: SessionPromptCache,
}

impl PersistenceManager {
    pub(super) fn project_sessions_dir(&self, workspace_path: &Path) -> PathBuf {
        let remote_mirror_root = PathManager::remote_ssh_mirror_root();
        if workspace_path.starts_with(&remote_mirror_root) {
            // Already resolved: either the mirror runtime root, the mirror sessions dir,
            // or a session sub-dir. Treat the path as the sessions root directly.
            // (Inputs that already include a trailing `sessions` segment stay correct;
            // inputs at the mirror runtime root would historically fall back to the
            // legacy slug, but no current call-site uses that shape.)
            return workspace_path.to_path_buf();
        }
        self.path_manager.project_sessions_dir(workspace_path)
    }

    pub(super) fn metadata_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).metadata_path(session_id)
    }

    pub(super) fn state_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).state_path(session_id)
    }

    pub(super) fn prompt_cache_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).prompt_cache_path(session_id)
    }

    pub(super) fn turns_dir(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).turns_dir(session_id)
    }

    pub(super) fn snapshots_dir(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).snapshots_dir(session_id)
    }

    pub(super) fn turn_path(&self, workspace_path: &Path, session_id: &str, turn_index: usize) -> PathBuf {
        self.session_layout(workspace_path).turn_path(session_id, turn_index)
    }

    pub(super) fn context_snapshot_path(&self, workspace_path: &Path, session_id: &str, turn_index: usize) -> PathBuf {
        self.session_layout(workspace_path)
            .context_snapshot_path(session_id, turn_index)
    }

    pub(super) fn skill_agent_snapshot_path(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> PathBuf {
        self.session_layout(workspace_path)
            .skill_agent_snapshot_path(session_id, turn_index)
    }

    pub(super) fn skill_agent_baseline_override_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path)
            .skill_agent_baseline_override_path(session_id)
    }

    pub(super) fn transcript_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).transcript_path(session_id)
    }

    pub(super) fn transcript_meta_path(&self, workspace_path: &Path, session_id: &str) -> PathBuf {
        self.session_layout(workspace_path).transcript_meta_path(session_id)
    }

    pub(super) fn index_path(&self, workspace_path: &Path) -> PathBuf {
        self.session_layout(workspace_path).index_path()
    }

    pub(super) fn session_layout(&self, workspace_path: &Path) -> SessionStorageLayout {
        SessionStorageLayout::new(self.project_sessions_dir(workspace_path))
    }

    pub(super) fn existing_project_sessions_dir(&self, workspace_path: &Path) -> Option<PathBuf> {
        let dir = self.project_sessions_dir(workspace_path);
        dir.exists().then_some(dir)
    }

    pub(super) async fn ensure_runtime_for_write(&self, workspace_path: &Path) -> NortHingResult<()> {
        let remote_mirror_root = PathManager::remote_ssh_mirror_root();
        if workspace_path.starts_with(&remote_mirror_root) {
            return Ok(());
        }

        self.runtime_service
            .ensure_local_workspace_runtime(workspace_path)
            .await
            .map(|_| ())
    }

    pub(super) async fn ensure_session_dir(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<PathBuf> {
        self.session_layout(workspace_path)
            .ensure_session_dir(session_id)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to create session directory: {}", e)))
    }

    pub(super) async fn ensure_turns_dir(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<PathBuf> {
        self.session_layout(workspace_path)
            .ensure_turns_dir(session_id)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to create turns directory: {}", e)))
    }

    pub(super) async fn ensure_snapshots_dir(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<PathBuf> {
        self.session_layout(workspace_path)
            .ensure_snapshots_dir(session_id)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to create snapshots directory: {}", e)))
    }

    pub(super) async fn ensure_artifacts_dir(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<PathBuf> {
        self.session_layout(workspace_path)
            .ensure_artifacts_dir(session_id)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to create artifacts directory: {}", e)))
    }

    pub(super) async fn read_json_optional<T: DeserializeOwned>(&self, path: &Path) -> NortHingResult<Option<T>> {
        JsonFileStore.read_optional(path).await.map_err(Self::json_store_error)
    }

    pub(super) async fn write_json_atomic<T: Serialize>(&self, path: &Path, value: &T) -> NortHingResult<()> {
        JsonFileStore
            .write_atomic(path, value)
            .await
            .map_err(Self::json_store_error)
    }

    pub(super) async fn get_session_metadata_update_lock(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> Arc<Mutex<()>> {
        let metadata_path = self.metadata_path(workspace_path, session_id);
        let registry = SESSION_METADATA_UPDATE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut registry_guard = registry.lock().await;
        registry_guard
            .entry(metadata_path)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub(super) fn json_store_error(error: JsonFileStoreError) -> NortHingError {
        if error.is_deserialization() {
            NortHingError::Deserialization(error.to_string())
        } else if error.is_serialization() {
            NortHingError::serialization(error.to_string())
        } else {
            NortHingError::io(error.to_string())
        }
    }

    pub(super) fn session_metadata_store_error(error: SessionMetadataStoreError) -> NortHingError {
        if error.is_deserialization() {
            NortHingError::Deserialization(error.to_string())
        } else if error.is_serialization() {
            NortHingError::serialization(error.to_string())
        } else {
            NortHingError::io(error.to_string())
        }
    }

    pub(super) fn system_time_to_unix_ms(time: SystemTime) -> u64 {
        time.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    pub(super) fn unix_ms_to_system_time(timestamp_ms: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(timestamp_ms)
    }

    pub(super) fn sanitize_messages_for_persistence(messages: &[Message]) -> Vec<Message> {
        messages.iter().map(Self::sanitize_message_for_persistence).collect()
    }

    pub(super) fn sanitize_message_for_persistence(message: &Message) -> Message {
        let mut sanitized = message.clone();

        match &mut sanitized.content {
            MessageContent::Multimodal { images, .. } => {
                for image in images.iter_mut() {
                    if image.data_url.as_ref().is_some_and(|v| !v.is_empty()) {
                        image.data_url = None;

                        let mut metadata = image.metadata.take().unwrap_or_else(|| serde_json::json!({}));
                        if !metadata.is_object() {
                            metadata = serde_json::json!({ "raw_metadata": metadata });
                        }
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("has_data_url".to_string(), serde_json::json!(true));
                        }
                        image.metadata = Some(metadata);
                    }
                }
            }
            MessageContent::ToolResult {
                result,
                image_attachments,
                ..
            } => {
                Self::redact_data_url_in_json(result);
                if image_attachments.is_some() {
                    *image_attachments = None;
                }
            }
            _ => {}
        }

        sanitized
    }

    pub(super) fn redact_data_url_in_json(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                let had_data_url = map.remove("data_url").is_some();
                if had_data_url {
                    map.insert("has_data_url".to_string(), serde_json::json!(true));
                }
                for child in map.values_mut() {
                    Self::redact_data_url_in_json(child);
                }
            }
            serde_json::Value::Array(arr) => {
                for child in arr {
                    Self::redact_data_url_in_json(child);
                }
            }
            _ => {}
        }
    }

    pub(super) fn sanitize_runtime_state(state: &SessionState) -> SessionState {
        match state {
            SessionState::Processing { .. } => SessionState::Idle,
            other => other.clone(),
        }
    }

    pub(super) fn turn_status_label(status: &crate::service::session::TurnStatus) -> &'static str {
        match status {
            crate::service::session::TurnStatus::InProgress => "inprogress",
            crate::service::session::TurnStatus::Completed => "completed",
            crate::service::session::TurnStatus::Error => "error",
            crate::service::session::TurnStatus::Cancelled => "cancelled",
        }
    }

    pub async fn load_prompt_cache(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<SessionPromptCache>> {
        Ok(self
            .read_json_optional::<StoredSessionPromptCacheFile>(&self.prompt_cache_path(workspace_path, session_id))
            .await?
            .map(|file| file.cache))
    }

    pub async fn save_prompt_cache(
        &self,
        workspace_path: &Path,
        session_id: &str,
        cache: &SessionPromptCache,
    ) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.ensure_session_dir(workspace_path, session_id).await?;

        self.write_json_atomic(
            &self.prompt_cache_path(workspace_path, session_id),
            &StoredSessionPromptCacheFile {
                schema_version: PROMPT_CACHE_SCHEMA_VERSION,
                cache: cache.clone(),
            },
        )
        .await
    }

    pub async fn delete_prompt_cache(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        match fs::remove_file(self.prompt_cache_path(workspace_path, session_id)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(NortHingError::io(format!(
                "Failed to delete prompt cache for session {}: {}",
                session_id, error
            ))),
        }
    }
}
