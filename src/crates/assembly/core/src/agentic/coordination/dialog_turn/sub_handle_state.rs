//! Sub-domain: turn_subhandlers state/dispatch phase (R47b refactor).
//!
//! Owns `dispatch_turn` — the second of the 4 sub-handler phases. Builds the
//! workspace binding/services, wraps the user input, attaches image-context
//! metadata, starts the dialog turn in the session manager, marks thread goal
//! turn-start, and persists the skill-agent snapshot. Populates the second half
//! of `TurnContext` (post-`prepare_turn`).
//!
//! Spec §2.1 R47b — extracted from `turn_subhandlers.rs` god-file.
//! Sibling imports `use super::super::coordinator::*` for the struct and
//! `use super::super::scheduler::DialogSubmissionPolicy` for the policy type.

use super::super::coordinator::*;
use super::super::scheduler::*;
use super::super::scheduler::{
    abort_thread_goal_continuation_for_session, clear_thread_goal_continuation_abort, DialogSubmissionPolicy,
};

use super::sub_handle_types::TurnContext;

use crate::agentic::core::{InternalReminderKind, Message};
use crate::agentic::goal_mode::should_skip_goal_for_turn;
use crate::agentic::remote_file_delivery::{needs_computer_links_for_source, remote_file_delivery_reminder};
use crate::util::errors::NortHingResult;
use std::collections::HashMap;
use tracing::info;

impl ConversationCoordinator {
    pub(super) async fn dispatch_turn(&self, ctx: &mut TurnContext) -> NortHingResult<()> {
        let session_id = ctx.session_id.clone();
        let turn_id = ctx.turn_id.clone();
        let user_input = ctx.user_input.clone();
        let original_user_input = ctx.original_user_input.clone();
        let image_contexts = ctx.image_contexts.clone();
        let submission_policy = ctx.submission_policy.clone();
        let additional_prepended_messages = ctx.additional_prepended_messages.clone();
        let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
        let session = ctx.session.clone().expect("prepare_turn must set ctx.session first");
        let effective_agent_type = ctx.effective_agent_type.clone();
        let previous_agent_type = ctx.previous_agent_type.clone();
        let turn_index = ctx.turn_index;
        let original_user_input = original_user_input.unwrap_or_else(|| user_input.clone());
        let mut user_message_metadata = extra_user_message_metadata;
        if let Some(imgs) = image_contexts.as_ref().filter(|imgs| !imgs.is_empty()) {
            let image_meta: Vec<serde_json::Value> = imgs
                .iter()
                .map(|img| {
                    let name = img
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("image.png");
                    let mut meta = serde_json::json!({
                        "id": &img.id,
                        "name": name,
                        "mime_type": &img.mime_type,
                    });
                    if let Some(url) = &img.data_url {
                        meta["data_url"] = serde_json::json!(url);
                    }
                    if let Some(path) = &img.image_path {
                        meta["image_path"] = serde_json::json!(path);
                    }
                    meta
                })
                .collect();
            let mut metadata = Self::ensure_user_message_metadata_object(user_message_metadata.take());
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert("images".to_string(), serde_json::json!(image_meta));
                obj.insert(
                    "original_text".to_string(),
                    serde_json::json!(original_user_input.clone()),
                );
            }
            user_message_metadata = Some(metadata);
        }
        let session_workspace = Self::build_workspace_binding(&session.config).await;
        let workspace_services = Self::build_workspace_services(&session_workspace).await;
        info!(
            "Dialog turn workspace context: session_id={}, workspace_path={:?}, is_remote={}, workspace_services={}",
            session_id,
            session.config.workspace_path,
            session_workspace.as_ref().map(|ws| ws.is_remote()).unwrap_or(false),
            if workspace_services.is_some() {
                "available"
            } else {
                "NONE"
            }
        );
        let turn_index = self.session_manager.get_turn_count(&session_id);
        let mut skill_agent_context_vars = HashMap::new();
        if user_message_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("acp_transport"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            skill_agent_context_vars.insert("acp_transport".to_string(), "true".to_string());
        }
        let wrapped_user_input_payload = self
            .wrap_user_input(
                &session_id,
                turn_index,
                &effective_agent_type,
                previous_agent_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                user_input,
                session_workspace.as_ref(),
                workspace_services.as_ref(),
                session.config.enable_tools,
                &skill_agent_context_vars,
            )
            .await?;
        let effective_user_input = wrapped_user_input_payload.content.clone();
        let mut prepended_messages = additional_prepended_messages;
        if needs_computer_links_for_source(submission_policy.trigger_source) {
            prepended_messages.push(Message::internal_reminder(
                InternalReminderKind::RemoteFileDelivery,
                remote_file_delivery_reminder(),
            ));
        }
        prepended_messages.extend(wrapped_user_input_payload.prepended_messages.clone());
        if original_user_input != effective_user_input {
            let mut metadata = Self::ensure_user_message_metadata_object(user_message_metadata.take());
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert(
                    "original_text".to_string(),
                    serde_json::json!(original_user_input.clone()),
                );
            }
            user_message_metadata = Some(metadata);
        }
        let turn_id = self
            .session_manager
            .start_dialog_turn_with_prepended_messages(
                &session_id,
                effective_agent_type.clone(),
                effective_user_input.clone(),
                turn_id,
                image_contexts,
                prepended_messages,
                user_message_metadata.clone(),
            )
            .await?;
        if let Ok(Some(goal)) = self.load_active_thread_goal(&session_id).await {
            if !should_skip_goal_for_turn(&original_user_input, user_message_metadata.as_ref()) {
                self.thread_goal_runtime.mark_turn_started(&turn_id, Some(&goal));
            }
        }
        match wrapped_user_input_payload.snapshot_persistence {
            SkillAgentSnapshotPersistence::None => {}
            SkillAgentSnapshotPersistence::SaveCurrentTurn => {
                self.session_manager
                    .remember_turn_skill_agent_snapshot(
                        &session_id,
                        turn_index,
                        wrapped_user_input_payload.skill_agent_snapshot.clone(),
                    )
                    .await;
            }
            SkillAgentSnapshotPersistence::RecoverFirstTurnBaseline => {
                self.session_manager
                    .recover_first_turn_skill_agent_snapshot(
                        &session_id,
                        wrapped_user_input_payload.skill_agent_snapshot.clone(),
                    )
                    .await;
                self.session_manager
                    .remove_listing_diff_internal_reminders(&session_id)
                    .await;
            }
        }
        ctx.user_message_metadata = user_message_metadata;
        ctx.session_workspace = session_workspace;
        ctx.workspace_services = workspace_services;
        ctx.effective_user_input = effective_user_input;
        ctx.wrapped_user_input_payload = wrapped_user_input_payload;
        ctx.final_turn_id = turn_id;
        ctx.turn_index = turn_index;
        Ok(())
    }
}
