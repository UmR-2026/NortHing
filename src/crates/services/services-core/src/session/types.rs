//! Types for session persistence
//!
//! Facade module: re-exports the sibling submodules so the historical
//! `crate::session::types::*` import surface keeps working after the R38d
//! god-split. New code should keep importing from `northhing_services_core::session`
//! (the parent `pub use` in `crate::session`) — this facade exists only for
//! backwards compatibility with code that explicitly walked the
//! `session::types::Foo` path.
//!
//! Sibling ownership:
//! - `dialog_turn`      — DialogTurnData / DialogTurnTokenUsageData / DialogTurnKind / TurnStatus
//! - `model_round`      — ModelRoundData and the per-round item DTOs (User/Text/Thinking/Tool + ToolCall/Result)
//! - `session_metadata` — SessionMetadata, relationship, status, list, on-disk wrappers, schema version
//! - `transcript`       — Transcript export DTOs and the index entry / line range

pub use super::dialog_turn::*;
pub use super::model_round::*;
pub use super::session_metadata::*;
pub use super::transcript::*;

#[cfg(test)]
mod tests {
    use super::{
        DialogTurnData, DialogTurnKind, ModelRoundData, SessionMetadata, SessionRelationship, SessionRelationshipKind,
        ToolItemData, UserMessageData,
    };
    use northhing_core_types::SessionKind;

    #[test]
    fn dialog_turn_kind_defaults_to_user_dialog_for_legacy_payloads() {
        let payload = serde_json::json!({
            "turnId": "turn-1",
            "turnIndex": 0,
            "sessionId": "session-1",
            "timestamp": 1,
            "userMessage": {
                "id": "user-1",
                "content": "hello",
                "timestamp": 1
            },
            "modelRounds": [],
            "startTime": 1,
            "status": "completed"
        });

        let turn: DialogTurnData = serde_json::from_value(payload).expect("legacy payload should deserialize");

        assert_eq!(turn.kind, DialogTurnKind::UserDialog);
    }

    #[test]
    fn dialog_turn_data_new_defaults_to_user_dialog() {
        let turn = DialogTurnData::new(
            "turn-1".to_string(),
            0,
            "session-1".to_string(),
            UserMessageData {
                id: "user-1".to_string(),
                content: "hello".to_string(),
                timestamp: 1,
                metadata: None,
            },
        );

        assert_eq!(turn.kind, DialogTurnKind::UserDialog);
    }

    #[test]
    fn dialog_turn_token_usage_round_trips_camel_case_payloads() {
        let payload = serde_json::json!({
            "turnId": "turn-1",
            "turnIndex": 0,
            "sessionId": "session-1",
            "timestamp": 1,
            "userMessage": {
                "id": "user-1",
                "content": "hello",
                "timestamp": 1
            },
            "modelRounds": [],
            "startTime": 1,
            "durationMs": 10,
            "tokenUsage": {
                "inputTokens": 1200,
                "outputTokens": 320,
                "totalTokens": 1520,
                "timestamp": 2
            },
            "status": "completed"
        });

        let turn: DialogTurnData = serde_json::from_value(payload).expect("turn payload should deserialize");

        let token_usage = turn.token_usage.as_ref().expect("token usage should be preserved");
        assert_eq!(token_usage.input_tokens, 1200);
        assert_eq!(token_usage.output_tokens, Some(320));
        assert_eq!(token_usage.total_tokens, 1520);

        let serialized = serde_json::to_value(&turn).expect("turn should serialize");
        assert_eq!(serialized["tokenUsage"]["totalTokens"], 1520);
    }

    #[test]
    fn local_usage_report_turn_is_model_invisible() {
        assert!(!DialogTurnKind::LocalCommand.is_model_visible());
    }

    #[test]
    fn manual_compaction_turn_is_model_invisible() {
        assert!(!DialogTurnKind::ManualCompaction.is_model_visible());
    }

    #[test]
    fn session_metadata_marks_explicit_subagent_as_non_standard() {
        let mut metadata = SessionMetadata::new(
            "session-1".to_string(),
            "Subagent: explore repo".to_string(),
            "Explore".to_string(),
            "model".to_string(),
        );
        metadata.session_kind = SessionKind::Subagent;

        assert!(metadata.is_subagent());
        assert!(!metadata.is_standard());
    }

    #[test]
    fn session_metadata_does_not_treat_standard_session_as_subagent_from_name_or_creator() {
        let mut metadata = SessionMetadata::new(
            "session-1".to_string(),
            "Subagent: repo sweep".to_string(),
            "Explore".to_string(),
            "model".to_string(),
        );
        metadata.created_by = Some("session-parent".to_string());

        assert!(!metadata.is_subagent());
        assert!(metadata.is_standard());
    }

    #[test]
    fn session_metadata_detects_legacy_leaked_subagent_candidate() {
        let mut metadata = SessionMetadata::new(
            "session-1".to_string(),
            "Subagent: repo sweep".to_string(),
            "Explore".to_string(),
            "model".to_string(),
        );
        metadata.created_by = Some("session-parent".to_string());

        assert!(!metadata.is_subagent());
        assert!(metadata.is_legacy_leaked_subagent_candidate());
        assert!(metadata.should_hide_from_user_lists());
    }

