use super::super::scheduler_types::{DialogScheduler, DialogSubmissionPolicy, DialogSubmitOutcome, QueuedTurn};
use crate::agentic::core::{InternalReminderKind, Message, SessionState};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::init_agents_md::build_init_agents_md_user_input;
use northhing_agent_runtime::scheduler::ActiveDialogTurn;
use northhing_runtime_ports::{
    resolve_dialog_submit_queue_action, AgentDialogPrependedReminder, AgentDialogTurnPort, AgentDialogTurnRequest,
    AgentInputAttachment, AgentSessionReplyRoute, DialogSubmitQueueAction, DialogSubmitQueueFacts, PortError,
    PortErrorKind, PortResult,
};
use std::path::Path;
use std::time::SystemTime;
use tracing::warn;
use uuid::Uuid;

impl DialogScheduler {
    #[allow(clippy::too_many_arguments)]
    pub async fn submit(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        policy: DialogSubmissionPolicy,
        reply_route: Option<AgentSessionReplyRoute>,
        user_message_metadata: Option<serde_json::Value>,
        image_contexts: Option<Vec<ImageContextData>>,
    ) -> Result<DialogSubmitOutcome, String> {
        self.submit_with_prepended_messages(
            session_id,
            user_input,
            original_user_input,
            turn_id,
            agent_type,
            workspace_path,
            policy,
            reply_route,
            user_message_metadata,
            Vec::new(),
            image_contexts,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn submit_with_prepended_messages(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        policy: DialogSubmissionPolicy,
        reply_route: Option<AgentSessionReplyRoute>,
        user_message_metadata: Option<serde_json::Value>,
        prepended_messages: Vec<Message>,
        image_contexts: Option<Vec<ImageContextData>>,
    ) -> Result<DialogSubmitOutcome, String> {
        let resolved_turn_id = turn_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let queued_turn = QueuedTurn {
            user_input,
            original_user_input,
            prepended_messages,
            turn_id: Some(resolved_turn_id.clone()),
            agent_type,
            workspace_path,
            policy,
            reply_route,
            user_message_metadata,
            image_contexts,
            enqueued_at: SystemTime::now(),
        };
        self.submit_queued_turn(session_id, resolved_turn_id, queued_turn).await
    }

    async fn submit_queued_turn(
        &self,
        session_id: String,
        resolved_turn_id: String,
        queued_turn: QueuedTurn,
    ) -> Result<DialogSubmitOutcome, String> {
        let state = self.session_manager.get_session(&session_id).map(|s| s.state.clone());

        let queue_has_items = self.queues.has_items(&session_id);
        let action = resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: Self::session_state_fact(state.as_ref()),
            queue_has_items,
            policy: queued_turn.policy,
        });

        match action {
            DialogSubmitQueueAction::StartImmediately => {
                let tid = self.start_turn(&session_id, &queued_turn).await?;
                self.record_last_submitted_agent_type(&session_id, &queued_turn.agent_type)
                    .await;
                Ok(DialogSubmitOutcome::Started {
                    session_id,
                    turn_id: tid,
                })
            }

            DialogSubmitQueueAction::ClearQueueAndStartImmediately => {
                self.clear_queue(&session_id);
                let tid = self.start_turn(&session_id, &queued_turn).await?;
                self.record_last_submitted_agent_type(&session_id, &queued_turn.agent_type)
                    .await;
                Ok(DialogSubmitOutcome::Started {
                    session_id,
                    turn_id: tid,
                })
            }

            DialogSubmitQueueAction::EnqueueThenStartNext => {
                self.enqueue(&session_id, queued_turn.clone())?;
                self.record_last_submitted_agent_type(&session_id, &queued_turn.agent_type)
                    .await;
                let started_tid = self.try_start_next_queued(&session_id).await?;
                let outcome = match started_tid {
                    Some(tid) if tid == resolved_turn_id => DialogSubmitOutcome::Started {
                        session_id: session_id.clone(),
                        turn_id: tid,
                    },
                    _ => DialogSubmitOutcome::Queued {
                        session_id: session_id.clone(),
                        turn_id: resolved_turn_id,
                    },
                };
                Ok(outcome)
            }

            DialogSubmitQueueAction::EnqueueForActiveTurn => {
                let accepted_agent_type = queued_turn.agent_type.clone();
                self.enqueue(&session_id, queued_turn)?;
                self.record_last_submitted_agent_type(&session_id, &accepted_agent_type)
                    .await;
                Ok(DialogSubmitOutcome::Queued {
                    session_id,
                    turn_id: resolved_turn_id,
                })
            }
        }
    }

