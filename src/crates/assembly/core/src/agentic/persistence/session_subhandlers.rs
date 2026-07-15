//! Session IO sub-handlers (Round 10a split)
//!
//! Save/load/delete/list/touch session files, including session state, and session metadata builders.
//!
//! This file owns the session io-related methods of `PersistenceManager`
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredSessionStateFile {
    schema_version: u32,
    config: SessionConfig,
    snapshot_session_id: Option<String>,
    // Derived runtime cache for reminder semantics. The source of truth lives
    // on persisted dialog turns via `DialogTurnData.agent_type`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_user_dialog_agent_type: Option<String>,
    // Session-level prompt-cache guard state. This records the most recent user
    // submission accepted by the scheduler and intentionally does not rewind on
    // history rollback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_submitted_agent_type: Option<String>,
    compression_state: CompressionState,
    runtime_state: SessionState,
}

impl PersistenceManager {
    async fn build_session_metadata(
        &self,
        workspace_path: &Path,
        session: &Session,
        existing: Option<&SessionMetadata>,
    ) -> SessionMetadata {
        let last_active_at = Self::system_time_to_unix_ms(session.last_activity_at);

        let resolved_identity = if let Some(workspace_root) = session.config.workspace_path.as_deref() {
            resolve_workspace_session_identity(
                workspace_root,
                session.config.remote_connection_id.as_deref(),
                session.config.remote_ssh_host.as_deref(),
            )
            .await
        } else {
            None
        };

        let workspace_root = resolved_identity
            .as_ref()
            .map(|identity| identity.logical_workspace_path().to_string())
            .or_else(|| session.config.workspace_path.clone())
            .or_else(|| existing.and_then(|value| value.workspace_path.clone()))
            .unwrap_or_else(|| workspace_path.to_string_lossy().to_string());
        let workspace_hostname = resolved_identity
            .as_ref()
            .map(|identity| identity.hostname.clone())
            .or_else(|| existing.and_then(|value| value.workspace_hostname.clone()))
            .or_else(|| {
                if session.config.remote_connection_id.is_some() {
                    session.config.remote_ssh_host.clone()
                } else {
                    Some(LOCAL_WORKSPACE_SSH_HOST.to_string())
                }
            });

        build_persisted_session_metadata(SessionMetadataBuildFacts {
            session_id: &session.session_id,
            session_name: &session.session_name,
            agent_type: &session.agent_type,
            last_user_dialog_agent_type: session.last_user_dialog_agent_type.as_deref(),
            last_submitted_agent_type: session.last_submitted_agent_type.as_deref(),
            created_by: session.created_by.as_deref(),
            session_kind: session.kind,
            model_name: session.config.model_id.as_deref(),
            created_at_ms: Self::system_time_to_unix_ms(session.created_at),
            last_active_at_ms: last_active_at,
            turn_count: session.dialog_turn_ids.len(),
            snapshot_session_id: session.snapshot_session_id.as_deref(),
            workspace_path: &workspace_root,
            workspace_hostname: workspace_hostname.as_deref(),
            existing,
        })
    }

    pub(super) async fn load_stored_session_state(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<StoredSessionStateFile>> {
        self.read_json_optional::<StoredSessionStateFile>(&self.state_path(workspace_path, session_id))
            .await
    }

    pub(super) async fn save_stored_session_state(
        &self,
        workspace_path: &Path,
        session_id: &str,
        state: &StoredSessionStateFile,
    ) -> NortHingResult<()> {
        self.write_json_atomic(&self.state_path(workspace_path, session_id), state)
            .await
    }

    pub async fn save_session(&self, workspace_path: &Path, session: &Session) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.ensure_session_dir(workspace_path, &session.session_id).await?;
        let existing_metadata = self.load_session_metadata(workspace_path, &session.session_id).await?;
        let metadata = self
            .build_session_metadata(workspace_path, session, existing_metadata.as_ref())
            .await;
        self.save_session_metadata(workspace_path, &metadata).await?;

        let state = StoredSessionStateFile {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            config: session.config.clone(),
            snapshot_session_id: session.snapshot_session_id.clone(),
            last_user_dialog_agent_type: session.last_user_dialog_agent_type.clone(),
            last_submitted_agent_type: session.last_submitted_agent_type.clone(),
            compression_state: session.compression_state.clone(),
            runtime_state: Self::sanitize_runtime_state(&session.state),
        };
        self.save_stored_session_state(workspace_path, &session.session_id, &state)
            .await
    }

