use crate::agentic::core::{Message, MessageSemanticKind, SessionState};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::session::SessionManager;
use crate::agentic::session::SessionPromptCache;
use crate::service::session::{DialogTurnData, UserMessageData};
use crate::util::errors::NortHingResult;
use std::path::Path;
use std::time::SystemTime;
use tracing::{debug, warn};

impl SessionManager {
    pub(crate) fn build_messages_from_turns(turns: &[DialogTurnData]) -> Vec<Message> {
        let mut messages = Vec::new();

        for turn in turns {
            if !turn.kind.is_model_visible() {
                continue;
            }

            let user_message = if let Some(metadata) = &turn.user_message.metadata {
                let images = metadata
                    .get("images")
                    .and_then(|value| value.as_array())
                    .map(|values| {
                        values
                            .iter()
                            .map(|value| ImageContextData {
                                id: value.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                image_path: value.get("image_path").and_then(|v| v.as_str()).map(str::to_string),
                                data_url: value.get("data_url").and_then(|v| v.as_str()).map(str::to_string),
                                mime_type: value
                                    .get("mime_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("image/png")
                                    .to_string(),
                                metadata: Some(value.clone()),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if images.is_empty() {
                    Message::user(turn.user_message.content.clone())
                } else {
                    Message::user_multimodal(turn.user_message.content.clone(), images)
                }
            } else {
                Message::user(turn.user_message.content.clone())
            };
            messages.push(
                user_message
                    .with_turn_id(turn.turn_id.clone())
                    .with_semantic_kind(MessageSemanticKind::ActualUserInput),
            );

            let assistant_text = turn
                .model_rounds
                .iter()
                .flat_map(|round| round.text_items.iter())
                .map(|item| item.content.clone())
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            let assistant_thinking = turn
                .model_rounds
                .iter()
                .flat_map(|round| round.thinking_items.iter())
                .map(|item| item.content.clone())
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");

            let has_text = !assistant_text.trim().is_empty();
            let has_thinking = !assistant_thinking.trim().is_empty();

            if has_text || has_thinking {
                let reasoning_content = if has_thinking { Some(assistant_thinking) } else { None };
                messages.push(
                    Message::assistant_with_reasoning(reasoning_content, assistant_text, Vec::new())
                        .with_turn_id(turn.turn_id.clone()),
                );
            }
        }

        messages
    }

    pub(crate) async fn persist_context_snapshot_for_turn_best_effort(
        &self,
        session_id: &str,
        turn_index: usize,
        reason: &str,
    ) {
        if !self.should_persist_session_id(session_id) {
            return;
        }

        let Some(workspace_path) = self.effective_session_workspace_path(session_id).await else {
            debug!(
                "Skipping context snapshot persistence because workspace path is unavailable: session_id={}, turn_index={}, reason={}",
                session_id, turn_index, reason
            );
            return;
        };

        let context_messages = self.context_store.get_context_messages(session_id);
        if let Err(err) = self
            .persistence_manager
            .save_turn_context_snapshot(&workspace_path, session_id, turn_index, &context_messages)
            .await
        {
            warn!(
                "failed to persist context snapshot: session_id={}, turn_index={}, reason={}, err={}",
                session_id, turn_index, reason, err
            );
        }
    }

    pub(crate) async fn persist_current_turn_context_snapshot_best_effort(&self, session_id: &str, reason: &str) {
        let Some(turn_index) = self
            .sessions
            .get(session_id)
            .and_then(|session| session.dialog_turn_ids.len().checked_sub(1))
        else {
            debug!(
                "Skipping current-turn context snapshot because no turn is active: session_id={}, reason={}",
                session_id, reason
            );
            return;
        };

        self.persist_context_snapshot_for_turn_best_effort(session_id, turn_index, reason)
            .await;
    }

    pub(crate) async fn persist_prompt_cache_best_effort(&self, session_id: &str, reason: &str) {
        if !self.should_persist_session_id(session_id) {
            return;
        }

        let Some(workspace_path) = self.effective_session_workspace_path(session_id).await else {
            debug!(
                "Skipping prompt cache persistence because workspace path is unavailable: session_id={}, reason={}",
                session_id, reason
            );
            return;
        };

        let cache = self.prompt_cache_store.get_cache(session_id).unwrap_or_default();

        let persist_result = if cache.system_prompt.is_none() && cache.user_context.is_none() {
            self.persistence_manager
                .delete_prompt_cache(&workspace_path, session_id)
                .await
        } else {
            self.persistence_manager
                .save_prompt_cache(&workspace_path, session_id, &cache)
                .await
        };

        if let Err(error) = persist_result {
            warn!(
                "Failed to persist prompt cache: session_id={}, workspace_path={}, reason={}, error={}",
                session_id,
                workspace_path.display(),
                reason,
                error
            );
        }
    }

    pub(crate) async fn sanitize_listing_diff_context_snapshot_if_needed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
        messages: Vec<Message>,
        cutoff_turn_index: Option<usize>,
        reason: &str,
    ) -> Vec<Message> {
        let Some(cutoff_turn_index) = cutoff_turn_index else {
            return messages;
        };
        // The rebuild performed at turn R already persisted snapshots on and after R against
        // the new baseline. Only snapshots strictly before that rebuilt turn need diff-reminder
        // cleanup, so the predicate is `< cutoff`, not `<= cutoff`.
        if turn_index >= cutoff_turn_index {
            return messages;
        }

        let (sanitized_messages, changed) = Self::strip_listing_diff_internal_reminders(messages);
        if !changed {
            return sanitized_messages;
        }

        debug!(
            "Sanitized listing diff reminders from pre-rebuild context snapshot: session_id={}, turn_index={}, cutoff_turn_index={}, reason={}",
            session_id, turn_index, cutoff_turn_index, reason
        );
        self.persist_context_snapshot_messages_best_effort(
            workspace_path,
            session_id,
            turn_index,
            &sanitized_messages,
            reason,
        )
        .await;
        sanitized_messages
    }

    pub(crate) fn reset_session_state_if_processing(&self, session_id: &str, expected_turn_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            if matches!(
                &session.state,
                SessionState::Processing {
                    current_turn_id,
                    ..
                } if current_turn_id == expected_turn_id
            ) {
                debug!(
                    "RAII guard resetting stuck Processing state to Idle: session_id={}, turn_id={}",
                    session_id, expected_turn_id
                );
                session.state = SessionState::Idle;
                session.updated_at = SystemTime::now();
                session.last_activity_at = SystemTime::now();
            }
        }
    }
}
