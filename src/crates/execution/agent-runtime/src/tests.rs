//! Tests for the port-backed [`crate::runtime::AgentRuntime`] facade.
//!
//! Split from `runtime.rs` (R39e). Test helpers are kept in this sibling file
//! so the facade stays focused on the public method surface.

use std::sync::{Arc, Mutex};

use northhing_runtime_ports::{
    AgentBackgroundResultRequest, AgentDialogTurnRequest, AgentInputAttachment, AgentLifecycleDeliveryPort,
    AgentSessionCreateResult, AgentSessionDeleteRequest, AgentSessionListRequest, AgentSessionManagementPort,
    AgentSessionSummary, AgentSessionWorkspaceRequest, AgentSubmissionResult, AgentSubmissionSource,
    AgentThreadGoalDeliveryKind, AgentThreadGoalDeliveryRequest, AgentTurnCancellationResult, ClockPort,
    DialogQueuePriority, DialogSubmissionPolicy, DialogSubmitOutcome, FileSystemPort, PermissionPort, PortError,
    PortErrorKind, PortResult, RuntimeEventSink, RuntimeEventType, RuntimeServiceCapability, SessionStorePort,
    ThreadGoal, ThreadGoalStatus, WorkspacePort,
};
use northhing_runtime_services::{test_support::FakeRuntimePort, RuntimeServicesBuilder};

use super::*;

#[derive(Debug, Default)]
struct FakeAgentRuntimePorts {
    created_sessions: Mutex<Vec<AgentSessionCreateRequest>>,
    submitted_messages: Mutex<Vec<AgentSubmissionRequest>>,
    cancelled_turns: Mutex<Vec<AgentTurnCancellationRequest>>,
    listed_sessions: Mutex<Vec<AgentSessionListRequest>>,
    deleted_sessions: Mutex<Vec<AgentSessionDeleteRequest>>,
    workspace_requests: Mutex<Vec<AgentSessionWorkspaceRequest>>,
    resolved_agent_type: Option<String>,
}

#[async_trait::async_trait]
impl AgentSessionManagementPort for FakeAgentRuntimePorts {
    async fn list_sessions(&self, request: AgentSessionListRequest) -> PortResult<Vec<AgentSessionSummary>> {
        self.listed_sessions.lock().unwrap().push(request.clone());
        Ok(vec![AgentSessionSummary {
            session_id: "session_1".to_string(),
            session_name: "Main".to_string(),
            agent_type: "agentic".to_string(),
            created_at_ms: 1000,
            last_active_at_ms: 2000,
        }])
    }

    async fn delete_session(&self, request: AgentSessionDeleteRequest) -> PortResult<()> {
        self.deleted_sessions.lock().unwrap().push(request);
        Ok(())
    }

    async fn resolve_session_workspace_path(
        &self,
        request: AgentSessionWorkspaceRequest,
    ) -> PortResult<Option<String>> {
        self.workspace_requests.lock().unwrap().push(request);
        Ok(Some("/workspace/project".to_string()))
    }
}

#[async_trait::async_trait]
impl AgentSubmissionPort for FakeAgentRuntimePorts {
    async fn create_session(&self, request: AgentSessionCreateRequest) -> PortResult<AgentSessionCreateResult> {
        self.created_sessions.lock().unwrap().push(request.clone());
        Ok(AgentSessionCreateResult {
            session_id: "session_1".to_string(),
            session_name: request.session_name,
            agent_type: request.agent_type,
        })
    }

    async fn submit_message(&self, request: AgentSubmissionRequest) -> PortResult<AgentSubmissionResult> {
        self.submitted_messages.lock().unwrap().push(request.clone());
        Ok(AgentSubmissionResult {
            turn_id: request.turn_id.unwrap_or_else(|| "generated_turn".to_string()),
            accepted: true,
        })
    }

    async fn resolve_session_agent_type(&self, _session_id: &str) -> PortResult<Option<String>> {
        Ok(self.resolved_agent_type.clone())
    }
}

