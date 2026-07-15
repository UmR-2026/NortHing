use super::session_manager::SessionManager;
use super::LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY;
use crate::agentic::core::{InternalReminderKind, Message};
use crate::service::session::SessionMetadata;
use crate::util::errors::NortHingResult;
use serde_json::json;
use std::path::Path;
use tracing::warn;

impl SessionManager {
    pub(crate) async fn seed_forked_skill_agent_listing_baselines(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
    ) {
        // Forked children need two different baselines at the same time:
        // - the parent's turn-0 snapshot stays as the prompt/listing baseline so the child's
        //   first request can reuse the same full skill/agent listing prefix
        // - the parent's latest snapshot becomes the child's own turn-0 snapshot so later child
        //   turns diff against the fork-time surface instead of diffing forever against the
        //   parent's original turn-0 baseline
        let prompt_listing_baseline = self.turn_skill_agent_snapshot(parent_session_id, 0).await;
        if let Some(snapshot) = prompt_listing_baseline.clone() {
            self.remember_skill_agent_baseline_override_snapshot(child_session_id, snapshot)
                .await;
        }

        let latest_parent_snapshot = match self.get_turn_count(parent_session_id).checked_sub(1) {
            Some(turn_index) => self
                .latest_turn_skill_agent_snapshot_at_or_before(parent_session_id, turn_index)
                .await
                .map(|(_, snapshot)| snapshot),
            None => None,
        };

        if let Some(snapshot) = latest_parent_snapshot.or(prompt_listing_baseline) {
            self.remember_turn_skill_agent_snapshot(child_session_id, 0, snapshot)
                .await;
        }
    }
    pub(crate) async fn rebuild_skill_agent_listing_baseline_to_latest(&self, session_id: &str) -> bool {
        let Some(turn_index) = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.len().checked_sub(1))
        else {
            return false;
        };

        let Some((_, latest_snapshot)) = self
            .latest_turn_skill_agent_snapshot_at_or_before(session_id, turn_index)
            .await
        else {
            return false;
        };

        if self.skill_agent_baseline_override_snapshot(session_id).await.is_some() {
            self.remember_skill_agent_baseline_override_snapshot(session_id, latest_snapshot.clone())
                .await;
        }

        self.recover_first_turn_skill_agent_snapshot(session_id, latest_snapshot)
            .await;
        self.persist_listing_baseline_rebuild_turn_index_best_effort(session_id, turn_index)
            .await;

        let _ = self.remove_listing_diff_internal_reminders(session_id).await;
        true
    }

    pub(crate) async fn remove_listing_diff_internal_reminders(&self, session_id: &str) -> bool {
        let context_messages = self.context_store.get_context_messages(session_id);
        if context_messages.is_empty() {
            return false;
        }

        let (filtered_messages, changed) = Self::strip_listing_diff_internal_reminders(context_messages);
        if !changed {
            return false;
        }

        self.context_store.replace_context(session_id, filtered_messages);
        self.persist_current_turn_context_snapshot_best_effort(session_id, "listing_diff_internal_reminders_removed")
            .await;
        true
    }

    pub(crate) fn strip_listing_diff_internal_reminders(messages: Vec<Message>) -> (Vec<Message>, bool) {
        let original_len = messages.len();
        let filtered_messages = messages
            .into_iter()
            .filter(|message| {
                !message
                    .internal_reminder_kind()
                    .is_some_and(InternalReminderKind::is_listing_diff)
            })
            .collect::<Vec<_>>();

        let changed = filtered_messages.len() != original_len;
        (filtered_messages, changed)
    }

    pub(crate) fn listing_baseline_rebuild_turn_index_from_custom_metadata(
        custom_metadata: Option<&serde_json::Value>,
    ) -> Option<usize> {
        custom_metadata?
            .get(LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY)?
            .as_u64()?
            .try_into()
            .ok()
    }

    pub(crate) fn listing_baseline_rebuild_turn_index_from_metadata(
        metadata: Option<&SessionMetadata>,
    ) -> Option<usize> {
        Self::listing_baseline_rebuild_turn_index_from_custom_metadata(
            metadata.and_then(|metadata| metadata.custom_metadata.as_ref()),
        )
    }

    pub(crate) async fn persist_context_snapshot_messages_best_effort(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
        messages: &[Message],
        reason: &str,
    ) {
        if !self.should_persist_session_id(session_id) {
            return;
        }

        if let Err(err) = self
            .persistence_manager
            .save_turn_context_snapshot(workspace_path, session_id, turn_index, messages)
            .await
        {
            warn!(
                "failed to persist explicit context snapshot: session_id={}, turn_index={}, reason={}, err={}",
                session_id, turn_index, reason, err
            );
        }
    }

    pub(crate) async fn persist_listing_baseline_rebuild_turn_index_best_effort(
        &self,
        session_id: &str,
        turn_index: usize,
    ) {
        if let Err(err) = self
            .merge_session_custom_metadata(
                session_id,
                json!({
                    LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY: turn_index,
                }),
            )
            .await
        {
            warn!(
                "failed to persist listing baseline rebuild turn index: session_id={}, turn_index={}, err={}",
                session_id, turn_index, err
            );
        }
    }

    pub(crate) async fn truncate_listing_baseline_rebuild_turn_index_after_rollback(
        &self,
        workspace_path: &Path,
        session_id: &str,
        target_turn: usize,
    ) -> NortHingResult<()> {
        let metadata = self
            .persistence_manager
            .load_session_metadata(workspace_path, session_id)
            .await?;
        let Some(existing_cutoff) = Self::listing_baseline_rebuild_turn_index_from_metadata(metadata.as_ref()) else {
            return Ok(());
        };

        if existing_cutoff <= target_turn {
            return Ok(());
        }

        // After rollback, the session branches again from `target_turn`. Keeping a cutoff newer
        // than that branch point would cause future snapshots on the new branch to be mistaken
        // for "pre-rebuild" history during the next restore, so clamp the cutoff down.
        self.merge_session_custom_metadata(
            session_id,
            json!({
                LISTING_BASELINE_REBUILD_TURN_INDEX_METADATA_KEY: target_turn,
            }),
        )
        .await
    }
}
