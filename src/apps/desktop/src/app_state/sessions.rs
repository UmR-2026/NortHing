//! sessions module — see mod.rs for the wiring entry point.

use super::slint_glue::AppWindow;
use super::*;
use slint::{ModelRc, SharedString, VecModel};

/// Format a SystemTime as a human-readable string
pub(super) fn format_time(time: std::time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

/// Convert a core SessionSummary to a Slint SessionItem
pub(super) fn session_summary_to_item(summary: &northhing_core::agentic::core::SessionSummary) -> SessionItem {
    let is_active = matches!(
        summary.state,
        northhing_core::agentic::core::SessionState::Processing { .. }
    );
    // Phase C.1: parent_id uses an empty-string sentinel so the Slint struct
    // can stay Default-constructible while the Rust side threads `Option<String>`
    // through. `build_sessions_model` (below) computes depth from these
    // parent_id values.
    let parent_id = summary.parent_session_id.clone().unwrap_or_default();
    SessionItem {
        id: SharedString::from(summary.session_id.clone()),
        name: SharedString::from(summary.session_name.clone()),
        timestamp: SharedString::from(format_time(summary.last_activity_at)),
        is_active,
        parent_id: SharedString::from(parent_id),
        depth: 0, // Filled in by `build_sessions_model`.
        // Phase 4: model override, broken-workspace marker, workspace path.
        // Currently unused (will be populated when Rust state model is
        // extended); default values keep the Slint struct constructible.
        model_override: SharedString::from(""),
        is_workspace_broken: false,
        workspace_path: SharedString::from(""),
    }
}

/// Convert a core Message to a Slint MessageItem
pub(super) fn message_to_item(msg: &northhing_core::agentic::core::Message, is_streaming: bool) -> MessageItem {
    let role = match msg.role {
        northhing_core::agentic::core::MessageRole::User => "user",
        northhing_core::agentic::core::MessageRole::Assistant => "assistant",
        northhing_core::agentic::core::MessageRole::Tool => "tool",
        northhing_core::agentic::core::MessageRole::System => "system",
    };

    let content = match &msg.content {
        northhing_core::agentic::core::MessageContent::Text(t) => t.clone(),
        northhing_core::agentic::core::MessageContent::Multimodal { text, .. } => text.clone(),
        northhing_core::agentic::core::MessageContent::ToolResult {
            result_for_assistant, ..
        } => result_for_assistant.clone().unwrap_or_default(),
        northhing_core::agentic::core::MessageContent::Mixed { text, .. } => text.clone(),
    };

    MessageItem {
        id: SharedString::from(msg.id.clone()),
        role: SharedString::from(role),
        content: SharedString::from(content),
        timestamp: SharedString::from(format_time(msg.timestamp)),
        is_streaming,
        // Phase 4: tool call fields. Default to no tool calls; Rust
        // will populate these when extracting tool_call records from
        // message content (Phase 5).
        tool_calls_count: 0,
        tool_calls_summary: SharedString::from(""),
        tool_calls_json: SharedString::from(""),
    }
}

/// Build a Slint ModelRc<SessionItem> from a list of summaries
pub(super) fn build_sessions_model(
    summaries: &[northhing_core::agentic::core::SessionSummary],
) -> ModelRc<SessionItem> {
    // Phase C.2: compute each session's depth in the subagent tree. The
    // tree can in principle be unbounded, but a hard cap protects the UI
    // from pathological data (e.g. a cycle created by a corrupt session).
    const MAX_DEPTH: i32 = 8;

    let items: Vec<SessionItem> = summaries.iter().map(session_summary_to_item).collect();

    // First pass: build id -> parent_id lookup.
    let parent_of: std::collections::HashMap<&str, &str> = items
        .iter()
        .filter(|item| !item.parent_id.is_empty())
        .map(|item| (item.id.as_str(), item.parent_id.as_str()))
        .collect();

    // Second pass: walk parent links for each item, bounded by MAX_DEPTH.
    // Cycles resolve to MAX_DEPTH (the walker stops when it revisits a
    // session id it has already seen on the current chain).
    let depths: Vec<i32> = items
        .iter()
        .map(|item| {
            let mut depth: i32 = 0;
            let mut current = item.id.to_string();
            let mut seen = std::collections::HashSet::new();
            seen.insert(current.clone());
            while let Some(parent_id) = parent_of.get(current.as_str()) {
                if !seen.insert((*parent_id).to_string()) {
                    depth = MAX_DEPTH;
                    break;
                }
                depth += 1;
                if depth >= MAX_DEPTH {
                    break;
                }
                current = (*parent_id).to_string();
            }
            depth
        })
        .collect();

    let items: Vec<SessionItem> = items
        .into_iter()
        .zip(depths)
        .map(|(mut item, depth)| {
            item.depth = depth;
            item
        })
        .collect();

    ModelRc::new(VecModel::from(items))
}

/// Build a Slint ModelRc<MessageItem> from a list of messages
/// A7: `streaming_session_id` marks the last assistant message as streaming
/// when it matches the session being viewed.
pub(super) fn build_messages_model(
    messages: &[northhing_core::agentic::core::Message],
    streaming_session_id: Option<&str>,
) -> ModelRc<MessageItem> {
    let items: Vec<MessageItem> = messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            // A streaming indicator is shown on the last message only
            // when the session is actively streaming and the message is
            // an assistant message.
            let is_last = idx == messages.len().saturating_sub(1);
            let is_assistant = matches!(msg.role, northhing_core::agentic::core::MessageRole::Assistant);
            let is_streaming = streaming_session_id.is_some() && is_last && is_assistant;
            message_to_item(msg, is_streaming)
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}
/// Refresh the sessions list in the UI
pub(super) async fn refresh_sessions_ui(ui: &AppWindow, current_session_id: &str) {
    let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
        return;
    };

    let workspace = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    match coordinator.list_sessions(std::path::Path::new(&workspace)).await {
        Ok(sessions) => {
            let model = build_sessions_model(&sessions);
            ui.set_sessions(model);
            if !current_session_id.is_empty() {
                ui.set_current_session_id(SharedString::from(current_session_id.to_string()));
            }
        }
        Err(e) => crate::app_state::set_session_error(ui, format!("Failed to list sessions: {e}")),
    }
}

/// Refresh the messages list in the UI for a given session
/// A7: `streaming_session_id` marks the last assistant message as streaming
pub(super) async fn refresh_messages_ui(ui: &AppWindow, session_id: &str, streaming_session_id: Option<&str>) {
    let Some(coordinator) = northhing_core::agentic::coordination::global_coordinator() else {
        return;
    };

    if session_id.is_empty() {
        ui.set_messages(ModelRc::new(VecModel::from(Vec::<MessageItem>::new())));
        return;
    }

    match coordinator.get_messages(session_id).await {
        Ok(messages) => {
            let model = build_messages_model(&messages, streaming_session_id);
            ui.set_messages(model);
        }
        Err(e) => crate::app_state::set_session_error(ui, format!("Failed to get messages: {e}")),
    }
}
