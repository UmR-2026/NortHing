//! Sub-domain: turn_start.
//! Spec §2.1 — facade methods extracted from dialog_turn/mod.rs (R44a refactor).
//! Contains 4 thin wrappers around `start_dialog_turn_internal` that handle the
//! image-context / prepended-message combinations.

use super::super::coordinator::*;
use super::super::ports::*;
use super::super::scheduler::*;

use crate::agentic::core::Message;
use crate::agentic::image_analysis::ImageContextData;
use crate::util::errors::NortHingResult;

impl ConversationCoordinator {
    /// Start a new dialog turn
    /// Note: Events are sent to frontend via EventLoop, no Stream returned.
    /// Submission behavior is controlled by `submission_policy`, which provides
    /// default per-source behavior while still allowing selective overrides.
    #[allow(clippy::too_many_arguments)]
    pub async fn start_dialog_turn(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<()> {
        self.start_dialog_turn_internal(
            session_id,
            user_input,
            original_user_input,
            None,
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            user_message_metadata,
            Vec::new(),
            false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_dialog_turn_with_prepended_messages(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        user_message_metadata: Option<serde_json::Value>,
        prepended_messages: Vec<Message>,
    ) -> NortHingResult<()> {
        self.start_dialog_turn_internal(
            session_id,
            user_input,
            original_user_input,
            None,
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            user_message_metadata,
            prepended_messages,
            false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_dialog_turn_with_image_contexts(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        image_contexts: Vec<ImageContextData>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        user_message_metadata: Option<serde_json::Value>,
    ) -> NortHingResult<()> {
        self.start_dialog_turn_internal(
            session_id,
            user_input,
            original_user_input,
            Some(image_contexts),
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            user_message_metadata,
            Vec::new(),
            false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_dialog_turn_with_image_contexts_and_prepended_messages(
        &self,
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        image_contexts: Vec<ImageContextData>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        user_message_metadata: Option<serde_json::Value>,
        prepended_messages: Vec<Message>,
    ) -> NortHingResult<()> {
        self.start_dialog_turn_internal(
            session_id,
            user_input,
            original_user_input,
            Some(image_contexts),
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            user_message_metadata,
            prepended_messages,
            false,
        )
        .await
    }
}
