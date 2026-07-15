//! Contract tests for agent-domain re-exports on the runtime-ports facade.
//!
//! R39d sibling: split facade-test bulk from lib.rs (agent domain).

use crate::*;

#[test]
fn agent_submission_request_serializes_with_stable_camel_case() {
    let request = AgentSubmissionRequest {
        session_id: "session_1".to_string(),
        message: "hello".to_string(),
        turn_id: None,
        source: None,
        attachments: Vec::new(),
        metadata: serde_json::Map::new(),
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["message"], "hello");
    assert!(json.get("source").is_none());
    assert!(json.get("attachments").is_none());
}

#[test]
fn agent_submission_request_serializes_source_without_changing_field_case() {
    let request = AgentSubmissionRequest {
        session_id: "session_1".to_string(),
        message: "hello".to_string(),
        turn_id: None,
        source: Some(AgentSubmissionSource::RemoteRelay),
        attachments: Vec::new(),
        metadata: serde_json::Map::new(),
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json["source"], "remote_relay");
    assert!(json.get("turnId").is_none());
}

#[test]
fn dialog_trigger_source_reuses_agent_submission_source_contract() {
    let json = serde_json::to_value(DialogTriggerSource::Cli).expect("serialize dialog trigger source");

    assert_eq!(json, serde_json::json!("cli"));
}

#[test]
fn dialog_submission_policy_preserves_current_surface_queue_defaults() {
    let remote = DialogSubmissionPolicy::for_source(DialogTriggerSource::RemoteRelay);
    assert_eq!(remote.queue_priority, DialogQueuePriority::Normal);
    assert!(remote.skip_tool_confirmation);

    let bot = DialogSubmissionPolicy::for_source(DialogTriggerSource::Bot);
    assert_eq!(bot.queue_priority, DialogQueuePriority::Normal);
    assert!(bot.skip_tool_confirmation);

    let agent_session = DialogSubmissionPolicy::for_source(DialogTriggerSource::AgentSession);
    assert_eq!(agent_session.queue_priority, DialogQueuePriority::Low);
    assert!(agent_session.skip_tool_confirmation);

    let cli = DialogSubmissionPolicy::for_source(DialogTriggerSource::Cli);
    assert_eq!(cli.queue_priority, DialogQueuePriority::Normal);
    assert!(!cli.skip_tool_confirmation);
}

#[test]
fn dialog_submit_outcome_preserves_started_and_queued_fields() {
    let started = DialogSubmitOutcome::Started {
        session_id: "session_1".to_string(),
        turn_id: "turn_1".to_string(),
    };
    let queued = DialogSubmitOutcome::Queued {
        session_id: "session_1".to_string(),
        turn_id: "turn_2".to_string(),
    };

    assert_eq!(
        started,
        DialogSubmitOutcome::Started {
            session_id: "session_1".to_string(),
            turn_id: "turn_1".to_string(),
        }
    );
    assert_ne!(started, queued);
}

#[test]
fn dialog_submit_queue_action_preserves_current_scheduler_routing_policy() {
    let remote = DialogSubmissionPolicy::for_source(DialogTriggerSource::RemoteRelay);

    let agent_session = DialogSubmissionPolicy::for_source(DialogTriggerSource::AgentSession);

    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Missing,
            queue_has_items: true,
            policy: remote,
        }),
        DialogSubmitQueueAction::StartImmediately
    );
    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Error,
            queue_has_items: true,
            policy: remote,
        }),
        DialogSubmitQueueAction::ClearQueueAndStartImmediately
    );
    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Idle,
            queue_has_items: false,
            policy: remote,
        }),
        DialogSubmitQueueAction::StartImmediately
    );
    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Idle,
            queue_has_items: true,
            policy: remote,
        }),
        DialogSubmitQueueAction::EnqueueThenStartNext
    );
    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Processing,
            queue_has_items: false,
            policy: remote,
        }),
        DialogSubmitQueueAction::EnqueueForActiveTurn
    );
    assert_eq!(
        resolve_dialog_submit_queue_action(DialogSubmitQueueFacts {
            session_state: DialogSessionStateFact::Processing,
            queue_has_items: false,
            policy: agent_session,
        }),
        DialogSubmitQueueAction::EnqueueForActiveTurn
    );
}

