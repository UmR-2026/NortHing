//! DTO conversion helpers.

use crate::agentic::core::{Message, MessageContent, MessageRole};
use crate::agentic::message::MessageMetadata;
use northhing_kernel_api::session::{
    MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto, SessionMetadataDto,
    ToolCallStub,
};
use northhing_kernel_api::turn::DialogSubmitOutcomeDto;

/// Converts a Message to MessageDto.
pub(crate) fn message_to_dto(m: Message) -> MessageDto {
    MessageDto {
        id: m.id,
        role: match m.role {
            MessageRole::User => MessageRoleDto::User,
            MessageRole::Assistant => MessageRoleDto::Assistant,
            MessageRole::Tool => MessageRoleDto::Tool,
            MessageRole::System => MessageRoleDto::System,
        },
        content: match &m.content {
            MessageContent::Text(t) => MessageContentDto::Text(t.clone()),
            MessageContent::Multimodal { text, images } => MessageContentDto::Multimodal {
                text: text.clone(),
                images: images.iter().filter_map(|img| img.image_path.clone()).collect(),
            },
            MessageContent::ToolResult {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                is_error,
                ..
            } => MessageContentDto::ToolResult {
                tool_id: tool_id.clone(),
                tool_name: tool_name.clone(),
                result: result.clone(),
                result_for_assistant: result_for_assistant.clone(),
                is_error: *is_error,
            },
            MessageContent::Mixed {
                reasoning_content,
                text,
                tool_calls,
            } => MessageContentDto::Mixed {
                reasoning_content: reasoning_content.clone(),
                text: text.clone(),
                tool_calls: tool_calls
                    .iter()
                    .map(|tc| ToolCallStub {
                        tool_name: tc.tool_name.clone(),
                        arguments: Some(tc.arguments.clone()),
                        is_error: tc.is_error,
                    })
                    .collect(),
            },
        },
        metadata: Some(metadata_to_message_dto(&m.metadata)),
    }
}

/// Converts MessageMetadata to MessageMetadataDto.
pub(crate) fn metadata_to_message_dto(m: &MessageMetadata) -> MessageMetadataDto {
    MessageMetadataDto {
        turn_id: m.turn_id.clone(),
        round_id: m.round_id.clone(),
        tokens: m.tokens,
        thinking_signature: m.thinking_signature.clone(),
        semantic_kind: m.semantic_kind.as_ref().map(|k| format!("{:?}", k)),
        internal_reminder_kind: m.internal_reminder_kind.as_ref().map(|k| format!("{:?}", k)),
        compression_payload: m.compression_payload.as_ref().map(|p| {
            serde_json::to_value(p).unwrap_or(serde_json::Value::Null)
        }),
    }
}

/// Converts SessionMetadata to SessionMetadataDto.
pub(crate) fn metadata_to_dto(
    m: &northhing_services_core::session::SessionMetadata,
) -> SessionMetadataDto {
    SessionMetadataDto {
        session_id: m.session_id.clone(),
        session_name: m.session_name.clone(),
        agent_type: m.agent_type.clone(),
        last_user_dialog_agent_type: m.last_user_dialog_agent_type.clone(),
        last_submitted_agent_type: m.last_submitted_agent_type.clone(),
        created_by: m.created_by.clone(),
        session_kind: match m.session_kind {
            northhing_core_types::SessionKind::Standard => super::SessionKindDto::Standard,
            northhing_core_types::SessionKind::Subagent => super::SessionKindDto::Subagent,
            northhing_core_types::SessionKind::EphemeralChild => super::SessionKindDto::EphemeralChild,
        },
        model_name: m.model_name.clone(),
        created_at: m.created_at,
        last_active_at: m.last_active_at,
        turn_count: m.turn_count,
        message_count: m.message_count,
        tool_call_count: m.tool_call_count,
        status: match m.status {
            northhing_services_core::session::SessionStatus::Active => super::SessionStatusDto::Active,
            northhing_services_core::session::SessionStatus::Archived => super::SessionStatusDto::Archived,
            northhing_services_core::session::SessionStatus::Completed => super::SessionStatusDto::Completed,
        },
        terminal_session_id: m.terminal_session_id.clone(),
        snapshot_session_id: m.snapshot_session_id.clone(),
        tags: m.tags.clone(),
        custom_metadata: m.custom_metadata.clone(),
        relationship: m.relationship.as_ref().map(|r| super::SessionRelationshipDto {
            kind: r.kind.as_ref().map(|k| format!("{k:?}")),
            parent_session_id: r.parent_session_id.clone(),
            parent_request_id: r.parent_request_id.clone(),
            parent_dialog_turn_id: r.parent_dialog_turn_id.clone(),
            parent_turn_index: r.parent_turn_index,
            parent_tool_call_id: r.parent_tool_call_id.clone(),
            subagent_type: r.subagent_type.clone(),
        }),
        todos: m.todos.clone(),
        deep_review_run_manifest: m.deep_review_run_manifest.clone(),
        deep_review_cache: m.deep_review_cache.clone(),
        workspace_path: m.workspace_path.clone(),
        workspace_hostname: m.workspace_hostname.clone(),
        unread_completion: m.unread_completion.clone(),
        needs_user_attention: m.needs_user_attention.clone(),
    }
}

/// Converts DialogSubmitOutcome to DialogSubmitOutcomeDto.
pub(crate) fn outcome_to_dto(
    o: crate::agentic::coordination::DialogSubmitOutcome,
) -> DialogSubmitOutcomeDto {
    use crate::agentic::coordination::DialogSubmitOutcome;
    match o {
        DialogSubmitOutcome::Started { turn_id, .. } => DialogSubmitOutcomeDto {
            turn_id,
            accepted: true,
            error: None,
        },
        DialogSubmitOutcome::Queued { turn_id, .. } => DialogSubmitOutcomeDto {
            turn_id,
            accepted: true,
            error: None,
        },
    }
}

/// Converts TurnStatus to TurnStateKind.
pub(crate) fn turn_status_to_kind(
    s: &northhing_services_core::session::TurnStatus,
) -> super::TurnStateKind {
    match s {
        northhing_services_core::session::TurnStatus::InProgress => super::TurnStateKind::Started,
        northhing_services_core::session::TurnStatus::Completed => super::TurnStateKind::Completed,
        northhing_services_core::session::TurnStatus::Error => super::TurnStateKind::Failed,
        northhing_services_core::session::TurnStatus::Cancelled => super::TurnStateKind::Cancelled,
    }
}
