//! Sub-domain: turn_subhandlers shared types (R47b refactor).
//!
//! Owns the `TurnContext` struct + `impl TurnContext::new` that flows through
//! the 4 sub-handler phases (`prepare_turn` -> `dispatch_turn` ->
//! `finalize_turn` -> `cleanup_turn`). Fields are populated progressively across
//! phases; methods live in `sub_handle_in.rs`, `sub_handle_state.rs`, and
//! `sub_handle_out.rs`.
//!
//! Spec §2.1 R47b — extracted from `turn_subhandlers.rs` god-file.
//! Sibling imports `use super::super::coordinator::*` for the `WrappedUserInputPayload`
//! and `SkillAgentSnapshotPersistence` types referenced by the struct fields.

use super::super::coordinator::*;
use super::super::scheduler::DialogSubmissionPolicy;

use crate::agentic::core::{Message, Session};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::agentic::workspace::WorkspaceServices;
use crate::agentic::WorkspaceBinding;

/// Shared mutable state for the 4 sub-handlers. Fields populated progressively
/// across `prepare_turn` -> `dispatch_turn` -> `finalize_turn` -> `cleanup_turn`.
#[allow(clippy::too_many_fields)]
pub(crate) struct TurnContext {
    pub session_id: String,
    pub user_input: String,
    pub original_user_input: Option<String>,
    pub image_contexts: Option<Vec<ImageContextData>>,
    pub turn_id: Option<String>,
    pub agent_type: String,
    pub workspace_path: Option<String>,
    pub submission_policy: DialogSubmissionPolicy,
    pub extra_user_message_metadata: Option<serde_json::Value>,
    pub additional_prepended_messages: Vec<Message>,
    pub suppress_session_title_generation: bool,
    pub session: Option<Session>,
    pub effective_agent_type: String,
    pub previous_agent_type: Option<String>,
    pub user_message_metadata: Option<serde_json::Value>,
    pub session_workspace: Option<WorkspaceBinding>,
    pub workspace_services: Option<WorkspaceServices>,
    pub effective_user_input: String,
    pub wrapped_user_input_payload: WrappedUserInputPayload,
    pub final_turn_id: String,
    pub turn_index: usize,
}

impl TurnContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        session_id: String,
        user_input: String,
        original_user_input: Option<String>,
        image_contexts: Option<Vec<ImageContextData>>,
        turn_id: Option<String>,
        agent_type: String,
        workspace_path: Option<String>,
        submission_policy: DialogSubmissionPolicy,
        extra_user_message_metadata: Option<serde_json::Value>,
        additional_prepended_messages: Vec<Message>,
        suppress_session_title_generation: bool,
    ) -> Self {
        Self {
            session_id,
            user_input,
            original_user_input,
            image_contexts,
            turn_id,
            agent_type,
            workspace_path,
            submission_policy,
            extra_user_message_metadata,
            additional_prepended_messages,
            suppress_session_title_generation,
            session: None,
            effective_agent_type: String::new(),
            previous_agent_type: None,
            user_message_metadata: None,
            session_workspace: None,
            workspace_services: None,
            effective_user_input: String::new(),
            wrapped_user_input_payload: WrappedUserInputPayload {
                content: String::new(),
                prepended_messages: Vec::new(),
                skill_agent_snapshot: TurnSkillAgentSnapshot::default(),
                snapshot_persistence: SkillAgentSnapshotPersistence::None,
            },
            final_turn_id: String::new(),
            turn_index: 0,
        }
    }
}
