//! Core-owned bindings for service and agent runtime ports.
//!
//! Owner crates keep portable contracts and orchestration policy. This module
//! centralizes the concrete core adapters that still own scheduler execution,
//! session restore, terminal pre-warm, remote image conversion, and runtime-port
//! implementations until a reviewed port/provider migration proves equivalence.

pub use sar_dispatch::CoreServiceAgentRuntime;
pub use sar_handler::{
    CoreRemoteDialogRuntimeHost, CoreRemotePollRuntimeHost, CoreRemoteSessionTrackerHost,
    CoreRemoteWorkspaceFileRuntimeHost, CoreRemoteWorkspaceRuntimeHost,
};
pub use sar_lifecycle::CoreRemoteSessionRuntimeHost;
pub use sar_state::{CoreRemoteCancelRuntimeHost, CoreRemoteInteractionRuntimeHost};

#[path = "sar_dispatch.rs"]
mod sar_dispatch;
#[path = "sar_handler.rs"]
mod sar_handler;
#[path = "sar_lifecycle.rs"]
mod sar_lifecycle;
#[path = "sar_state.rs"]
mod sar_state;
#[path = "sar_types.rs"]
mod sar_types;

#[cfg(test)]
mod tests {
    use super::sar_dispatch::CoreServiceAgentRuntime;
    use super::sar_types::{
        agent_input_attachment_from_image_context, core_dialog_submission_policy, normalize_remote_model_selection,
        normalize_remote_session_model_id, remote_chat_messages_from_turns, strip_remote_user_input_tags,
    };
    use crate::agentic::image_analysis::ImageContextData;
    use crate::service::session::{
        DialogTurnData, DialogTurnKind, ModelRoundData, TextItemData, ThinkingItemData, ToolCallData, ToolItemData,
        TurnStatus, UserMessageData,
    };

    #[test]
    fn core_service_agent_runtime_owner_keeps_coordinator_port_contracts() {
        fn assert_runtime_ports<T>()
        where
            T: northhing_runtime_ports::AgentSubmissionPort
                + northhing_runtime_ports::AgentSessionManagementPort
                + northhing_runtime_ports::AgentTurnCancellationPort
                + northhing_runtime_ports::RemoteControlStatePort
                + northhing_runtime_ports::SessionTranscriptReader,
        {
        }

        assert_runtime_ports::<crate::agentic::coordination::ConversationCoordinator>();
    }

    #[test]
    fn core_service_agent_runtime_owner_keeps_scheduler_lifecycle_port_contracts() {
        fn assert_scheduler_ports<T>()
        where
            T: northhing_runtime_ports::AgentDialogTurnPort
                + northhing_runtime_ports::AgentLifecycleDeliveryPort
                + northhing_runtime_ports::AgentTurnCancellationPort,
        {
        }

        assert_scheduler_ports::<crate::agentic::coordination::DialogScheduler>();
    }

    #[test]
    fn core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port() {
        fn assert_agent_runtime(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime(coordinator)
        }

        fn assert_agent_runtime_with_dialog_turns(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_dialog_turns(coordinator, scheduler)
        }

        fn assert_agent_runtime_with_lifecycle_delivery(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_lifecycle_delivery(coordinator, scheduler)
        }

        fn assert_agent_runtime_with_scheduler_ports(
            coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
            scheduler: std::sync::Arc<crate::agentic::coordination::DialogScheduler>,
        ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
            CoreServiceAgentRuntime::agent_runtime_with_scheduler_ports(coordinator, scheduler)
        }

        fn assert_remote_control_port(
            coordinator: &crate::agentic::coordination::ConversationCoordinator,
        ) -> &(dyn northhing_runtime_ports::RemoteControlStatePort + '_) {
            CoreServiceAgentRuntime::remote_control_state_port(coordinator)
        }

        let _ = assert_agent_runtime;
        let _ = assert_agent_runtime_with_dialog_turns;
        let _ = assert_agent_runtime_with_lifecycle_delivery;
        let _ = assert_agent_runtime_with_scheduler_ports;
        let _ = assert_remote_control_port;
    }

