//! Skill Agent Snapshots sub-handlers (Round 10a split)
//!
//! Save/load/delete turn context and skill agent baseline/snapshot files.
//!
//! This file owns the skill agent snapshots-related methods of `PersistenceManager`
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
pub(super) struct StoredTurnContextSnapshotFile {
    schema_version: u32,
    session_id: String,
    turn_index: usize,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredTurnSkillAgentSnapshotFile {
    schema_version: u32,
    session_id: String,
    turn_index: usize,
    snapshot: TurnSkillAgentSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredSkillAgentBaselineOverrideFile {
    schema_version: u32,
    session_id: String,
    snapshot: TurnSkillAgentSnapshot,
}

#[derive(Debug, Default)]
pub(super) struct ContextSnapshotPayloadStats {
    tool_result_count: usize,
    raw_result_string_chars: usize,
    result_for_assistant_chars: usize,
    largest_raw_result_chars: usize,
    largest_raw_result_path: String,
}

pub(super) fn collect_json_string_stats(
    value: &serde_json::Value,
    path: &str,
    total: &mut usize,
    largest: &mut (usize, String),
) {
    match value {
        serde_json::Value::String(text) => {
            let char_count = text.chars().count();
            *total += char_count;
            if char_count > largest.0 {
                *largest = (char_count, path.to_string());
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_json_string_stats(item, &format!("{}[{}]", path, index), total, largest);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                let next_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };
                collect_json_string_stats(item, &next_path, total, largest);
            }
        }
        _ => {}
    }
}

pub(super) fn context_snapshot_payload_stats(messages: &[Message]) -> ContextSnapshotPayloadStats {
    let mut stats = ContextSnapshotPayloadStats::default();
    for (message_index, message) in messages.iter().enumerate() {
        let MessageContent::ToolResult {
            tool_name,
            result,
            result_for_assistant,
            ..
        } = &message.content
        else {
            continue;
        };

        stats.tool_result_count += 1;
        if let Some(text) = result_for_assistant.as_deref() {
            stats.result_for_assistant_chars += text.chars().count();
        }

        let mut raw_chars = 0usize;
        let mut largest = (0usize, String::new());
        collect_json_string_stats(
            result,
            &format!("message[{}].{}", message_index, tool_name),
            &mut raw_chars,
            &mut largest,
        );
        stats.raw_result_string_chars += raw_chars;
        if largest.0 > stats.largest_raw_result_chars {
            stats.largest_raw_result_chars = largest.0;
            stats.largest_raw_result_path = largest.1;
        }
    }
    stats
}

impl PersistenceManager {
    pub async fn save_turn_context_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
        messages: &[Message],
    ) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.ensure_snapshots_dir(workspace_path, session_id).await?;

        let snapshot = StoredTurnContextSnapshotFile {
            schema_version: SESSION_STORAGE_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            turn_index,
            messages: Self::sanitize_messages_for_persistence(messages),
        };

