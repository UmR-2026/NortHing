// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room kernel facade bridge (P0a).
// Thin async wrapper over `northhing_core::kernel_facade()` and an event mpsc channel.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::events::{KernelEventDto, KernelEventsApi};
use northhing_kernel_api::session::{
    KernelSessionApi, SessionConfigDto, SessionDto, SessionId, SessionSummaryDto,
};
use northhing_kernel_api::settings::{
    AIModelConfigDto, GlobalConfigDto, KernelSettingsApi, MCPServerDto,
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

/// Retrieves the detail of a single session.
pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
    kernel_facade().get_session(id).await
}

/// Ensures a room session exists, returning an existing or newly created `SessionId`.
pub async fn ensure_room_session() -> Result<SessionId, KernelError> {
    let list = list_sessions().await?;
    if let Some(first) = list.into_iter().next() {
        return Ok(first.id);
    }
    let config = SessionConfigDto {
        workspace_path: None,
        agent_type: "agentic".into(),
        model_name: "default".into(),
        name: Some("诊室".into()),
    };
    kernel_facade().create_session(config).await
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
        let _ = get_session(&"test-session".to_string()).await;
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
}
