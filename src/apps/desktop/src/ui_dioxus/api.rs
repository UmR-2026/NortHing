// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room kernel facade bridge (P0a).
// Thin async wrapper over `northhing_core::kernel_facade()` and an event mpsc channel.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi};
use northhing_kernel_api::session::{
    KernelSessionApi, MessageDto, SessionConfigDto, SessionDto, SessionId, SessionSummaryDto, WorkspaceSessionsDto,
};
use northhing_kernel_api::settings::{
    AIModelConfigDto, GlobalConfigDto, KernelSettingsApi, MCPServerDto, ProviderFormDto, ProviderTestResultDto,
};
use northhing_kernel_api::tools::KernelToolsApi;
use northhing_kernel_api::turn::{KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto};

/// Submits a user dialog turn for the given session.
///
/// Builds a default agentic `TurnInputDto` and forwards it to the kernel facade.
/// Returns the generated `TurnId` on acceptance, or `KernelError::Runtime` on rejection.
pub async fn submit_turn(session_id: &str, text: String) -> Result<TurnId, KernelError> {
    let input = TurnInputDto {
        session_id: session_id.to_string(),
        text,
        mode: "agentic".into(),
        policy: SubmissionPolicyDto {
            allow_subagent: true,
            max_turns: None,
        },
        source: TriggerSourceDto::User,
        workspace_path: None,
    };
    let outcome = kernel_facade().submit_turn(input).await?;
    if !outcome.accepted {
        let err_msg = outcome
            .error
            .unwrap_or_else(|| "submit_turn rejected by kernel".to_string());
        return Err(KernelError::Runtime(err_msg));
    }
    Ok(outcome.turn_id)
}

/// Stops/cancels an executing dialog turn.
pub async fn stop_turn(turn_id: &TurnId) -> Result<(), KernelError> {
    kernel_facade().stop_turn(turn_id).await
}

/// Lists summaries for all sessions.
pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
    kernel_facade().list_sessions().await
}

/// Lists workspace-grouped session summaries across all workspaces.
pub async fn list_sessions_all_workspaces() -> Result<Vec<WorkspaceSessionsDto>, KernelError> {
    kernel_facade().list_sessions_all_workspaces().await
}

/// Retrieves the detail of a single session.
pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
    kernel_facade().get_session(id).await
}

/// Retrieves the message history of a single session.
pub async fn get_messages(id: &SessionId) -> Result<Vec<MessageDto>, KernelError> {
    kernel_facade().get_messages(id).await
}

/// Pick the room session from workspace-grouped summaries.
/// Preferred workspace hit wins; otherwise the first group that has any
/// session (groups are ordered most-recent-access first by the facade);
/// `None` means "create fresh".
fn pick_room_session<'a>(
    groups: &'a [WorkspaceSessionsDto],
    preferred_workspace: Option<&str>,
) -> Option<&'a SessionSummaryDto> {
    if let Some(ws) = preferred_workspace {
        groups
            .iter()
            .find(|g| g.workspace_path == ws)
            .and_then(|g| g.sessions.first())
    } else {
        groups
            .iter()
            .find(|g| !g.sessions.is_empty())
            .and_then(|g| g.sessions.first())
    }
}

static ROOM_SESSION_CACHE: tokio::sync::Mutex<Option<String>> = tokio::sync::Mutex::const_new(None);

/// Ensures a room session exists, returning an existing or newly created `SessionId`.
pub async fn ensure_room_session() -> Result<SessionId, KernelError> {
    // ponytail: process-lifetime room session cache, restart required to switch room after session deletion/archival; upgrade path = invalidate on delete_session event
    let mut guard = ROOM_SESSION_CACHE.lock().await;
    if let Some(ref cached_id) = *guard {
        return Ok(cached_id.clone());
    }

    let preferred_workspace = match super::super::app_state::settings::load_app_settings().await {
        Ok(s) => s
            .current_workspace
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| s.workspaces.first().map(|w| w.path.to_string_lossy().to_string())),
        Err(e) => {
            tracing::warn!("ensure_room_session failed to load app settings: {e}");
            None
        }
    };

    let groups = list_sessions_all_workspaces().await?;
    let session_id = if let Some(summary) = pick_room_session(&groups, preferred_workspace.as_deref()) {
        summary.id.clone()
    } else {
        let config = SessionConfigDto {
            workspace_path: preferred_workspace.clone(),
            agent_type: "agentic".into(),
            model_name: "default".into(),
            name: Some("诊室".into()),
        };
        kernel_facade().create_session(config).await?
    };

    *guard = Some(session_id.clone());
    Ok(session_id)
}