#[test]
fn agent_session_reply_decisions_preserve_cancel_suppression_boundary() {
    let policy = DialogSubmissionPolicy::for_source(DialogTriggerSource::AgentSession);
    assert!(should_suppress_agent_session_cancelled_reply(
        &policy,
        Some("requester"),
        "requester",
    ));
    assert!(!should_suppress_agent_session_cancelled_reply(
        &policy,
        Some("requester"),
        "other",
    ));

    let remote = DialogSubmissionPolicy::for_source(DialogTriggerSource::RemoteRelay);
    assert!(!should_suppress_agent_session_cancelled_reply(
        &remote,
        Some("requester"),
        "requester",
    ));

    assert!(should_skip_agent_session_reply(DialogTurnOutcomeKind::Cancelled, true,));
    assert!(!should_skip_agent_session_reply(
        DialogTurnOutcomeKind::Cancelled,
        false,
    ));
    assert!(!should_skip_agent_session_reply(DialogTurnOutcomeKind::Completed, true,));
    assert!(!should_skip_agent_session_reply(DialogTurnOutcomeKind::Failed, true,));
}

#[test]
fn agent_session_reply_route_keeps_requester_fields() {
    let route = AgentSessionReplyRoute {
        source_session_id: "requester_session".to_string(),
        source_workspace_path: "/workspace/requester".to_string(),
    };

    assert_eq!(route.source_session_id, "requester_session");
    assert_eq!(route.source_workspace_path, "/workspace/requester");
}

#[test]
fn dialog_steer_outcome_preserves_buffered_fields() {
    let outcome = DialogSteerOutcome::Buffered {
        session_id: "session_1".to_string(),
        turn_id: "turn_1".to_string(),
        steering_id: "steer_1".to_string(),
    };

    assert_eq!(
        outcome,
        DialogSteerOutcome::Buffered {
            session_id: "session_1".to_string(),
            turn_id: "turn_1".to_string(),
            steering_id: "steer_1".to_string(),
        }
    );
}

#[test]
fn round_injection_contract_keeps_kind_and_target_identity() {
    assert_eq!(RoundInjectionKind::UserSteering, RoundInjectionKind::UserSteering);
    assert_ne!(RoundInjectionKind::UserSteering, RoundInjectionKind::BackgroundResult);

    let target = RoundInjectionTarget::ExactTurn("turn_1".to_string());
    assert_eq!(target, RoundInjectionTarget::ExactTurn("turn_1".to_string()));
    assert_ne!(target, RoundInjectionTarget::CurrentRunningTurn);
}

#[test]
fn round_injection_source_contract_drains_portable_injections() {
    struct StaticInjectionSource {
        injection: RoundInjection,
    }

    impl DialogRoundInjectionSource for StaticInjectionSource {
        fn has_pending(&self, session_id: &str, turn_id: &str) -> bool {
            session_id == "session_1" && turn_id == "turn_1"
        }

        fn take_pending(&self, session_id: &str, turn_id: &str) -> Vec<RoundInjection> {
            if self.has_pending(session_id, turn_id) {
                vec![self.injection.clone()]
            } else {
                Vec::new()
            }
        }
    }

    let source = StaticInjectionSource {
        injection: RoundInjection {
            id: "injection_1".to_string(),
            kind: RoundInjectionKind::BackgroundResult,
            target: RoundInjectionTarget::CurrentRunningTurn,
            content: "result".to_string(),
            display_content: "result".to_string(),
            created_at: std::time::SystemTime::UNIX_EPOCH,
        },
    };

    assert!(source.has_pending("session_1", "turn_1"));
    assert!(!source.has_pending("session_2", "turn_1"));
    let drained = source.take_pending("session_1", "turn_1");
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].id, "injection_1");
    assert_eq!(drained[0].kind, RoundInjectionKind::BackgroundResult);
}

#[test]
fn thread_goal_active_status_includes_budget_limited() {
    let active = ThreadGoal {
        goal_id: "goal_1".to_string(),
        session_id: "session_1".to_string(),
        objective: "Ship feature".to_string(),
        status: ThreadGoalStatus::Active,
        token_budget: Some(10_000),
        tokens_used: 100,
        time_used_seconds: 5,
        created_at: 1,
        updated_at: 2,
        auto_continuation_count: 0,
    };
    assert!(active.is_active());
    assert_eq!(active.remaining_tokens(), Some(9_900));

    let budget_limited = ThreadGoal {
        status: ThreadGoalStatus::BudgetLimited,
        ..active.clone()
    };
    assert!(budget_limited.is_active());

    let paused = ThreadGoal {
        status: ThreadGoalStatus::Paused,
        ..active
    };
    assert!(!paused.is_active());
}

#[test]
fn thread_goal_tool_response_serializes_optional_fields() {
    let response = ThreadGoalToolResponse {
        goal: None,
        remaining_tokens: Some(42),
        completion_budget_report: None,
    };
    let json = serde_json::to_value(response).expect("serialize thread goal tool response");
    assert!(json.get("goal").is_none());
    assert_eq!(json["remainingTokens"], 42);
    assert!(json.get("completionBudgetReport").is_none());
}