    async fn resolve_session_agent_type(
        &self,
        session_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<String, String> {
        let session = match self.session_manager.get_session(session_id) {
            Some(session) => session,
            None => {
                let workspace_path = workspace_path
                    .ok_or_else(|| format!("workspace_path is required when restoring session: {}", session_id))?;
                self.session_manager
                    .restore_session(Path::new(workspace_path), session_id)
                    .await
                    .map_err(|error| error.to_string())?
            }
        };
        let agent_type = session.agent_type.trim();
        if agent_type.is_empty() {
            Ok("agentic".to_string())
        } else {
            Ok(agent_type.to_string())
        }
    }

    async fn record_last_submitted_agent_type(&self, session_id: &str, agent_type: &str) {
        if let Err(error) = self
            .coordinator
            .update_last_submitted_agent_type(session_id, agent_type)
            .await
        {
            warn!(
                "Failed to record last submitted agent type: session_id={}, agent_type={}, error={}",
                session_id, agent_type, error
            );
        }
    }

    async fn start_turn(&self, session_id: &str, queued_turn: &QueuedTurn) -> Result<String, String> {
        let res = match queued_turn.image_contexts.as_ref().filter(|imgs| !imgs.is_empty()) {
            Some(imgs) => {
                if queued_turn.prepended_messages.is_empty() {
                    self.coordinator
                        .start_dialog_turn_with_image_contexts(
                            session_id.to_string(),
                            queued_turn.user_input.clone(),
                            queued_turn.original_user_input.clone(),
                            imgs.clone(),
                            queued_turn.turn_id.clone(),
                            queued_turn.agent_type.clone(),
                            queued_turn.workspace_path.clone(),
                            queued_turn.policy,
                            queued_turn.user_message_metadata.clone(),
                        )
                        .await
                } else {
                    self.coordinator
                        .start_dialog_turn_with_image_contexts_and_prepended_messages(
                            session_id.to_string(),
                            queued_turn.user_input.clone(),
                            queued_turn.original_user_input.clone(),
                            imgs.clone(),
                            queued_turn.turn_id.clone(),
                            queued_turn.agent_type.clone(),
                            queued_turn.workspace_path.clone(),
                            queued_turn.policy,
                            queued_turn.user_message_metadata.clone(),
                            queued_turn.prepended_messages.clone(),
                        )
                        .await
                }
            }
            None => {
                if queued_turn.prepended_messages.is_empty() {
                    self.coordinator
                        .start_dialog_turn(
                            session_id.to_string(),
                            queued_turn.user_input.clone(),
                            queued_turn.original_user_input.clone(),
                            queued_turn.turn_id.clone(),
                            queued_turn.agent_type.clone(),
                            queued_turn.workspace_path.clone(),
                            queued_turn.policy,
                            queued_turn.user_message_metadata.clone(),
                        )
                        .await
                } else {
                    self.coordinator
                        .start_dialog_turn_with_prepended_messages(
                            session_id.to_string(),
                            queued_turn.user_input.clone(),
                            queued_turn.original_user_input.clone(),
                            queued_turn.turn_id.clone(),
                            queued_turn.agent_type.clone(),
                            queued_turn.workspace_path.clone(),
                            queued_turn.policy,
                            queued_turn.user_message_metadata.clone(),
                            queued_turn.prepended_messages.clone(),
                        )
                        .await
                }
            }
        };

        res.map_err(|e| e.to_string())?;

        let resolved = self
            .session_manager
            .get_session(session_id)
            .and_then(|s| match &s.state {
                SessionState::Processing { current_turn_id, .. } => Some(current_turn_id.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "Failed to resolve turn_id after starting dialog: session_id={}",
                    session_id
                )
            })?;

        self.active_turns.insert(
            session_id,
            ActiveDialogTurn::new(
                resolved.clone(),
                queued_turn.workspace_path.clone(),
                queued_turn.agent_type.clone(),
                queued_turn
                    .original_user_input
                    .clone()
                    .unwrap_or_else(|| queued_turn.user_input.clone()),
                queued_turn.user_message_metadata.clone(),
                queued_turn.policy,
                queued_turn.reply_route.clone(),
            ),
        );

        Ok(resolved)
    }

    pub async fn submit_init_agents_md(
        &self,
        session_id: String,
        workspace_path: Option<String>,
        policy: DialogSubmissionPolicy,
    ) -> Result<DialogSubmitOutcome, String> {
        let agent_type = self
            .resolve_session_agent_type(&session_id, workspace_path.as_deref())
            .await?;
        let (user_input, prepended_messages) = build_init_agents_md_user_input()
            .await
            .map_err(|error| error.to_string())?;

        self.submit_with_prepended_messages(
            session_id,
            user_input.clone(),
            Some(user_input),
            None,
            agent_type,
            workspace_path,
            policy,
            None,
            None,
            prepended_messages,
            None,
        )
        .await
    }

    pub(crate) async fn try_start_next_queued(&self, session_id: &str) -> Result<Option<String>, String> {
        let state = self.session_manager.get_session(session_id).map(|s| s.state.clone());
        if matches!(state, Some(SessionState::Processing { .. })) {
            return Ok(None);
        }

        let Some(next_turn) = self.dequeue_next(session_id) else {
            return Ok(None);
        };

        let remaining = self.queues.depth(session_id);
        tracing::info!(
            "Dispatching queued message: session_id={}, priority={:?}, remaining_queue_depth={}",
            session_id,
            next_turn.policy.queue_priority,
            remaining
        );

        match self.start_turn(session_id, &next_turn).await {
            Ok(tid) => Ok(Some(tid)),
            Err(err) => {
                self.requeue_front(session_id, next_turn);
                Err(err)
            }
        }
    }
}

fn metadata_string(metadata: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn mime_type_from_data_url(data_url: &str) -> Option<String> {
    data_url
        .split_once(',')
        .and_then(|(header, _)| header.strip_prefix("data:").and_then(|rest| rest.split(';').next()))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn image_context_metadata(attachment: &AgentInputAttachment) -> Option<serde_json::Value> {
    if let Some(metadata) = attachment.metadata.get("metadata").cloned() {
        return Some(metadata);
    }

    let mut metadata = serde_json::Map::new();
    if let Some(name) = metadata_string(&attachment.metadata, "name") {
        metadata.insert("name".to_string(), serde_json::Value::String(name));
    }
    if attachment.metadata.contains_key("dataUrl") {
        metadata.insert("source".to_string(), serde_json::Value::String("remote".to_string()));
    }

    if metadata.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(metadata))
    }
}

fn agent_dialog_turn_image_contexts(attachments: &[AgentInputAttachment]) -> PortResult<Option<Vec<ImageContextData>>> {
    if attachments.is_empty() {
        return Ok(None);
    }

    let mut image_contexts = Vec::with_capacity(attachments.len());
    for attachment in attachments {
        if attachment.kind != "remote_image" {
            return Err(PortError::new(
                PortErrorKind::InvalidRequest,
                format!("unsupported agent dialog attachment kind: {}", attachment.kind),
            ));
        }

        let data_url = metadata_string(&attachment.metadata, "dataUrl");
        let image_path = metadata_string(&attachment.metadata, "imagePath");
        if data_url.is_none() && image_path.is_none() {
            return Err(PortError::new(
                PortErrorKind::InvalidRequest,
                "remote_image attachment requires dataUrl or imagePath",
            ));
        }

        let mime_type = metadata_string(&attachment.metadata, "mimeType")
            .or_else(|| data_url.as_deref().and_then(mime_type_from_data_url))
            .unwrap_or_else(|| "image/png".to_string());

        image_contexts.push(ImageContextData {
            id: attachment.id.clone(),
            image_path,
            data_url,
            mime_type,
            metadata: image_context_metadata(attachment),
        });
    }

    Ok(Some(image_contexts))
}

fn agent_dialog_turn_prepended_messages(reminders: &[AgentDialogPrependedReminder]) -> PortResult<Vec<Message>> {
    reminders
        .iter()
        .map(|reminder| {
            let kind = match reminder.kind.as_str() {
                "session_message_request" => InternalReminderKind::SessionMessageRequest,
                "scheduled_job" => InternalReminderKind::ScheduledJob,
                other => {
                    return Err(PortError::new(
                        PortErrorKind::InvalidRequest,
                        format!("unsupported agent dialog prepended reminder kind: {other}"),
                    ));
                }
            };
            Ok(Message::internal_reminder(kind, reminder.text.clone()))
        })
        .collect()
}

#[async_trait::async_trait]
impl AgentDialogTurnPort for DialogScheduler {
    async fn submit_dialog_turn(&self, request: AgentDialogTurnRequest) -> PortResult<DialogSubmitOutcome> {
        let image_contexts = agent_dialog_turn_image_contexts(&request.attachments)?;
        let prepended_messages = agent_dialog_turn_prepended_messages(&request.prepended_reminders)?;
        let user_message_metadata = if request.metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(request.metadata))
        };

        self.submit_with_prepended_messages(
            request.session_id,
            request.message,
            request.original_message,
            request.turn_id,
            request.agent_type,
            request.workspace_path,
            request.policy,
            request.reply_route,
            user_message_metadata,
            prepended_messages,
            image_contexts,
        )
        .await
        .map_err(|error| PortError::new(PortErrorKind::Backend, error))
    }
}
