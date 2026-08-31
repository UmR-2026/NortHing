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

/// Deletes a session by id.
pub async fn delete_session(id: &SessionId) -> Result<(), KernelError> {
    kernel_facade().delete_session(id).await
}

/// Renames a session by id.
pub async fn rename_session(id: &SessionId, name: &str) -> Result<(), KernelError> {
    kernel_facade().rename_session(id, name).await
}

/// Searches sessions across workspace matching query text.
pub async fn search_sessions(query: &str, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError> {
    kernel_facade().search_sessions(query, None, limit).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::session::SessionStatusDto;
    use northhing_kernel_api::session::SessionSummaryDto;
    use northhing_kernel_api::session::WorkspaceSessionsDto;

    #[tokio::test]
    async fn test_ensure_room_session_fails_cleanly_when_uninitialized() {
        let res = ensure_room_session().await;
        assert!(res.is_err());
    }

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
}
