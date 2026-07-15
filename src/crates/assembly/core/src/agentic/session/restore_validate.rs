//! R49b split sibling: rollback_context_to_turn_start
//!
//! Contains the rollback function that truncates a session's context and
//! persisted state to before a specified turn.

use super::session_manager::SessionManager;

use crate::agentic::core::SessionState;
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use std::time::SystemTime;
use tracing::warn;

impl SessionManager {
    /// Rollback "model context" to before the start of specified turn (i.e., keep 0..target_turn-1)
    pub(crate) async fn rollback_context_to_turn_start(
        &self,
        workspace_path: &Path,
        session_id: &str,
        target_turn: usize,
    ) -> NortHingResult<()> {
        // Ensure session is in memory (restore from persistence if necessary)
        if !self.sessions.contains_key(session_id) && self.config.enable_persistence {
            let _ = self.restore_session(workspace_path, session_id).await;
        }

        // Rollback may load a historical snapshot from before the latest rebuilt baseline. In
        // that case we must strip all listing diff reminders before the snapshot re-enters
        // runtime context, otherwise old diffs reappear after rollback/reopen.
        let listing_baseline_rebuild_turn_index = if self.config.enable_persistence {
            let metadata = self
                .persistence_manager
                .load_session_metadata(workspace_path, session_id)
                .await?;
            Self::listing_baseline_rebuild_turn_index_from_metadata(metadata.as_ref())
        } else {
            None
        };

        // 1) Load target context (target_turn == 0 => empty context)
        let messages = if target_turn == 0 {
            Vec::new()
        } else {
            let messages = self
                .persistence_manager
                .load_turn_context_snapshot(workspace_path, session_id, target_turn - 1)
                .await?
                .ok_or_else(|| {
                    NortHingError::NotFound(format!(
                        "turn context snapshot not found: session_id={} turn={}",
                        session_id,
                        target_turn - 1
                    ))
                })?;
            self.sanitize_listing_diff_context_snapshot_if_needed(
                workspace_path,
                session_id,
                target_turn - 1,
                messages,
                listing_baseline_rebuild_turn_index,
                "rollback_restore_pre_listing_baseline_rebuild_snapshot",
            )
            .await
        };

        // 2) Restore the in-memory context cache.
        self.context_store.replace_context(session_id, messages);

        let last_user_dialog_agent_type = if target_turn == 0 {
            None
        } else {
            let surviving_turns = self
                .persistence_manager
                .load_session_turns(workspace_path, session_id)
                .await?;
            let kept_turns = surviving_turns.into_iter().take(target_turn).collect::<Vec<_>>();
            let fallback_agent_type = self.sessions.get(session_id).map(|session| session.agent_type.clone());
            Self::derive_last_user_dialog_agent_type_from_turns(&kept_turns, fallback_agent_type.as_deref())
        };

        // 3) Truncate session turn list & persist
        // IMPORTANT: keep the DashMap guard scope short -- do NOT hold it across .await.
        let session_snapshot = if let Some(mut session) = self.sessions.get_mut(session_id) {
            if session.dialog_turn_ids.len() > target_turn {
                session.dialog_turn_ids.truncate(target_turn);
            }
            session.last_user_dialog_agent_type = last_user_dialog_agent_type;
            session.state = SessionState::Idle;
            session.updated_at = SystemTime::now();
            session.last_activity_at = SystemTime::now();

            let should_persist = Self::should_persist_session(&session) && self.config.enable_persistence;
            if should_persist {
                Some(session.clone())
            } else {
                None
            }
        } else {
            None
        };
        // RefMut guard released here -- DashMap shard lock is free.

        if let Some(session) = session_snapshot {
            self.persistence_manager.save_session(workspace_path, &session).await?;
        }

        // 4) Delete persisted turns and snapshots from target_turn (inclusive) onwards.
        // Runtime restore rebuilds history from persisted turn files, so removing only
        // context snapshots would make rolled-back prompts reappear after reload.
        if self.config.enable_persistence {
            self.persistence_manager
                .delete_dialog_turns_from(workspace_path, session_id, target_turn)
                .await?;
            self.persistence_manager
                .delete_turn_context_snapshots_from(workspace_path, session_id, target_turn)
                .await?;
            self.truncate_listing_baseline_rebuild_turn_index_after_rollback(workspace_path, session_id, target_turn)
                .await?;
        }
        self.turn_skill_agent_snapshot_store
            .remove_from(session_id, target_turn);

        Ok(())
    }
}