/// Responds to a pending tool execution confirmation (approve/reject).
pub async fn respond_to_tool_confirmation(tool_id: &str, approved: bool) -> Result<(), KernelError> {
    kernel_facade()
        .respond_to_tool_confirmation(tool_id, approved, None)
        .await
}

/// Retrieves global configuration including providers and default provider id.
pub async fn get_global_config() -> Result<GlobalConfigDto, KernelError> {
    kernel_facade().get_global_config().await
}

/// Lists all configured AI models.
pub async fn list_model_configs() -> Result<Vec<AIModelConfigDto>, KernelError> {
    kernel_facade().list_model_configs().await
}

/// Sets the default AI provider / model ID.
pub async fn set_default_provider(id: &str) -> Result<(), KernelError> {
    kernel_facade().set_default_provider(id).await
}

/// Lists all configured MCP servers.
pub async fn list_mcp_servers() -> Result<Vec<MCPServerDto>, KernelError> {
    kernel_facade().list_mcp_servers().await
}

/// Sets the enabled state of an MCP server and updates its configuration.
pub async fn set_mcp_enabled(mut server: MCPServerDto, enabled: bool) -> Result<(), KernelError> {
    server.enabled = Some(enabled);
    kernel_facade().upsert_mcp_server(server).await
}

/// Tests a provider configuration without modifying persistent global config.
pub async fn test_provider_config(form: ProviderFormDto) -> Result<ProviderTestResultDto, KernelError> {
    kernel_facade().test_provider_config(form).await
}

/// Stores an API key in the OS keyring for the onboarding flow.
pub async fn store_provider_api_key(provider_id: &str, plaintext: &str) -> anyhow::Result<String> {
    super::super::app_state::settings::store_api_key(
        &*super::super::app_state::settings::PRODUCTION_KEYRING,
        provider_id,
        plaintext,
    )
}

/// Adds or updates an AI model / provider configuration in the kernel facade.
pub async fn upsert_model_config(config: AIModelConfigDto, api_key: Option<String>) -> Result<(), KernelError> {
    kernel_facade().upsert_model_config(config, api_key).await
}

/// Persists the onboarding provider configuration into the OS keyring and kernel facade,
/// and sets it as the default provider in the global configuration.
///
/// Returns `Ok(provider_id)` on success, or `Err(user_facing_chinese_error)` on failure.
pub async fn persist_onboarding_provider(
    model: &str,
    base_url: &str,
    api_key: &str,
    agent_name: &str,
) -> Result<String, String> {
    let provider_id = uuid::Uuid::new_v4().to_string();
    let wire_format = super::super::app_state::settings::infer_provider_wire_format(base_url, model);

    // 1. Store API key in keyring under the provider id
    if let Err(e) = store_provider_api_key(&provider_id, api_key).await {
        let first_line = e.to_string().lines().next().unwrap_or("Key 存储失败").trim().to_string();
        return Err(format!("Key 存储失败: {first_line}"));
    }

    // 2. Build model DTO and persist into core facade
    let model_dto = AIModelConfigDto {
        id: provider_id.clone(),
        provider_id: wire_format.to_string(),
        model: model.trim().to_string(),
        display_name: Some(if !agent_name.trim().is_empty() {
            agent_name.trim().to_string()
        } else {
            model.trim().to_string()
        }),
        max_tokens: None,
        temperature: None,
        base_url: if base_url.trim().is_empty() {
            None
        } else {
            Some(base_url.trim().to_string())
        },
        enabled: Some(true),
        category: Some("general_chat".to_string()),
        capabilities: Some(vec!["text_chat".to_string()]),
        auth: Some("api_key".to_string()),
        inline_think_in_text: Some(false),
    };

    if let Err(e) = upsert_model_config(model_dto, Some(api_key.to_string())).await {
        let first_line = e.to_string().lines().next().unwrap_or("Provider 保存失败").trim().to_string();
        return Err(format!("Provider 保存失败: {first_line}"));
    }

    // 3. Set as default provider in core config
    if let Err(e) = set_default_provider(&provider_id).await {
        let first_line = e.to_string().lines().next().unwrap_or("设为默认 Provider 失败").trim().to_string();
        return Err(format!("设为默认 Provider 失败: {first_line}"));
    }

    Ok(provider_id)
}

/// Capacity limit for buffered lossy text chunks before dropping under UI lag.
pub const MAX_PENDING_TEXT_CHUNKS: usize = 256;