#[async_trait::async_trait]
impl AgentTurnCancellationPort for FakeAgentRuntimePorts {
    async fn cancel_turn(&self, request: AgentTurnCancellationRequest) -> PortResult<AgentTurnCancellationResult> {
        self.cancelled_turns.lock().unwrap().push(request.clone());
        Ok(AgentTurnCancellationResult {
            session_id: request.session_id,
            turn_id: request.turn_id,
            requested: true,
        })
    }
}

#[derive(Debug, Default)]
struct RecordingRuntimeEventSink {
    events: Mutex<Vec<RuntimeEventEnvelope>>,
}

impl RecordingRuntimeEventSink {
    fn events(&self) -> Vec<RuntimeEventEnvelope> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl RuntimeEventSink for RecordingRuntimeEventSink {
    async fn publish_runtime_event(&self, event: RuntimeEventEnvelope) -> PortResult<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

fn runtime_services_with_events(events: Arc<dyn RuntimeEventSink>) -> RuntimeServices {
    let filesystem: Arc<dyn FileSystemPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::FileSystem));
    let workspace: Arc<dyn WorkspacePort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Workspace));
    let session_store: Arc<dyn SessionStorePort> =
        Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::SessionStore));
    let permission: Arc<dyn PermissionPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Permission));
    let clock: Arc<dyn ClockPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Clock));

    RuntimeServicesBuilder::new()
        .with_filesystem(filesystem)
        .with_workspace(workspace)
        .with_session_store(session_store)
        .with_permission(permission)
        .with_events(events)
        .with_clock(clock)
        .build()
        .expect("runtime services")
}

#[tokio::test]
async fn builder_requires_submission_port() {
    let err = AgentRuntimeBuilder::new().build().unwrap_err();
    assert_eq!(err, RuntimeBuildError::MissingSubmissionPort);
}

#[tokio::test]
async fn run_creates_session_and_submits_turn_through_ports() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports.clone())
        .build()
        .expect("runtime");

    let mut metadata = serde_json::Map::new();
    metadata.insert("source".to_string(), serde_json::json!("sdk-test"));

    let handle = runtime
        .run(
            AgentRunRequest::new(
                SessionSelector::create("SDK Session", "agentic", Some("/workspace/project".to_string()))
                    .with_metadata(metadata.clone()),
                "hello",
            )
            .with_turn_id("turn_1")
            .with_source(AgentSubmissionSource::Cli),
        )
        .await
        .expect("run");

    assert_eq!(handle.session_id, "session_1");
    assert_eq!(handle.turn_id, "turn_1");
    assert_eq!(handle.agent_type.as_deref(), Some("agentic"));
    assert!(handle.accepted);
    assert_eq!(ports.created_sessions.lock().unwrap()[0].metadata, metadata);
    assert_eq!(ports.submitted_messages.lock().unwrap()[0].session_id, "session_1");
    assert!(handle.events.is_none());
}

#[tokio::test]
async fn run_existing_session_resolves_agent_type_without_creating_session() {
    let ports = Arc::new(FakeAgentRuntimePorts {
        resolved_agent_type: Some("Claw".to_string()),
        ..Default::default()
    });
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports.clone())
        .build()
        .expect("runtime");

    let handle = runtime
        .run(AgentRunRequest::new(
            SessionSelector::existing("session_existing"),
            "continue",
        ))
        .await
        .expect("run existing session");

    assert_eq!(handle.session_id, "session_existing");
    assert_eq!(handle.agent_type.as_deref(), Some("Claw"));
    assert!(ports.created_sessions.lock().unwrap().is_empty());
    assert_eq!(
        ports.submitted_messages.lock().unwrap()[0].session_id,
        "session_existing"
    );
}

#[tokio::test]
async fn cancel_turn_requires_registered_cancellation_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .build()
        .expect("runtime");

    let err = runtime
        .cancel_turn(AgentTurnCancellationRequest {
            session_id: "session_1".to_string(),
            turn_id: Some("turn_1".to_string()),
            source: None,
            requester_session_id: None,
            reason: None,
            wait_timeout_ms: None,
        })
        .await
        .unwrap_err();

    assert_eq!(err, RuntimeError::MissingCancellationPort);
}

