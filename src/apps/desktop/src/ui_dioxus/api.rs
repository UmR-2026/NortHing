// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room kernel facade bridge (P0a).
// Thin async wrapper over `northhing_core::kernel_facade()` and an event mpsc channel.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi};
use northhing_kernel_api::session::{
    KernelSessionApi, MessageDto, SessionConfigDto, SessionDto, SessionId, SessionSummaryDto,
    WorkspaceSessionsDto,
};
use northhing_kernel_api::settings::{
    AIModelConfigDto, GlobalConfigDto, KernelSettingsApi, MCPServerDto, ProviderFormDto,
    ProviderTestResultDto,
};
use northhing_kernel_api::tools::KernelToolsApi;
use northhing_kernel_api::turn::{
    KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto,
};

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
pub async fn respond_to_tool_confirmation(
    tool_id: &str,
    approved: bool,
) -> Result<(), KernelError> {
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
pub async fn test_provider_config(
    form: ProviderFormDto,
) -> Result<ProviderTestResultDto, KernelError> {
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

/// Creates a subscription to the kernel event stream and returns an unbounded/bounded mpsc receiver.
///
/// Converts the callback-based `subscribe_events` interface into an async `tokio::sync::mpsc::Receiver`.
/// Events that exceed channel capacity are dropped to prevent stalling kernel event delivery.
pub fn event_channel() -> tokio::sync::mpsc::Receiver<KernelEventDto> {
    let (tx, rx) = tokio::sync::mpsc::channel(256);
    let subscribe_task = async move {
        let callback = Box::new(move |dto: KernelEventDto| {
            let _ = tx.try_send(dto);
        });
        if let Err(e) = kernel_facade().subscribe_events(callback).await {
            tracing::warn!("ui_dioxus::api::event_channel subscribe failed: {e}");
        }
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(subscribe_task);
    } else {
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
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
}