/// Tiered event receiver bridging kernel events to the Dioxus UI.
///
/// Implements tiered event buffering (F2):
/// - TextChunk events are lossy: bounded to `MAX_PENDING_TEXT_CHUNKS` to avoid unbounded memory
///   growth when the UI consumer lags or re-renders heavily.
/// - Control events (TurnState, ToolCall, TurnPhase, Banner, Error) are guaranteed: delivered via
///   an unbounded channel so critical state machine transitions (e.g. Completed/Failed) and approval
///   cards are never dropped.
/// - Event ordering is strictly preserved in FIFO order across all event types within a single stream.
pub struct EventReceiver {
    rx: tokio::sync::mpsc::UnboundedReceiver<KernelEventDto>,
    pending_text_chunks: Arc<AtomicUsize>,
}

impl EventReceiver {
    /// Receives the next event from the tiered event channel.
    pub async fn recv(&mut self) -> Option<KernelEventDto> {
        let event = self.rx.recv().await?;
        if matches!(event, KernelEventDto::TextChunk { .. }) {
            self.pending_text_chunks.fetch_sub(1, Ordering::Relaxed);
        }
        Some(event)
    }

    /// Returns the current number of pending lossy text chunks in the queue.
    pub fn pending_text_chunks(&self) -> usize {
        self.pending_text_chunks.load(Ordering::Relaxed)
    }
}

/// Creates an isolated tiered event bridge returning a callback and receiver pair.
///
/// Used by `event_channel()` for live kernel subscriptions and by unit tests for deterministic verification.
pub fn create_event_bridge() -> (Box<dyn Fn(KernelEventDto) + Send + 'static>, EventReceiver) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let pending_text_chunks = Arc::new(AtomicUsize::new(0));
    let pending_counter = pending_text_chunks.clone();

    let callback = Box::new(move |dto: KernelEventDto| match dto {
        KernelEventDto::TextChunk { .. } => {
            let mut current = pending_counter.load(Ordering::Relaxed);
            loop {
                if current >= MAX_PENDING_TEXT_CHUNKS {
                    tracing::debug!(
                        pending = current,
                        max = MAX_PENDING_TEXT_CHUNKS,
                        "ui_dioxus::api dropping TextChunk due to capacity limit"
                    );
                    return;
                }
                match pending_counter.compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed)
                {
                    Ok(_) => break,
                    Err(actual) => current = actual,
                }
            }
            let _ = tx.send(dto);
        }
        control_dto => {
            let _ = tx.send(control_dto);
        }
    });

    (
        callback,
        EventReceiver {
            rx,
            pending_text_chunks,
        },
    )
}