#[tokio::test]
async fn cancel_turn_delegates_to_cancellation_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports.clone())
        .with_cancellation_port(ports.clone())
        .build()
        .expect("runtime");

    let result = runtime
        .cancel_turn(AgentTurnCancellationRequest {
            session_id: "session_1".to_string(),
            turn_id: Some("turn_1".to_string()),
            source: Some(AgentSubmissionSource::RemoteRelay),
            requester_session_id: Some("requester_session".to_string()),
            reason: Some("user_cancelled".to_string()),
            wait_timeout_ms: Some(100),
        })
        .await
        .expect("cancel");

    assert!(result.requested);
    assert_eq!(result.turn_id.as_deref(), Some("turn_1"));
    assert_eq!(ports.cancelled_turns.lock().unwrap().len(), 1);
    assert_eq!(
        ports.cancelled_turns.lock().unwrap()[0].requester_session_id.as_deref(),
        Some("requester_session")
    );
}

#[tokio::test]
async fn session_management_requires_registered_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .build()
        .expect("runtime");

    let err = runtime
        .list_sessions(AgentSessionListRequest {
            workspace_path: "/workspace/project".to_string(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, RuntimeError::MissingSessionManagementPort);
}

#[tokio::test]
async fn session_management_delegates_to_registered_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports.clone())
        .with_session_management_port(ports.clone())
        .build()
        .expect("runtime");

    let sessions = runtime
        .list_sessions(AgentSessionListRequest {
            workspace_path: "/workspace/project".to_string(),
        })
        .await
        .expect("list sessions");
    runtime
        .delete_session(AgentSessionDeleteRequest {
            workspace_path: "/workspace/project".to_string(),
            session_id: "session_1".to_string(),
        })
        .await
        .expect("delete session");
    let workspace_path = runtime
        .resolve_session_workspace_path(AgentSessionWorkspaceRequest {
            session_id: "session_1".to_string(),
        })
        .await
        .expect("resolve workspace");

    assert_eq!(sessions[0].session_id, "session_1");
    assert_eq!(workspace_path.as_deref(), Some("/workspace/project"));
    assert_eq!(ports.listed_sessions.lock().unwrap().len(), 1);
    assert_eq!(ports.deleted_sessions.lock().unwrap().len(), 1);
    assert_eq!(ports.workspace_requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn submit_dialog_turn_requires_registered_dialog_turn_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .build()
        .expect("runtime");

    let err = runtime
        .submit_dialog_turn(AgentDialogTurnRequest {
            session_id: "session_1".to_string(),
            message: "hello".to_string(),
            original_message: None,
            turn_id: Some("turn_1".to_string()),
            agent_type: "agentic".to_string(),
            workspace_path: Some("/workspace/project".to_string()),
            policy: DialogSubmissionPolicy::new(AgentSubmissionSource::RemoteRelay, DialogQueuePriority::Normal, true),
            reply_route: None,
            prepended_reminders: Vec::new(),
            attachments: Vec::new(),
            metadata: serde_json::Map::new(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, RuntimeError::MissingDialogTurnPort);
}

#[tokio::test]
async fn submit_dialog_turn_delegates_to_dialog_turn_port() {
    #[derive(Debug, Default)]
    struct RecordingDialogTurnPort {
        requests: Mutex<Vec<AgentDialogTurnRequest>>,
    }

    #[async_trait::async_trait]
    impl northhing_runtime_ports::AgentDialogTurnPort for RecordingDialogTurnPort {
        async fn submit_dialog_turn(&self, request: AgentDialogTurnRequest) -> PortResult<DialogSubmitOutcome> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(DialogSubmitOutcome::Queued {
                session_id: request.session_id,
                turn_id: request.turn_id.unwrap_or_else(|| "generated".to_string()),
            })
        }
    }

    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let dialog_turns = Arc::new(RecordingDialogTurnPort::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .with_dialog_turn_port(dialog_turns.clone())
        .build()
        .expect("runtime");

    let result = runtime
        .submit_dialog_turn(AgentDialogTurnRequest {
            session_id: "session_1".to_string(),
            message: "hello".to_string(),
            original_message: Some("hello".to_string()),
            turn_id: Some("turn_1".to_string()),
            agent_type: "agentic".to_string(),
            workspace_path: Some("/workspace/project".to_string()),
            policy: DialogSubmissionPolicy::new(AgentSubmissionSource::RemoteRelay, DialogQueuePriority::High, true),
            reply_route: None,
            prepended_reminders: Vec::new(),
            attachments: vec![AgentInputAttachment::remote_image(
                "remote-image-1",
                "clip.png",
                "data:image/png;base64,abc",
            )],
            metadata: serde_json::Map::new(),
        })
        .await
        .expect("dialog turn");

    assert_eq!(
        result,
        DialogSubmitOutcome::Queued {
            session_id: "session_1".to_string(),
            turn_id: "turn_1".to_string(),
        }
    );
    assert_eq!(dialog_turns.requests.lock().unwrap().len(), 1);
    assert_eq!(
        dialog_turns.requests.lock().unwrap()[0].policy.queue_priority,
        DialogQueuePriority::High
    );
    assert_eq!(
        dialog_turns.requests.lock().unwrap()[0].attachments[0].kind,
        "remote_image"
    );
}

#[tokio::test]
async fn deliver_background_result_requires_registered_lifecycle_port() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .build()
        .expect("runtime");

    let err = runtime
        .deliver_background_result(AgentBackgroundResultRequest {
            session_id: "session_1".to_string(),
            agent_type: "agentic".to_string(),
            workspace_path: None,
            content: "result".to_string(),
            display_content: None,
            metadata: serde_json::Map::new(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, RuntimeError::MissingLifecycleDeliveryPort);
}

#[tokio::test]
async fn lifecycle_delivery_delegates_to_registered_port() {
    #[derive(Debug, Default)]
    struct RecordingLifecycleDeliveryPort {
        background_results: Mutex<Vec<AgentBackgroundResultRequest>>,
        thread_goals: Mutex<Vec<AgentThreadGoalDeliveryRequest>>,
    }

    #[async_trait::async_trait]
    impl AgentLifecycleDeliveryPort for RecordingLifecycleDeliveryPort {
        async fn deliver_background_result(&self, request: AgentBackgroundResultRequest) -> PortResult<()> {
            self.background_results.lock().unwrap().push(request);
            Ok(())
        }

        async fn deliver_thread_goal(&self, request: AgentThreadGoalDeliveryRequest) -> PortResult<()> {
            self.thread_goals.lock().unwrap().push(request);
            Ok(())
        }
    }

    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let lifecycle = Arc::new(RecordingLifecycleDeliveryPort::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .with_lifecycle_delivery_port(lifecycle.clone())
        .build()
        .expect("runtime");

    runtime
        .deliver_background_result(AgentBackgroundResultRequest {
            session_id: "session_1".to_string(),
            agent_type: "agentic".to_string(),
            workspace_path: Some("/workspace/project".to_string()),
            content: "result".to_string(),
            display_content: Some("display".to_string()),
            metadata: serde_json::Map::new(),
        })
        .await
        .expect("background result");

    runtime
        .deliver_thread_goal(AgentThreadGoalDeliveryRequest {
            session_id: "session_1".to_string(),
            agent_type: "agentic".to_string(),
            workspace_path: Some("/workspace/project".to_string()),
            kind: AgentThreadGoalDeliveryKind::Resumed,
            goal: ThreadGoal {
                goal_id: "goal_1".to_string(),
                session_id: "session_1".to_string(),
                objective: "Ship the refactor".to_string(),
                status: ThreadGoalStatus::Active,
                token_budget: None,
                tokens_used: 0,
                time_used_seconds: 0,
                created_at: 1,
                updated_at: 2,
                auto_continuation_count: 0,
            },
        })
        .await
        .expect("thread goal delivery");

    assert_eq!(lifecycle.background_results.lock().unwrap().len(), 1);
    assert_eq!(
        lifecycle.background_results.lock().unwrap()[0]
            .display_content
            .as_deref(),
        Some("display")
    );
    assert_eq!(lifecycle.thread_goals.lock().unwrap().len(), 1);
    assert_eq!(
        lifecycle.thread_goals.lock().unwrap()[0].kind,
        AgentThreadGoalDeliveryKind::Resumed
    );
}

#[tokio::test]
async fn publish_event_requires_registered_runtime_services() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .build()
        .expect("runtime");

    let err = runtime
        .publish_event(RuntimeEventEnvelope {
            session_id: "session_1".to_string(),
            turn_id: Some("turn_1".to_string()),
            source: Some(AgentSubmissionSource::Cli),
            event_type: RuntimeEventType::TurnStarted,
            payload: serde_json::json!({ "phase": "submitted" }),
        })
        .await
        .unwrap_err();

    assert_eq!(err, RuntimeError::MissingEventSink);
}

#[tokio::test]
async fn publish_event_uses_runtime_services_event_sink() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let events = Arc::new(RecordingRuntimeEventSink::default());
    let services = runtime_services_with_events(events.clone());
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .with_services(services)
        .build()
        .expect("runtime");

    let event = RuntimeEventEnvelope {
        session_id: "session_1".to_string(),
        turn_id: Some("turn_1".to_string()),
        source: Some(AgentSubmissionSource::Cli),
        event_type: RuntimeEventType::TurnStarted,
        payload: serde_json::json!({ "phase": "submitted" }),
    };

    runtime.publish_event(event.clone()).await.expect("publish event");

    assert_eq!(events.events(), vec![event]);
}

#[tokio::test]
async fn run_handle_exposes_configured_agent_event_stream() {
    let ports = Arc::new(FakeAgentRuntimePorts::default());
    let events = AgentEventStream::new();
    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(ports)
        .with_event_stream(events.clone())
        .build()
        .expect("runtime");

    let handle = runtime
        .run(AgentRunRequest::new(SessionSelector::existing("session_1"), "hello"))
        .await
        .expect("run");

    let handle_events = handle.events.as_ref().expect("event stream");
    assert!(handle_events.is_empty());

    let event = RuntimeEventEnvelope {
        session_id: handle.session_id.clone(),
        turn_id: Some(handle.turn_id.clone()),
        source: Some(AgentSubmissionSource::Cli),
        event_type: RuntimeEventType::TurnStarted,
        payload: serde_json::json!({ "phase": "submitted" }),
    };

    runtime.publish_event(event.clone()).await.expect("publish event");

    assert_eq!(handle_events.snapshot(), vec![event.clone()]);
    assert_eq!(events.drain(), vec![event]);
    assert!(handle_events.is_empty());
}

#[tokio::test]
async fn port_errors_remain_typed() {
    #[derive(Debug)]
    struct FailingSubmissionPort;

    #[async_trait::async_trait]
    impl AgentSubmissionPort for FailingSubmissionPort {
        async fn create_session(&self, _request: AgentSessionCreateRequest) -> PortResult<AgentSessionCreateResult> {
            Err(PortError::new(PortErrorKind::Backend, "backend failed"))
        }

        async fn submit_message(&self, _request: AgentSubmissionRequest) -> PortResult<AgentSubmissionResult> {
            Err(PortError::new(PortErrorKind::Backend, "backend failed"))
        }

        async fn resolve_session_agent_type(&self, _session_id: &str) -> PortResult<Option<String>> {
            Ok(None)
        }
    }

    let runtime = AgentRuntimeBuilder::new()
        .with_submission_port(Arc::new(FailingSubmissionPort))
        .build()
        .expect("runtime");

    let err = runtime
        .run(AgentRunRequest::new(SessionSelector::existing("session_1"), "hello"))
        .await
        .unwrap_err();

    assert_eq!(
        err,
        RuntimeError::Port(PortError::new(PortErrorKind::Backend, "backend failed"))
    );
}