        self.write_json_atomic(
            &self.context_snapshot_path(workspace_path, session_id, turn_index),
            &snapshot,
        )
        .await
    }

    pub async fn load_turn_context_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<Option<Vec<Message>>> {
        let snapshot = self
            .read_json_optional::<StoredTurnContextSnapshotFile>(&self.context_snapshot_path(
                workspace_path,
                session_id,
                turn_index,
            ))
            .await?;
        Ok(snapshot.map(|value| value.messages))
    }

    pub async fn load_latest_turn_context_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<(usize, Vec<Message>)>> {
        let started_at = Instant::now();
        let dir = self.snapshots_dir(workspace_path, session_id);
        if !dir.exists() {
            return Ok(None);
        }

        let scan_started_at = Instant::now();
        let mut latest: Option<usize> = None;
        let mut snapshot_file_count = 0usize;
        let mut rd = fs::read_dir(&dir)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to read snapshots directory: {}", e)))?;

        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| NortHingError::io(format!("Failed to iterate snapshots directory: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(index_str) = stem.strip_prefix("context-") else {
                continue;
            };
            if let Ok(index) = index_str.parse::<usize>() {
                snapshot_file_count += 1;
                latest = Some(latest.map(|value| value.max(index)).unwrap_or(index));
            }
        }
        let scan_duration = scan_started_at.elapsed();

        let Some(turn_index) = latest else {
            return Ok(None);
        };

        let load_started_at = Instant::now();
        let Some(messages) = self
            .load_turn_context_snapshot(workspace_path, session_id, turn_index)
            .await?
        else {
            return Ok(None);
        };
        let load_duration = load_started_at.elapsed();
        let total_duration = started_at.elapsed();

        if total_duration >= Duration::from_millis(80) || snapshot_file_count >= 10 {
            let payload_stats = context_snapshot_payload_stats(&messages);
            debug!(
                "Loaded latest context snapshot: session_id={} turn_index={} snapshot_file_count={} scan_duration_ms={} load_duration_ms={} total_duration_ms={} message_count={} tool_result_count={} raw_result_string_chars={} result_for_assistant_chars={} largest_raw_result_chars={} largest_raw_result_path={}",
                session_id,
                turn_index,
                snapshot_file_count,
                scan_duration.as_millis(),
                load_duration.as_millis(),
                total_duration.as_millis(),
                messages.len(),
                payload_stats.tool_result_count,
                payload_stats.raw_result_string_chars,
                payload_stats.result_for_assistant_chars,
                payload_stats.largest_raw_result_chars,
                payload_stats.largest_raw_result_path
            );
        }

        Ok(Some((turn_index, messages)))
    }

    pub async fn save_turn_skill_agent_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
        snapshot: &TurnSkillAgentSnapshot,
    ) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.ensure_snapshots_dir(workspace_path, session_id).await?;

        self.write_json_atomic(
            &self.skill_agent_snapshot_path(workspace_path, session_id, turn_index),
            &StoredTurnSkillAgentSnapshotFile {
                schema_version: SESSION_STORAGE_SCHEMA_VERSION,
                session_id: session_id.to_string(),
                turn_index,
                snapshot: snapshot.clone(),
            },
        )
        .await
    }

    pub async fn load_turn_skill_agent_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<Option<TurnSkillAgentSnapshot>> {
        let stored = self
            .read_json_optional::<StoredTurnSkillAgentSnapshotFile>(&self.skill_agent_snapshot_path(
                workspace_path,
                session_id,
                turn_index,
            ))
            .await?;
        Ok(stored.map(|value| value.snapshot))
    }

    pub async fn delete_turn_skill_agent_snapshots_from(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<()> {
        let dir = self.snapshots_dir(workspace_path, session_id);
        if !dir.exists() {
            return Ok(());
        }

        let mut rd = fs::read_dir(&dir)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to read snapshots directory: {}", e)))?;
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| NortHingError::io(format!("Failed to iterate snapshots directory: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(index_str) = stem.strip_prefix("skill-agent-") else {
                continue;
            };
            let Ok(index) = index_str.parse::<usize>() else {
                continue;
            };
            if index >= turn_index {
                let _ = fs::remove_file(&path).await;
            }
        }

        Ok(())
    }

    pub async fn save_skill_agent_baseline_override_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
        snapshot: &TurnSkillAgentSnapshot,
    ) -> NortHingResult<()> {
        self.ensure_runtime_for_write(workspace_path).await?;
        self.ensure_snapshots_dir(workspace_path, session_id).await?;

        self.write_json_atomic(
            &self.skill_agent_baseline_override_path(workspace_path, session_id),
            &StoredSkillAgentBaselineOverrideFile {
                schema_version: SESSION_STORAGE_SCHEMA_VERSION,
                session_id: session_id.to_string(),
                snapshot: snapshot.clone(),
            },
        )
        .await
    }

    pub async fn load_skill_agent_baseline_override_snapshot(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<TurnSkillAgentSnapshot>> {
        let stored = self
            .read_json_optional::<StoredSkillAgentBaselineOverrideFile>(
                &self.skill_agent_baseline_override_path(workspace_path, session_id),
            )
            .await?;
        Ok(stored.map(|value| value.snapshot))
    }

    pub async fn delete_turn_context_snapshots_from(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<()> {
        let dir = self.snapshots_dir(workspace_path, session_id);
        if !dir.exists() {
            return Ok(());
        }

        let mut rd = fs::read_dir(&dir)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to read snapshots directory: {}", e)))?;
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| NortHingError::io(format!("Failed to iterate snapshots directory: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let index_str = if let Some(index) = stem.strip_prefix("context-") {
                index
            } else if let Some(index) = stem.strip_prefix("skill-agent-") {
                index
            } else {
                continue;
            };
            let Ok(index) = index_str.parse::<usize>() else {
                continue;
            };
            if index >= turn_index {
                let _ = fs::remove_file(&path).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{context_snapshot_payload_stats, PersistenceManager};
    use crate::agentic::core::{Message, ToolResult};
    use crate::agentic::skill_agent_snapshot::{AgentSnapshotEntry, SkillSnapshotEntry, TurnSkillAgentSnapshot};
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

    #[test]
    fn context_snapshot_payload_stats_counts_tool_result_payloads_without_contents() {
        let messages = vec![
            Message::assistant("hello".to_string()),
            Message::tool_result(ToolResult {
                tool_id: "tool-1".to_string(),
                tool_name: "Bash".to_string(),
                result: serde_json::json!({ "output": "x".repeat(40) }),
                result_for_assistant: Some("assistant summary".to_string()),
                is_error: false,
                duration_ms: Some(1),
                image_attachments: None,
            }),
        ];

        let stats = context_snapshot_payload_stats(&messages);

        assert_eq!(stats.tool_result_count, 1);
        assert_eq!(stats.raw_result_string_chars, 40);
        assert_eq!(stats.result_for_assistant_chars, 17);
        assert_eq!(stats.largest_raw_result_chars, 40);
        assert_eq!(stats.largest_raw_result_path, "message[1].Bash.output");
        assert!(!stats.largest_raw_result_path.contains(&"x".repeat(40)));
    }

    #[tokio::test]
    async fn skill_agent_snapshots_persist_and_truncate_with_context_snapshots() {
        let workspace = TestWorkspace::new();
        let manager =
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager");
        let session_id = Uuid::new_v4().to_string();
        let snapshot = TurnSkillAgentSnapshot {
            skills: vec![SkillSnapshotEntry {
                name: "skill-a".to_string(),
                description: "desc-a".to_string(),
                location: "/skills/a".to_string(),
            }],
            subagents: vec![AgentSnapshotEntry {
                id: "agent-a".to_string(),
                description: "desc-a".to_string(),
                default_tools: vec!["Read".to_string()],
            }],
        };

        manager
            .save_turn_context_snapshot(workspace.path(), &session_id, 0, &[Message::user("hi".to_string())])
            .await
            .expect("context snapshot should save");
        manager
            .save_turn_skill_agent_snapshot(workspace.path(), &session_id, 0, &snapshot)
            .await
            .expect("skill-agent snapshot should save");

        let loaded = manager
            .load_turn_skill_agent_snapshot(workspace.path(), &session_id, 0)
            .await
            .expect("skill-agent snapshot should load")
            .expect("skill-agent snapshot should exist");
        assert_eq!(loaded, snapshot);

        manager
            .delete_turn_context_snapshots_from(workspace.path(), &session_id, 0)
            .await
            .expect("snapshot deletion should succeed");

        assert!(manager
            .load_turn_skill_agent_snapshot(workspace.path(), &session_id, 0)
            .await
            .expect("skill-agent snapshot reload should succeed")
            .is_none());
        assert!(manager
            .load_turn_context_snapshot(workspace.path(), &session_id, 0)
            .await
            .expect("context snapshot reload should succeed")
            .is_none());
    }
}