#[test]
fn agent_submission_request_serializes_explicit_turn_id_contract() {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "turnId".to_string(),
        serde_json::Value::String("legacy_metadata_turn".to_string()),
    );
    let request = AgentSubmissionRequest {
        session_id: "session_1".to_string(),
        message: "hello".to_string(),
        turn_id: Some("explicit_turn".to_string()),
        source: Some(AgentSubmissionSource::RemoteRelay),
        attachments: Vec::new(),
        metadata,
    };

    let json = serde_json::to_value(request).expect("serialize request");

    assert_eq!(json["turnId"], "explicit_turn");
    assert_eq!(json["metadata"]["turnId"], "legacy_metadata_turn");
}

#[test]
fn remote_image_attachment_serializes_portable_metadata_contract() {
    let attachment = AgentInputAttachment::remote_image("image-1", "clip.png", "data:image/png;base64,abc");

    let json = serde_json::to_value(attachment).expect("serialize attachment");

    assert_eq!(json["kind"], "remote_image");
    assert_eq!(json["id"], "image-1");
    assert_eq!(json["metadata"]["name"], "clip.png");
    assert_eq!(json["metadata"]["dataUrl"], "data:image/png;base64,abc");
}

#[test]
fn agent_dialog_turn_request_serializes_lifecycle_contract() {
    let request = AgentDialogTurnRequest {
        session_id: "session_1".to_string(),
        message: "hello".to_string(),
        original_message: Some("raw hello".to_string()),
        turn_id: Some("turn_1".to_string()),
        agent_type: "agentic".to_string(),
        workspace_path: Some("/workspace/project".to_string()),
        policy: DialogSubmissionPolicy::new(AgentSubmissionSource::RemoteRelay, DialogQueuePriority::High, true),
        reply_route: Some(AgentSessionReplyRoute {
            source_session_id: "source_session".to_string(),
            source_workspace_path: "/workspace/source".to_string(),
        }),
        prepended_reminders: vec![AgentDialogPrependedReminder {
            kind: "session_message_request".to_string(),
            text: "sent by another agent".to_string(),
        }],
        attachments: vec![AgentInputAttachment::remote_image(
            "image-1",
            "clip.png",
            "data:image/png;base64,abc",
        )],
        metadata: serde_json::Map::new(),
    };

    let json = serde_json::to_value(request).expect("serialize dialog turn request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["message"], "hello");
    assert_eq!(json["originalMessage"], "raw hello");
    assert_eq!(json["turnId"], "turn_1");
    assert_eq!(json["agentType"], "agentic");
    assert_eq!(json["workspacePath"], "/workspace/project");
    assert_eq!(json["policy"]["triggerSource"], "remote_relay");
    assert_eq!(json["policy"]["queuePriority"], "high");
    assert_eq!(json["policy"]["skipToolConfirmation"], true);
    assert_eq!(json["replyRoute"]["sourceSessionId"], "source_session");
    assert_eq!(json["prependedReminders"][0]["kind"], "session_message_request");
    assert_eq!(json["attachments"][0]["kind"], "remote_image");
}

#[test]
fn agent_background_result_request_serializes_lifecycle_contract() {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "kind".to_string(),
        serde_json::Value::String("background_result".to_string()),
    );
    let request = AgentBackgroundResultRequest {
        session_id: "session_1".to_string(),
        agent_type: "agentic".to_string(),
        workspace_path: Some("/workspace/project".to_string()),
        content: "full result".to_string(),
        display_content: Some("short result".to_string()),
        metadata,
    };

    let json = serde_json::to_value(request).expect("serialize background result request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["agentType"], "agentic");
    assert_eq!(json["workspacePath"], "/workspace/project");
    assert_eq!(json["content"], "full result");
    assert_eq!(json["displayContent"], "short result");
    assert_eq!(json["metadata"]["kind"], "background_result");
}

#[test]
fn agent_thread_goal_delivery_request_serializes_lifecycle_contract() {
    let request = AgentThreadGoalDeliveryRequest {
        session_id: "session_1".to_string(),
        agent_type: "agentic".to_string(),
        workspace_path: Some("/workspace/project".to_string()),
        kind: AgentThreadGoalDeliveryKind::ObjectiveUpdated,
        goal: ThreadGoal {
            goal_id: "goal_1".to_string(),
            session_id: "session_1".to_string(),
            objective: "Ship the refactor".to_string(),
            status: ThreadGoalStatus::Active,
            token_budget: Some(1000),
            tokens_used: 10,
            time_used_seconds: 0,
            created_at: 1,
            updated_at: 2,
            auto_continuation_count: 0,
        },
    };

    let json = serde_json::to_value(request).expect("serialize thread goal delivery request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["agentType"], "agentic");
    assert_eq!(json["workspacePath"], "/workspace/project");
    assert_eq!(json["kind"], "objective_updated");
    assert_eq!(json["goal"]["goalId"], "goal_1");
}

#[test]
fn agent_turn_cancellation_request_serializes_current_contract() {
    let request = AgentTurnCancellationRequest {
        session_id: "session_1".to_string(),
        turn_id: Some("turn_1".to_string()),
        source: Some(AgentSubmissionSource::Bot),
        requester_session_id: Some("requester_session".to_string()),
        reason: Some("user_cancelled".to_string()),
        wait_timeout_ms: Some(1500),
    };

    let json = serde_json::to_value(request).expect("serialize cancel request");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["turnId"], "turn_1");
    assert_eq!(json["source"], "bot");
    assert_eq!(json["requesterSessionId"], "requester_session");
    assert_eq!(json["reason"], "user_cancelled");
    assert_eq!(json["waitTimeoutMs"], 1500);
}

#[test]
fn agent_session_management_contracts_serialize_stable_shape() {
    let list_request = AgentSessionListRequest {
        workspace_path: "/workspace/project".to_string(),
    };
    let summary = AgentSessionSummary {
        session_id: "session_1".to_string(),
        session_name: "Main".to_string(),
        agent_type: "agentic".to_string(),
        created_at_ms: 1000,
        last_active_at_ms: 2000,
    };
    let delete_request = AgentSessionDeleteRequest {
        workspace_path: "/workspace/project".to_string(),
        session_id: "session_1".to_string(),
    };
    let workspace_request = AgentSessionWorkspaceRequest {
        session_id: "session_1".to_string(),
    };

    let list_json = serde_json::to_value(list_request).expect("serialize list request");
    let summary_json = serde_json::to_value(summary).expect("serialize summary");
    let delete_json = serde_json::to_value(delete_request).expect("serialize delete request");
    let workspace_json = serde_json::to_value(workspace_request).expect("serialize workspace request");

    assert_eq!(list_json["workspacePath"], "/workspace/project");
    assert_eq!(summary_json["sessionId"], "session_1");
    assert_eq!(summary_json["createdAtMs"], 1000);
    assert_eq!(summary_json["lastActiveAtMs"], 2000);
    assert_eq!(delete_json["sessionId"], "session_1");
    assert_eq!(workspace_json["sessionId"], "session_1");
}

#[test]
fn runtime_event_envelope_serializes_observational_surface_facts() {
    let event = RuntimeEventEnvelope {
        session_id: "session_1".to_string(),
        turn_id: Some("turn_1".to_string()),
        source: Some(AgentSubmissionSource::RemoteRelay),
        event_type: RuntimeEventType::TurnCancelled,
        payload: serde_json::json!({ "reason": "user_cancelled" }),
    };

    let json = serde_json::to_value(event).expect("serialize event");

    assert_eq!(json["sessionId"], "session_1");
    assert_eq!(json["turnId"], "turn_1");
    assert_eq!(json["source"], "remote_relay");
    assert_eq!(json["eventType"], "turn_cancelled");
    assert_eq!(json["payload"]["reason"], "user_cancelled");
}

#[test]
fn dynamic_tool_descriptor_serializes_current_wire_shape() {
    let descriptor = DynamicToolDescriptor {
        name: "external_search".to_string(),
        description: "Search external docs".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        provider_id: Some("provider-a".to_string()),
    };

    let json = serde_json::to_value(descriptor).expect("serialize descriptor");

    assert_eq!(json["name"], "external_search");
    assert_eq!(json["description"], "Search external docs");
    assert_eq!(json["inputSchema"]["type"], "object");
    assert_eq!(json["providerId"], "provider-a");
    assert!(json.get("provider_id").is_none());
}
#[test]
fn subagent_context_mode_preserves_fork_wire_value() {
    assert_eq!(SubagentContextMode::default(), SubagentContextMode::Fresh);
    assert_eq!(SubagentContextMode::Fresh.as_str(), "fresh");
    assert_eq!(SubagentContextMode::Fork.as_str(), "fork");

    let json = serde_json::to_value(SubagentContextMode::Fork).expect("serialize subagent context mode");

    assert_eq!(json, serde_json::json!("fork"));
}

#[test]
fn delegation_policy_child_blocks_recursive_spawn_without_losing_depth() {
    let top_level = DelegationPolicy::top_level();
    assert!(top_level.allow_subagent_spawn);
    assert_eq!(top_level.nesting_depth, 0);

    let child = top_level.spawn_child();

    assert!(!child.allow_subagent_spawn);
    assert_eq!(child.nesting_depth, 1);
    assert_eq!(child.spawn_child().nesting_depth, 2);
}

#[test]
fn dynamic_tool_descriptor_omits_missing_provider_id() {
    let descriptor = DynamicToolDescriptor {
        name: "local_tool".to_string(),
        description: "Local tool".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        provider_id: None,
    };

    let json = serde_json::to_value(descriptor).expect("serialize descriptor");

    assert!(json.get("providerId").is_none());
}