    pub async fn load_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<Session> {
        let (session, _) = self.load_session_with_turns(workspace_path, session_id).await?;
        Ok(session)
    }

    pub(super) fn build_session_from_persisted_parts(
        metadata: SessionMetadata,
        stored_state: Option<StoredSessionStateFile>,
        turns: &[DialogTurnData],
    ) -> Session {
        let mut config = stored_state
            .as_ref()
            .map(|value| value.config.clone())
            .unwrap_or_default();
        if config.workspace_path.is_none() {
            config.workspace_path = metadata.workspace_path.clone();
        }
        if config.remote_ssh_host.is_none() {
            config.remote_ssh_host = metadata
                .workspace_hostname
                .clone()
                .filter(|host| host != LOCAL_WORKSPACE_SSH_HOST && host != "_unresolved");
        }
        if config.model_id.is_none() && !metadata.model_name.is_empty() {
            config.model_id = Some(metadata.model_name.clone());
        }

        let compression_state = stored_state
            .as_ref()
            .map(|value| value.compression_state.clone())
            .unwrap_or_default();
        let runtime_state = stored_state
            .as_ref()
            .map(|value| Self::sanitize_runtime_state(&value.runtime_state))
            .unwrap_or(SessionState::Idle);
        let created_at = Self::unix_ms_to_system_time(metadata.created_at);
        let last_activity_at = Self::unix_ms_to_system_time(metadata.last_active_at);
        let dialog_turn_ids = turns.iter().map(|turn| turn.turn_id.clone()).collect();

        Session {
            session_id: metadata.session_id.clone(),
            session_name: metadata.session_name.clone(),
            agent_type: metadata.agent_type.clone(),
            last_user_dialog_agent_type: stored_state
                .as_ref()
                .and_then(|value| value.last_user_dialog_agent_type.clone())
                .or_else(|| metadata.last_user_dialog_agent_type.clone()),
            last_submitted_agent_type: stored_state
                .as_ref()
                .and_then(|value| value.last_submitted_agent_type.clone())
                .or_else(|| metadata.last_submitted_agent_type.clone()),
            created_by: metadata.created_by.clone(),
            kind: metadata.session_kind,
            snapshot_session_id: stored_state
                .as_ref()
                .and_then(|value| value.snapshot_session_id.clone())
                .or(metadata.snapshot_session_id.clone()),
            dialog_turn_ids,
            state: runtime_state,
            config,
            compression_state,
            // Phase D.2 + I.4: project the persistence-layer
            // SessionRelationship into the lightweight
            // InMemoryRelationship (parent_session_id + parent_request_id
            // + parent_tool_call_id + parent_dialog_turn_id + parent_turn_index).
            // When this code path runs (the normal disk-load path), the
            // in-memory list_sessions branch will start surfacing the
            // parent link without an extra disk read. The closure
            // captures `r` so the projection can reach all fields
            // without re-borrowing `metadata`.
            relationship: metadata.relationship.as_ref().and_then(|r| {
                let parent_session_id = r.parent_session_id.clone();
                let parent_request_id = r.parent_request_id.clone();
                let parent_dialog_turn_id = r.parent_dialog_turn_id.clone();
                let parent_turn_index = r.parent_turn_index;
                let parent_tool_call_id = r.parent_tool_call_id.clone();
                if parent_session_id.is_none()
                    && parent_request_id.is_none()
                    && parent_dialog_turn_id.is_none()
                    && parent_tool_call_id.is_none()
                {
                    return None;
                }
                Some(InMemoryRelationship {
                    parent_session_id,
                    parent_request_id,
                    parent_dialog_turn_id,
                    parent_turn_index,
                    parent_tool_call_id,
                })
            }),
            created_at,
            updated_at: last_activity_at,
            last_activity_at,
        }
    }

