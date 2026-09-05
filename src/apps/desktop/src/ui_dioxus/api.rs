// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room kernel facade bridge (P0a).
// Thin async wrapper over `northhing_core::kernel_facade()`. split into
// api_settings.rs, api_events.rs, api_memory.rs; this file retains the
// turn/session/room/confirmation core pipeline.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::session::{
    KernelSessionApi, MessageDto, SessionConfigDto, SessionDto, SessionId, SessionSearchHitDto, SessionSummaryDto,
    WorkspaceSessionsDto,
};
use northhing_kernel_api::tools::KernelToolsApi;
use northhing_kernel_api::turn::{KernelTurnApi, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto};

#[path = "api_provider_edit.rs"]
mod api_provider_edit;
pub use super::api_events::*;
pub use super::api_memory::*;
pub use super::api_settings::*;
pub use api_provider_edit::*;

/// Submits a user dialog turn for the given session.
///
/// Builds a default agentic `TurnInputDto` and forwards it to the kernel facade.
/// Returns the generated `TurnId` on acceptance, or `KernelError::Runtime` on rejection.
pub async fn submit_turn(session_id: &str, text: String) -> Result<TurnId, KernelError> {
    let session_id = session_id.to_string();
    kernel_dispatch("submit_turn", async move {
        let input = TurnInputDto {
            session_id,
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
    })
    .await
}

/// Stops/cancels an executing dialog turn.
pub async fn stop_turn(turn_id: &TurnId) -> Result<(), KernelError> {
    let turn_id = turn_id.clone();
    kernel_dispatch("stop_turn", async move {
        kernel_facade().stop_turn(&turn_id).await
    })
    .await
}

/// Lists summaries for all sessions.
pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, KernelError> {
    kernel_dispatch("list_sessions", async move {
        kernel_facade().list_sessions().await
    })
    .await
}

/// Lists workspace-grouped session summaries across all workspaces.
pub async fn list_sessions_all_workspaces() -> Result<Vec<WorkspaceSessionsDto>, KernelError> {
    kernel_dispatch("list_sessions_all_workspaces", async move {
        kernel_facade().list_sessions_all_workspaces().await
    })
    .await
}

/// Retrieves the detail of a single session.
pub async fn get_session(id: &SessionId) -> Result<SessionDto, KernelError> {
    let id = id.clone();
    kernel_dispatch("get_session", async move {
        kernel_facade().get_session(&id).await
    })
    .await
}

/// Retrieves the message history of a single session.
pub async fn get_messages(id: &SessionId) -> Result<Vec<MessageDto>, KernelError> {
    let id = id.clone();
    kernel_dispatch("get_messages", async move {
        kernel_facade().get_messages(&id).await
    })
    .await
}

/// Deletes a session by id.
pub async fn delete_session(id: &SessionId) -> Result<(), KernelError> {
    let id = id.clone();
    kernel_dispatch("delete_session", async move {
        kernel_facade().delete_session(&id).await
    })
    .await
}

/// Renames a session by id.
pub async fn rename_session(id: &SessionId, name: &str) -> Result<(), KernelError> {
    let id = id.clone();
    let name = name.to_string();
    kernel_dispatch("rename_session", async move {
        kernel_facade().rename_session(&id, &name).await
    })
    .await
}

/// Searches sessions across workspace matching query text.
pub async fn search_sessions(query: &str, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError> {
    let query = query.to_string();
    kernel_dispatch("search_sessions", async move {
        kernel_facade().search_sessions(&query, None, limit).await
    })
    .await
}

/// Returns the cached room session id if any.
pub async fn get_room_session_id() -> Option<String> {
    let guard = ROOM_SESSION_CACHE.lock().await;
    guard.clone()
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

    let session_id = kernel_dispatch("ensure_room_session", async move {
        let groups = kernel_facade().list_sessions_all_workspaces().await?;
        if let Some(summary) = pick_room_session(&groups, preferred_workspace.as_deref()) {
            Ok(summary.id.clone())
        } else {
            let config = SessionConfigDto {
                workspace_path: preferred_workspace.clone(),
                agent_type: "agentic".into(),
                model_name: "default".into(),
                name: Some("诊室".into()),
            };
            kernel_facade().create_session(config).await
        }
    })
    .await?;

    *guard = Some(session_id.clone());
    Ok(session_id)
}

/// Responds to a pending tool execution confirmation (approve/reject).
pub async fn respond_to_tool_confirmation(tool_id: &str, approved: bool) -> Result<(), KernelError> {
    let tool_id = tool_id.to_string();
    kernel_dispatch("respond_to_tool_confirmation", async move {
        kernel_facade()
            .respond_to_tool_confirmation(&tool_id, approved, None)
            .await
    })
    .await
}

/// Spawns an async operation onto the worker `turn_runtime` and awaits its result
/// on the UI executor via a oneshot channel.
///
/// Keeps kernel awaits off the Dioxus UI executor to prevent busy-polling hangs.
/// When turn_runtime is unavailable (e.g. tests or uninit CLI), falls back to inline execution.
pub(crate) async fn spawn_on_turn_runtime<T, F>(caller: &'static str, fut: F) -> Result<T, ()>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    let Some(rt) = crate::app_state::turn_runtime::turn_runtime() else {
        tracing::warn!("ui_dioxus::{caller} turn_runtime handle unavailable, falling back to inline execution");
        return Ok(fut.await);
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    let start = std::time::Instant::now();
    tracing::info!("ui_dioxus::{caller} spawning onto turn_runtime");
    rt.spawn(async move {
        let res = fut.await;
        let _ = tx.send(res);
    });

    match rx.await {
        Ok(val) => {
            tracing::info!("ui_dioxus::{caller} completed on turn_runtime in {:?}", start.elapsed());
            Ok(val)
        }
        Err(e) => {
            tracing::warn!("ui_dioxus::{caller} background channel closed after {:?}: {e}", start.elapsed());
            Err(())
        }
    }
}

/// Dispatches a kernel operation returning `Result<T, KernelError>` to `turn_runtime`.
///
/// Flattens background channel failures to `KernelError::Runtime`.
pub(crate) async fn kernel_dispatch<T, F>(caller: &'static str, fut: F) -> Result<T, KernelError>
where
    T: Send + 'static,
    F: std::future::Future<Output = Result<T, KernelError>> + Send + 'static,
{
    match spawn_on_turn_runtime(caller, fut).await {
        Ok(Ok(res)) => Ok(res),
        Ok(Err(e)) => {
            tracing::warn!("ui_dioxus::{caller} kernel returned error: {e}");
            Err(e)
        }
        Err(()) => Err(KernelError::Runtime(format!("ui_dioxus::{caller} background channel closed"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::session::SessionStatusDto;
    use northhing_kernel_api::session::SessionSummaryDto;
    use northhing_kernel_api::session::WorkspaceSessionsDto;

    #[test]
    fn test_pick_room_session_preferred_hit() {
        let s1 = SessionSummaryDto {
            id: "s1".into(),
            name: "Room 1".into(),
            updated_at: 100,
            status: SessionStatusDto::Active,
            parent_session_id: None,
            state: None,
        };
        let s2 = SessionSummaryDto {
            id: "s2".into(),
            name: "Room 2".into(),
            updated_at: 200,
            status: SessionStatusDto::Active,
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
            status: SessionStatusDto::Active,
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
            status: SessionStatusDto::Active,
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
    async fn test_spawn_on_turn_runtime_behavior() {
        let res = spawn_on_turn_runtime("test", async { 42 }).await;
        assert_eq!(res, Ok(42));
    }

    #[tokio::test]
    async fn test_kernel_dispatch_behavior() {
        let res = kernel_dispatch("test_kernel", async { Ok(100) }).await;
        assert!(matches!(res, Ok(100)));

        let err_res: Result<(), KernelError> = kernel_dispatch("test_kernel_err", async {
            Err(KernelError::Runtime("inner failure".into()))
        })
        .await;
        assert!(matches!(err_res, Err(KernelError::Runtime(ref msg)) if msg.contains("inner failure")));
    }
}