/// Creates a subscription to the kernel event stream and returns a tiered event receiver.
///
/// Converts the callback-based `subscribe_events` interface into an async `EventReceiver`.
/// TextChunk events that exceed capacity (256) are dropped under UI lag, while critical control
/// events (TurnState, ToolCall, etc.) are always delivered without loss.
pub fn event_channel() -> EventReceiver {
    let (callback, rx) = create_event_bridge();
    let subscribe_task = async move {
        if let Err(e) = kernel_facade().subscribe_events(callback).await {
            tracing::warn!("ui_dioxus::api::event_channel subscribe failed: {e}");
        }
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(subscribe_task);
    } else {
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                rt.block_on(subscribe_task);
            }
        });
    }

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_functions_fail_cleanly_before_init() {
        // Facade is uninitialized in isolated test environment, should return Err not panic
        let _ = submit_turn("test-session", "hello".into()).await;
        let _ = stop_turn(&"test-turn".to_string()).await;
        let _ = list_sessions().await;
        let _ = list_sessions_all_workspaces().await;
        let _ = get_session(&"test-session".to_string()).await;
        let _ = get_messages(&"test-session".to_string()).await;
        let _ = respond_to_tool_confirmation("call-1", true).await;
        let _ = ensure_room_session().await;
        let _ = get_global_config().await;
        let _ = list_model_configs().await;
        let _ = set_default_provider("test-model").await;
        let _ = list_mcp_servers().await;
        let mcp = MCPServerDto {
            id: "test".into(),
            name: "test".into(),
            config: northhing_kernel_api::settings::MCPServerConfigDto {
                command: "node".into(),
                args: vec![],
                env: None,
            },
            location: northhing_kernel_api::settings::ConfigLocationDto::User,
            enabled: Some(true),
        };
        let _ = set_mcp_enabled(mcp, false).await;
        let form = ProviderFormDto {
            provider_id: "onboarding".into(),
            base_url: Some("http://localhost".into()),
            api_key: Some("key".into()),
            model: Some("default".into()),
            provider_type: None,
        };
        let _ = test_provider_config(form).await;
        let _ = store_provider_api_key("onboarding", "key").await;
        let dummy_model = AIModelConfigDto {
            id: "test".into(),
            provider_id: "openai".into(),
            model: "test".into(),
            display_name: None,
            max_tokens: None,
            temperature: None,
            base_url: None,
            enabled: Some(true),
            category: None,
            capabilities: None,
            auth: None,
            inline_think_in_text: None,
        };
        let _ = upsert_model_config(dummy_model, None).await;
        let _ = persist_onboarding_provider("claude", "http://localhost", "key", "Agent").await;
    }

    #[tokio::test]
    async fn test_persist_onboarding_provider_success_flow() -> anyhow::Result<()> {
        let _ = northhing_core::service::config::initialize_global_config().await;
        let res = persist_onboarding_provider(
            "claude-3-7-sonnet",
            "https://api.anthropic.com/v1",
            "sk-ant-test-key-9999",
            "TestAgent",
        )
        .await;
        assert!(res.is_ok(), "persist_onboarding_provider failed: {:?}", res.err());
        let provider_id = res.unwrap();

        // Verify in global config
        let global_cfg = get_global_config().await?;
        assert_eq!(global_cfg.default_provider_id.as_deref(), Some(provider_id.as_str()));
        let provider = global_cfg
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .expect("persisted provider must be in global config");
        assert_eq!(provider.model, "claude-3-7-sonnet");
        assert_eq!(provider.provider_type.as_deref(), Some("anthropic"));

        // Verify in model configs
        let models = list_model_configs().await?;
        let model = models
            .iter()
            .find(|m| m.id == provider_id)
            .expect("persisted model must be in model configs");
        assert_eq!(model.model, "claude-3-7-sonnet");
        assert_eq!(model.provider_id, "anthropic");

        // Clean up
        let _ = kernel_facade().delete_model_config(&provider_id).await;
        Ok(())
    }

    #[tokio::test]
    async fn test_ensure_room_session_fails_cleanly_when_uninitialized() {
        let res = ensure_room_session().await;
        assert!(res.is_err());
    }

    #[test]
    fn test_event_channel_returns_receiver() {
        let rx = event_channel();
        drop(rx);
    }

    #[test]
    fn test_pick_room_session_preferred_hit() {
        let s1 = SessionSummaryDto {
            id: "s1".into(),
            name: "Room 1".into(),
            updated_at: 100,
            status: northhing_kernel_api::session::SessionStatusDto::Active,
            parent_session_id: None,
            state: None,
        };
        let s2 = SessionSummaryDto {
            id: "s2".into(),
            name: "Room 2".into(),
            updated_at: 200,
            status: northhing_kernel_api::session::SessionStatusDto::Active,
            parent_session_id: None,
            state: None,
        };
        let groups = vec![
            WorkspaceSessionsDto {
                workspace_path: "/ws/a".into(),
                sessions: vec![s1.clone()],
            },
            WorkspaceSessionsDto {
                workspace_path: "/ws/b".into(),
                sessions: vec![s2.clone()],
            },
        ];

        let picked = pick_room_session(&groups, Some("/ws/b"));
        assert_eq!(picked.map(|s| s.id.as_str()), Some("s2"));
    }

    #[test]
    fn test_pick_room_session_preferred_miss_returns_none() {
        let s1 = SessionSummaryDto {
            id: "s1".into(),
            name: "Room 1".into(),
            updated_at: 100,
            status: northhing_kernel_api::session::SessionStatusDto::Active,
            parent_session_id: None,
            state: None,
        };
        let groups = vec![
            WorkspaceSessionsDto {
                workspace_path: "/ws/a".into(),
                sessions: vec![s1],
            },
            WorkspaceSessionsDto {
                workspace_path: "/ws/b".into(),
                sessions: vec![],
            },
        ];

        // Preferred ws does not exist -> returns None
        assert!(pick_room_session(&groups, Some("/ws/c")).is_none());

        // Preferred ws exists but sessions vector is empty -> returns None
        assert!(pick_room_session(&groups, Some("/ws/b")).is_none());
    }

    #[test]
    fn test_pick_room_session_no_preferred_picks_first_non_empty() {
        let s2 = SessionSummaryDto {
            id: "s2".into(),
            name: "Room 2".into(),
            updated_at: 200,
            status: northhing_kernel_api::session::SessionStatusDto::Active,
            parent_session_id: None,
            state: None,
        };
        let groups = vec![
            WorkspaceSessionsDto {
                workspace_path: "/ws/a".into(),
                sessions: vec![],
            },
            WorkspaceSessionsDto {
                workspace_path: "/ws/b".into(),
                sessions: vec![s2.clone()],
            },
        ];

        let picked = pick_room_session(&groups, None);
        assert_eq!(picked.map(|s| s.id.as_str()), Some("s2"));
    }

    #[test]
    fn test_pick_room_session_empty_groups_returns_none() {
        let groups_empty: Vec<WorkspaceSessionsDto> = vec![];
        assert!(pick_room_session(&groups_empty, None).is_none());

        let groups_all_empty = vec![WorkspaceSessionsDto {
            workspace_path: "/ws/a".into(),
            sessions: vec![],
        }];
        assert!(pick_room_session(&groups_all_empty, None).is_none());
    }

    #[tokio::test]
    async fn test_tiered_event_channel_text_chunk_lossy_control_guaranteed() {
        use northhing_kernel_api::events::{ToolCallDto, ToolCallPhase};
        use northhing_kernel_api::turn::TurnStateKind;

        let (callback, mut rx) = create_event_bridge();

        // 1. Schedulers/kernel emit 356 TextChunks (saturating the 256 MAX_PENDING_TEXT_CHUNKS buffer)
        for i in 0..356 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("chunk-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), MAX_PENDING_TEXT_CHUNKS);

        // 2. Emit critical control events while the lossy channel is 100% saturated
        callback(KernelEventDto::TurnState {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            state: TurnStateKind::Completed,
            duration_ms: Some(123),
            error: None,
            error_kind: None,
        });

        callback(KernelEventDto::ToolCall(ToolCallDto {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            call_id: "c1".into(),
            phase: ToolCallPhase::AwaitingConfirmation,
            name: "execute_cmd".into(),
            summary: "run test".into(),
            detail: None,
            result_count: None,
        }));

        // 3. Verify exactly 256 TextChunks arrive in FIFO order (chunks 0..256)
        for i in 0..256 {
            match rx.recv().await {
                Some(KernelEventDto::TextChunk { text, .. }) => {
                    assert_eq!(text, format!("chunk-{i}"));
                }
                other => panic!("expected TextChunk {i}, got {other:?}"),
            }
        }

        // 4. Verify TurnState::Completed is not dropped and arrives immediately after the 256 chunks
        match rx.recv().await {
            Some(KernelEventDto::TurnState { state, turn_id, .. }) => {
                assert!(matches!(state, TurnStateKind::Completed));
                assert_eq!(turn_id, "t1");
            }
            other => panic!("expected TurnState::Completed, got {other:?}"),
        }

        // 5. Verify ToolCall is not dropped
        match rx.recv().await {
            Some(KernelEventDto::ToolCall(tc)) => {
                assert_eq!(tc.call_id, "c1");
                assert_eq!(tc.phase, ToolCallPhase::AwaitingConfirmation);
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }

        // Buffer is fully drained
        assert_eq!(rx.pending_text_chunks(), 0);
    }

    #[tokio::test]
    async fn test_tiered_event_channel_drain_refills_budget() {
        use northhing_kernel_api::turn::TurnStateKind;

        let (callback, mut rx) = create_event_bridge();

        // Fill to capacity
        for i in 0..256 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("c-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), 256);

        // One extra chunk dropped
        callback(KernelEventDto::TextChunk {
            session_id: "s1".into(),
            text: "dropped".into(),
        });
        assert_eq!(rx.pending_text_chunks(), 256);

        // Consume 10 chunks
        for _ in 0..10 {
            assert!(rx.recv().await.is_some());
        }
        assert_eq!(rx.pending_text_chunks(), 246);

        // Send 10 new chunks - should be accepted
        for i in 0..10 {
            callback(KernelEventDto::TextChunk {
                session_id: "s1".into(),
                text: format!("refill-{i}"),
            });
        }
        assert_eq!(rx.pending_text_chunks(), 256);

        // Control event accepted at full capacity
        callback(KernelEventDto::TurnState {
            session_id: "s1".into(),
            turn_id: "t2".into(),
            state: TurnStateKind::Failed,
            duration_ms: None,
            error: Some("test error".into()),
            error_kind: None,
        });

        // Drain remaining 246 initial chunks + 10 refill chunks
        for _ in 0..256 {
            assert!(matches!(rx.recv().await, Some(KernelEventDto::TextChunk { .. })));
        }

        // TurnState arrived safely
        match rx.recv().await {
            Some(KernelEventDto::TurnState { state, error, .. }) => {
                assert!(matches!(state, TurnStateKind::Failed));
                assert_eq!(error.as_deref(), Some("test error"));
            }
            other => panic!("expected TurnState::Failed, got {other:?}"),
        }
    }
}
