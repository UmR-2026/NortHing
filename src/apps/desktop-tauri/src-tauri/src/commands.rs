//! Tauri commands: minimal chat loop.
//! Each command body forwards work to the long-lived core runtime (W4
//! discipline) via a oneshot channel so the Tauri async runtime never
//! blocks_on or spawns a throwaway runtime.

use serde::{Deserialize, Serialize};
use std::path::Path;

const DEFAULT_MODE_ID: &str = "agentic";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetaDto {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDto {
    pub id: String,
    pub role: String,
    pub content: String,
    pub is_streaming: bool,
}

/// Extract the textual content of a core `MessageContent`.
fn message_content_text(content: &northhing_core::agentic::core::MessageContent) -> String {
    match content {
        northhing_core::agentic::core::MessageContent::Text(t) => t.clone(),
        northhing_core::agentic::core::MessageContent::Multimodal { text, .. } => text.clone(),
        northhing_core::agentic::core::MessageContent::Mixed { text, .. } => text.clone(),
        northhing_core::agentic::core::MessageContent::ToolResult {
            result_for_assistant,
            ..
        } => result_for_assistant.clone().unwrap_or_default(),
    }
}

/// Map a core `MessageRole` to its wire string.
fn message_role_str(role: &northhing_core::agentic::core::MessageRole) -> String {
    match role {
        northhing_core::agentic::core::MessageRole::User => "user".to_string(),
        northhing_core::agentic::core::MessageRole::Assistant => "assistant".to_string(),
        northhing_core::agentic::core::MessageRole::Tool => "tool".to_string(),
        northhing_core::agentic::core::MessageRole::System => "system".to_string(),
    }
}

fn workspace_path() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

fn system_time_to_ms(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[tauri::command]
pub async fn create_session() -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_create_session().await.map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_create_session() -> anyhow::Result<String> {
    let coordinator = northhing_core::agentic::coordination::global_coordinator()
        .ok_or_else(|| anyhow::anyhow!("global coordinator not available"))?;
    let workspace = workspace_path();
    let config = northhing_core::agentic::core::SessionConfig {
        workspace_path: Some(workspace.clone()),
        ..Default::default()
    };
    let name = format!("session-{}", system_time_to_ms(std::time::SystemTime::now()));
    let session = coordinator
        .create_session(name, DEFAULT_MODE_ID.to_string(), config)
        .await?;
    Ok(session.session_id)
}

#[tauri::command]
pub async fn list_sessions() -> Result<Vec<SessionMetaDto>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_list_sessions().await.map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_list_sessions() -> anyhow::Result<Vec<SessionMetaDto>> {
    let coordinator = northhing_core::agentic::coordination::global_coordinator()
        .ok_or_else(|| anyhow::anyhow!("global coordinator not available"))?;
    let workspace = workspace_path();
    let summaries = coordinator.list_sessions(Path::new(&workspace)).await?;
    Ok(summaries
        .into_iter()
        .map(|s| SessionMetaDto {
            id: s.session_id,
            name: s.session_name,
            updated_at: system_time_to_ms(s.last_activity_at),
        })
        .collect())
}

#[tauri::command]
pub async fn send_message(session_id: String, text: String) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_send_message(session_id, text)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_send_message(session_id: String, text: String) -> anyhow::Result<()> {
    let scheduler = northhing_core::agentic::coordination::global_scheduler()
        .ok_or_else(|| anyhow::anyhow!("global scheduler not available"))?;
    let workspace = workspace_path();
    let outcome = scheduler
        .submit(
            session_id,
            text,
            None,
            None,
            DEFAULT_MODE_ID.to_string(),
            Some(workspace),
            northhing_core::agentic::coordination::DialogSubmissionPolicy::for_source(
                northhing_core::agentic::coordination::DialogTriggerSource::DesktopApi,
            ),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    match outcome {
        northhing_core::agentic::coordination::DialogSubmitOutcome::Started { .. }
        | northhing_core::agentic::coordination::DialogSubmitOutcome::Queued { .. } => Ok(()),
    }
}

#[tauri::command]
pub async fn get_messages(session_id: String) -> Result<Vec<MessageDto>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_get_messages(session_id).await.map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_get_messages(session_id: String) -> anyhow::Result<Vec<MessageDto>> {
    let coordinator = northhing_core::agentic::coordination::global_coordinator()
        .ok_or_else(|| anyhow::anyhow!("global coordinator not available"))?;
    let messages = coordinator.get_messages(&session_id).await?;
    Ok(messages
        .into_iter()
        .map(|m| MessageDto {
            id: m.id,
            role: message_role_str(&m.role),
            content: message_content_text(&m.content),
            is_streaming: false,
        })
        .collect())
}