    #[test]
    fn core_service_agent_runtime_owner_maps_remote_dialog_policy() {
        let relay = core_dialog_submission_policy(
            northhing_services_integrations::remote_connect::RemoteDialogSubmissionPolicy {
                source: northhing_services_integrations::remote_connect::RemoteConnectSubmissionSource::Relay,
                queue_priority: northhing_services_integrations::remote_connect::RemoteDialogQueuePriority::High,
                skip_tool_confirmation: true,
            },
        );
        assert_eq!(
            relay.trigger_source,
            crate::agentic::coordination::DialogTriggerSource::RemoteRelay
        );
        assert_eq!(
            relay.queue_priority,
            crate::agentic::coordination::DialogQueuePriority::High
        );
        assert!(relay.skip_tool_confirmation);

        let bot = core_dialog_submission_policy(
            northhing_services_integrations::remote_connect::RemoteDialogSubmissionPolicy {
                source: northhing_services_integrations::remote_connect::RemoteConnectSubmissionSource::Bot,
                queue_priority: northhing_services_integrations::remote_connect::RemoteDialogQueuePriority::Low,
                skip_tool_confirmation: false,
            },
        );
        assert_eq!(
            bot.trigger_source,
            crate::agentic::coordination::DialogTriggerSource::Bot
        );
        assert_eq!(
            bot.queue_priority,
            crate::agentic::coordination::DialogQueuePriority::Low
        );
        assert!(!bot.skip_tool_confirmation);
    }

    #[test]
    fn core_service_agent_runtime_owner_maps_image_context_to_lifecycle_attachment() {
        let attachment = agent_input_attachment_from_image_context(ImageContextData {
            id: "ctx-1".to_string(),
            image_path: Some("/workspace/clip.png".to_string()),
            data_url: Some("data:image/png;base64,abc".to_string()),
            mime_type: "image/png".to_string(),
            metadata: Some(serde_json::json!({ "name": "clip.png" })),
        });

        assert_eq!(attachment.kind, "remote_image");
        assert_eq!(attachment.id, "ctx-1");
        assert_eq!(
            attachment.metadata.get("imagePath"),
            Some(&serde_json::json!("/workspace/clip.png"))
        );
        assert_eq!(
            attachment.metadata.get("dataUrl"),
            Some(&serde_json::json!("data:image/png;base64,abc"))
        );
        assert_eq!(
            attachment.metadata.get("mimeType"),
            Some(&serde_json::json!("image/png"))
        );
        assert_eq!(
            attachment.metadata.get("metadata").and_then(|value| value.get("name")),
            Some(&serde_json::json!("clip.png"))
        );
    }

    #[test]
    fn core_service_agent_runtime_owner_normalizes_remote_session_model_ids() {
        assert_eq!(normalize_remote_session_model_id(None), Some("auto".to_string()));
        assert_eq!(
            normalize_remote_session_model_id(Some("".to_string())),
            Some("auto".to_string())
        );
        assert_eq!(
            normalize_remote_session_model_id(Some("  default  ".to_string())),
            Some("auto".to_string())
        );
        assert_eq!(
            normalize_remote_session_model_id(Some(" model-1 ".to_string())),
            Some("model-1".to_string())
        );
    }

    #[test]
    fn core_service_agent_runtime_owner_normalizes_remote_model_selection_aliases() {
        assert_eq!(normalize_remote_model_selection("auto", None).unwrap(), "auto");
        assert_eq!(normalize_remote_model_selection("default", None).unwrap(), "auto");
        assert_eq!(normalize_remote_model_selection("primary", None).unwrap(), "primary");
        assert_eq!(normalize_remote_model_selection("fast", None).unwrap(), "fast");
        assert_eq!(
            normalize_remote_model_selection("   ", None).unwrap_err(),
            "model_id is required"
        );
        assert_eq!(
            normalize_remote_model_selection("custom-alias", None).unwrap_err(),
            "Config service not available"
        );
    }