    pub async fn save_session_state(
        &self,
        workspace_path: &Path,
        session_id: &str,
        state: &SessionState,
    ) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        let mut stored_state = self
            .load_stored_session_state(workspace_path, session_id)
            .await?
            .unwrap_or(StoredSessionStateFile {
                schema_version: SESSION_STORAGE_SCHEMA_VERSION,
                config: SessionConfig {
                    workspace_path: None,
                    ..Default::default()
                },
                snapshot_session_id: None,
                last_user_dialog_agent_type: None,
                last_submitted_agent_type: None,
                compression_state: CompressionState::default(),
                runtime_state: SessionState::Idle,
            });
        stored_state.schema_version = SESSION_STORAGE_SCHEMA_VERSION;
        stored_state.runtime_state = Self::sanitize_runtime_state(state);
        self.save_stored_session_state(workspace_path, session_id, &stored_state)
            .await
    }

    pub async fn delete_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        self.session_metadata_store(workspace_path)
            .delete_session_dir_and_index(session_id)
            .await
            .map_err(Self::session_metadata_store_error)?;
        info!("Session deleted: session_id={}", session_id);
        Ok(())
    }

    pub async fn list_sessions(&self, workspace_path: &Path) -> NortHingResult<Vec<SessionSummary>> {
        let metadata_list = self.list_session_metadata(workspace_path).await?;
        let mut summaries = Vec::with_capacity(metadata_list.len());

        for metadata in metadata_list {
            let state = self
                .load_stored_session_state(workspace_path, &metadata.session_id)
                .await?
                .map(|value| Self::sanitize_runtime_state(&value.runtime_state))
                .unwrap_or(SessionState::Idle);

            summaries.push(SessionSummary {
                session_id: metadata.session_id,
                session_name: metadata.session_name,
                agent_type: metadata.agent_type,
                last_user_dialog_agent_type: metadata.last_user_dialog_agent_type,
                last_submitted_agent_type: metadata.last_submitted_agent_type,
                created_by: metadata.created_by,
                kind: metadata.session_kind,
                turn_count: metadata.turn_count,
                created_at: Self::unix_ms_to_system_time(metadata.created_at),
                last_activity_at: Self::unix_ms_to_system_time(metadata.last_active_at),
                state,
                parent_session_id: metadata.relationship.as_ref().and_then(|r| r.parent_session_id.clone()),
            });
        }

        summaries.sort_by_key(|b| std::cmp::Reverse(b.last_activity_at));
        Ok(summaries)
    }

    pub async fn touch_session(&self, workspace_path: &Path, session_id: &str) -> NortHingResult<()> {
        if let Some(mut metadata) = self.load_session_metadata(workspace_path, session_id).await? {
            metadata.touch();
            self.save_session_metadata(workspace_path, &metadata).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PersistenceManager;
    use crate::infrastructure::PathManager;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use uuid::Uuid;

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("northhing-session-transcript-test-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("test workspace should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn path_manager(&self) -> Arc<PathManager> {
            Arc::new(PathManager::with_user_root_for_tests(self.path.join("user-root")))
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[tokio::test]
    async fn listing_sessions_does_not_create_sessions_dir_for_uninitialized_runtime() {
        let workspace = TestWorkspace::new();
        let manager = PersistenceManager::new(workspace.path_manager()).expect("persistence manager");

        let visible = manager
            .list_session_metadata(workspace.path())
            .await
            .expect("visible listing should succeed");
        let raw = manager
            .list_session_metadata_including_internal(workspace.path())
            .await
            .expect("raw listing should succeed");

        assert!(visible.is_empty());
        assert!(raw.is_empty());
        assert!(
            !manager.project_sessions_dir(workspace.path()).exists(),
            "listing sessions should not create the runtime sessions directory"
        );
    }
}