    #[test]
    fn session_relationship_round_trips_through_metadata_contract() {
        let mut metadata = SessionMetadata::new(
            "session-relationship".to_string(),
            "Review child".to_string(),
            "CodeReview".to_string(),
            "model".to_string(),
        );
        metadata.relationship = Some(SessionRelationship {
            kind: Some(SessionRelationshipKind::Review),
            parent_session_id: Some("parent-1".to_string()),
            parent_request_id: Some("request-1".to_string()),
            parent_dialog_turn_id: Some("turn-2".to_string()),
            parent_turn_index: Some(2),
            parent_tool_call_id: None,
            subagent_type: None,
        });

        let json = serde_json::to_value(&metadata).expect("metadata should serialize");
        let round_trip: SessionMetadata = serde_json::from_value(json).expect("metadata should deserialize");

        assert_eq!(round_trip.relationship, metadata.relationship);
    }

    #[test]
    fn session_metadata_keeps_normal_sessions_visible() {
        let metadata = SessionMetadata::new(
            "session-1".to_string(),
            "Normal Session".to_string(),
            "agentic".to_string(),
            "model".to_string(),
        );

        assert!(!metadata.is_subagent());
        assert!(metadata.is_standard());
    }

    #[test]
    fn persisted_runtime_span_fields_are_optional_and_round_trip() {
        let legacy_round_payload = serde_json::json!({
            "id": "round-legacy",
            "turnId": "turn-1",
            "roundIndex": 0,
            "timestamp": 1,
            "textItems": [],
            "toolItems": [],
            "thinkingItems": [],
            "startTime": 1,
            "endTime": 2,
            "status": "completed"
        });

        let legacy_round: ModelRoundData =
            serde_json::from_value(legacy_round_payload).expect("legacy round should deserialize");
        assert_eq!(legacy_round.duration_ms, None);
        assert_eq!(legacy_round.model_id, None);
        assert_eq!(legacy_round.first_chunk_ms, None);

        let round_payload = serde_json::json!({
            "id": "round-1",
            "turnId": "turn-1",
            "roundIndex": 0,
            "timestamp": 1,
            "textItems": [],
            "toolItems": [],
            "thinkingItems": [],
            "startTime": 1,
            "endTime": 121,
            "durationMs": 120,
            "providerId": "provider-a",
            "modelId": "model-a",
            "modelAlias": "Model A",
            "firstChunkMs": 10,
            "firstVisibleOutputMs": 12,
            "streamDurationMs": 90,
            "attemptCount": 2,
            "failureCategory": "rate_limit",
            "tokenDetails": { "reasoningTokens": 7 },
            "status": "completed"
        });

        let round: ModelRoundData = serde_json::from_value(round_payload).expect("P1 round should deserialize");
        assert_eq!(round.duration_ms, Some(120));
        assert_eq!(round.provider_id.as_deref(), Some("provider-a"));
        assert_eq!(round.model_id.as_deref(), Some("model-a"));
        assert_eq!(round.first_visible_output_ms, Some(12));
        assert_eq!(round.attempt_count, Some(2));
        assert_eq!(round.failure_category.as_deref(), Some("rate_limit"));

        let encoded = serde_json::to_value(&round).expect("round should serialize");
        assert_eq!(encoded["durationMs"], 120);
        assert_eq!(encoded["modelId"], "model-a");
        assert_eq!(encoded["firstChunkMs"], 10);

        let tool_payload = serde_json::json!({
            "id": "tool-1",
            "toolName": "write_file",
            "toolCall": { "id": "call-1", "input": { "file_path": "src/main.rs" } },
            "startTime": 5,
            "endTime": 105,
            "durationMs": 100,
            "queueWaitMs": 7,
            "preflightMs": 11,
            "confirmationWaitMs": 13,
            "executionMs": 69,
            "status": "completed"
        });

        let tool: ToolItemData = serde_json::from_value(tool_payload).expect("P1 tool should deserialize");
        assert_eq!(tool.queue_wait_ms, Some(7));
        assert_eq!(tool.preflight_ms, Some(11));
        assert_eq!(tool.confirmation_wait_ms, Some(13));
        assert_eq!(tool.execution_ms, Some(69));

        let encoded = serde_json::to_value(&tool).expect("tool should serialize");
        assert_eq!(encoded["queueWaitMs"], 7);
        assert_eq!(encoded["executionMs"], 69);
    }

    #[test]
    fn session_metadata_preserves_deep_review_run_manifest() {
        let payload = serde_json::json!({
            "sessionId": "session-1",
            "sessionName": "Deep Review",
            "agentType": "DeepReview",
            "sessionKind": "standard",
            "modelName": "fast",
            "createdAt": 1,
            "lastActiveAt": 1,
            "turnCount": 0,
            "messageCount": 0,
            "toolCallCount": 0,
            "status": "active",
            "deep_review_run_manifest": {
                "reviewMode": "deep",
                "coreReviewers": [
                    { "subagentId": "ReviewBusinessLogic" }
                ],
                "skippedReviewers": [
                    { "subagentId": "ReviewFrontend", "reason": "not_applicable" }
                ]
            }
        });

        let metadata: SessionMetadata = serde_json::from_value(payload).expect("metadata should deserialize");

        assert_eq!(
            metadata.deep_review_run_manifest.as_ref().unwrap()["reviewMode"],
            "deep"
        );

        let serialized = serde_json::to_value(&metadata).expect("metadata should serialize");
        assert_eq!(
            serialized["deepReviewRunManifest"]["coreReviewers"][0]["subagentId"],
            "ReviewBusinessLogic"
        );
    }
}
