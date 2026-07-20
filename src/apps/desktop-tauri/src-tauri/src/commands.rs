//! Tauri commands: minimal chat loop.
//! Each command body forwards work to the long-lived core runtime (W4
//! discipline) via a oneshot channel so the Tauri async runtime never
//! blocks_on or spawns a throwaway runtime.

use serde::{Deserialize, Serialize};
use northhing_core::kernel_facade::{kernel_facade, KernelFacade};
use northhing_kernel_api::turn::{SubmissionPolicyDto, TriggerSourceDto, TurnInputDto};
use northhing_kernel_api::session::{SessionConfigDto, KernelSessionApi};
use northhing_kernel_api::KernelTurnApi;

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

/// Map a kernel_api MessageRole to its wire string.
fn message_role_str(role: &northhing_kernel_api::session::MessageRoleDto) -> String {
    match role {
        northhing_kernel_api::session::MessageRoleDto::User => "user".to_string(),
        northhing_kernel_api::session::MessageRoleDto::Assistant => "assistant".to_string(),
        northhing_kernel_api::session::MessageRoleDto::Tool => "tool".to_string(),
        northhing_kernel_api::session::MessageRoleDto::System => "system".to_string(),
    }
}

/// Map kernel_api MessageContentDto to text string.
fn message_content_text(content: &northhing_kernel_api::session::MessageContentDto) -> String {
    match content {
        northhing_kernel_api::session::MessageContentDto::Text(t) => t.clone(),
        northhing_kernel_api::session::MessageContentDto::Multimodal { text, .. } => text.clone(),
        northhing_kernel_api::session::MessageContentDto::Mixed { text, .. } => text.clone(),
        northhing_kernel_api::session::MessageContentDto::ToolResult {
            result_for_assistant,
            ..
        } => result_for_assistant.clone().unwrap_or_default(),
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
    let facade = kernel_facade();
    let workspace = workspace_path();
    let config = SessionConfigDto {
        workspace_path: Some(workspace),
        agent_type: DEFAULT_MODE_ID.to_string(),
        model_name: String::new(),
    };
    let session_id = facade.create_session(config).await?;
    Ok(session_id)
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
    let facade = kernel_facade();
    let summaries = facade.list_sessions().await?;
    Ok(summaries
        .into_iter()
        .map(|s| SessionMetaDto {
            id: s.id,
            name: s.name,
            updated_at: s.updated_at as u64,
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
    if text.trim().is_empty() {
        anyhow::bail!("message text must not be empty");
    }
    let facade = kernel_facade();
    let input = TurnInputDto {
        session_id,
        text,
        mode: DEFAULT_MODE_ID.to_string(),
        policy: SubmissionPolicyDto {
            allow_subagent: true,
            max_turns: None,
        },
        source: TriggerSourceDto::User,
    };
    facade.submit_turn(input).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_or_create_latest_session() -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_get_or_create_latest_session()
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_get_or_create_latest_session() -> anyhow::Result<String> {
    let facade = kernel_facade();
    let summaries = facade.list_sessions().await?;
    if let Some(latest) = summaries
        .into_iter()
        .max_by_key(|s| s.updated_at)
    {
        return Ok(latest.id);
    }
    do_create_session().await
}

#[tauri::command]
pub async fn stop_streaming(session_id: String, turn_id: String) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core_rt::core_rt().spawn(async move {
        let r = do_stop_streaming(session_id, turn_id)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.map_err(|_| "core runtime dropped".to_string())?
}

async fn do_stop_streaming(_session_id: String, turn_id: String) -> anyhow::Result<()> {
    let facade = kernel_facade();
    facade.stop_turn(&turn_id).await?;
    Ok(())
}

// ---------- UI preferences (display-only; NOT the AI config) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPrefsDto {
    pub agent_name: String,
}

impl Default for UiPrefsDto {
    fn default() -> Self {
        Self {
            agent_name: "northhing".to_string(),
        }
    }
}

fn ui_prefs_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("northhing")
        .join("config")
        .join("desktop-ui.json")
}

#[tauri::command]
pub async fn get_ui_prefs() -> Result<UiPrefsDto, String> {
    let path = ui_prefs_path();
    tokio::task::spawn_blocking(move || match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| e.to_string()),
        Err(_) => Ok(UiPrefsDto::default()),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_ui_prefs(agent_name: String) -> Result<(), String> {
    let name = agent_name.trim().to_string();
    if name.is_empty() {
        return Err("agent name must not be empty".to_string());
    }
    let path = ui_prefs_path();
    let parent = path.parent().map(|p| p.to_path_buf());
    let prefs = UiPrefsDto { agent_name: name };
    let raw = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        if let Some(ref p) = parent {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, raw).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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
    let facade = kernel_facade();
    let messages = facade.get_messages(&session_id).await?;
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