    #[test]
    fn core_service_agent_runtime_owner_preserves_remote_chat_history_shape() {
        let turn = remote_history_test_turn(
            TurnStatus::Completed,
            Some(serde_json::json!({
                "original_text": "original question",
                "images": [
                    {
                        "name": "screenshot.png",
                        "data_url": "data:image/png;base64,abcd"
                    }
                ]
            })),
        );

        let messages = remote_chat_messages_from_turns(&[turn]);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "original question");
        assert_eq!(messages[0].images.as_ref().unwrap()[0].name, "screenshot.png");

        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "visible text");
        assert_eq!(messages[1].thinking.as_deref(), Some("visible thought"));
        let items = messages[1].items.as_ref().expect("assistant items");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].item_type, "thinking");
        assert_eq!(items[1].item_type, "text");
        assert_eq!(items[2].item_type, "tool");
        assert_eq!(messages[1].tools.as_ref().unwrap()[0].name, "AskUserQuestion");
    }

    #[test]
    fn core_service_agent_runtime_owner_skips_in_progress_remote_assistant_history() {
        let turn = remote_history_test_turn(TurnStatus::InProgress, None);

        let messages = remote_chat_messages_from_turns(&[turn]);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
    }

    #[test]
    fn core_service_agent_runtime_owner_strips_enhanced_remote_user_input() {
        let content = "User uploaded a file.\nUser's question:\n  explain this  ";

        assert_eq!(strip_remote_user_input_tags(content), "explain this");
    }

    fn remote_history_test_turn(status: TurnStatus, metadata: Option<serde_json::Value>) -> DialogTurnData {
        DialogTurnData {
            turn_id: "turn-1".to_string(),
            turn_index: 0,
            session_id: "session-1".to_string(),
            timestamp: 1_000,
            kind: DialogTurnKind::UserDialog,
            agent_type: None,
            user_message: UserMessageData {
                id: "user-1".to_string(),
                content: "fallback text".to_string(),
                timestamp: 1_000,
                metadata,
            },
            model_rounds: vec![ModelRoundData {
                id: "round-1".to_string(),
                turn_id: "turn-1".to_string(),
                round_index: 0,
                timestamp: 1_100,
                text_items: vec![
                    TextItemData {
                        id: "text-hidden".to_string(),
                        content: "hidden text".to_string(),
                        is_streaming: false,
                        timestamp: 1_111,
                        is_markdown: true,
                        order_index: Some(1),
                        is_subagent_item: Some(true),
                        parent_task_tool_id: None,
                        subagent_session_id: None,
                        status: None,
                    },
                    TextItemData {
                        id: "text-1".to_string(),
                        content: "visible text".to_string(),
                        is_streaming: false,
                        timestamp: 1_112,
                        is_markdown: true,
                        order_index: Some(1),
                        is_subagent_item: None,
                        parent_task_tool_id: None,
                        subagent_session_id: None,
                        status: None,
                    },
                ],
                tool_items: vec![ToolItemData {
                    id: "tool-1".to_string(),
                    tool_name: "AskUserQuestion".to_string(),
                    tool_call: ToolCallData {
                        input: serde_json::json!({ "question": "confirm?" }),
                        id: "call-1".to_string(),
                    },
                    tool_result: None,
                    ai_intent: None,
                    start_time: 1_130,
                    end_time: None,
                    duration_ms: Some(25),
                    queue_wait_ms: None,
                    preflight_ms: None,
                    confirmation_wait_ms: None,
                    execution_ms: None,
                    order_index: Some(2),
                    is_subagent_item: None,
                    parent_task_tool_id: None,
                    subagent_session_id: None,
                    subagent_model_id: None,
                    subagent_model_alias: None,
                    status: Some("running".to_string()),
                    interruption_reason: None,
                }],
                thinking_items: vec![ThinkingItemData {
                    id: "thinking-1".to_string(),
                    content: "visible thought".to_string(),
                    is_streaming: false,
                    is_collapsed: false,
                    timestamp: 1_105,
                    order_index: Some(0),
                    status: None,
                    is_subagent_item: None,
                    parent_task_tool_id: None,
                    subagent_session_id: None,
                }],
                start_time: 1_100,
                end_time: Some(1_200),
                duration_ms: Some(100),
                provider_id: None,
                model_id: None,
                model_alias: None,
                first_chunk_ms: None,
                first_visible_output_ms: None,
                stream_duration_ms: None,
                attempt_count: None,
                failure_category: None,
                token_details: None,
                status: "completed".to_string(),
            }],
            start_time: 1_000,
            end_time: Some(1_250),
            duration_ms: Some(250),
            token_usage: None,
            status,
        }
    }
}
