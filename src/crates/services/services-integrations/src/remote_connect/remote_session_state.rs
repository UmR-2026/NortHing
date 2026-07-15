//! Remote-connect session live-state plumbing (Round 11b split).
//!
//! Owns the `RemoteSessionStateTracker` and `RemoteSessionTrackerRegistry`
//! types that power remote polling and live diff streams. The struct impl
//! lives here because all 30+ accessor methods plus the 360-line
//! `handle_agentic_event` body share the same `RwLock<TrackerState>` interior.
//!
//! Sub-domain split (R11b, QClaw R11b REQUIRED):
//! - `remote_session_state.rs` (~700): state management (this file)
//! - `remote_session_response_builders.rs` (~700): DTOs + response builders +
//!   session/poll command handlers
//!
//! Cross-sibling visibility:
//! - `TrackerState` stays private here (only the impl block needs it).
//! - `TrackerEvent`, `RemoteSessionStateTracker`, `RemoteSessionTrackerHost`,
//!   `RemoteSessionTrackerRegistry` are `pub` and re-exported by `mod.rs`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use northhing_events::AgenticEvent;
use serde::{Deserialize, Serialize};

use super::remote_request_builders::{make_slim_tool_params, ChatMessageItem, RemoteToolStatus};

/// Snapshot of the in-flight turn for poll responses. Lives with the state
/// file because it is read out of `RemoteSessionStateTracker` and only
/// re-serialized by `remote_session_response_builders.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveTurnSnapshot {
    pub turn_id: String,
    pub status: String,
    pub text: String,
    pub thinking: String,
    pub tools: Vec<RemoteToolStatus>,
    pub round_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ChatMessageItem>>,
}

#[derive(Debug)]
struct TrackerState {
    session_state: String,
    title: String,
    turn_id: Option<String>,
    turn_status: String,
    accumulated_text: String,
    accumulated_thinking: String,
    active_tools: Vec<RemoteToolStatus>,
    round_index: usize,
    active_items: Vec<ChatMessageItem>,
    persistence_dirty: bool,
    linked_subagent_sessions: HashMap<String, String>,
}

/// Lightweight event broadcast by the tracker for real-time consumers.
#[derive(Debug, Clone, PartialEq)]
pub enum TrackerEvent {
    TextChunk(String),
    ThinkingChunk(String),
    ThinkingEnd,
    ToolStarted {
        tool_id: String,
        tool_name: String,
        params: Option<serde_json::Value>,
    },
    ToolCompleted {
        tool_id: String,
        tool_name: String,
        duration_ms: Option<u64>,
        success: bool,
    },
    TurnCompleted {
        turn_id: String,
    },
    TurnFailed {
        turn_id: String,
        error: String,
    },
    TurnCancelled {
        turn_id: String,
    },
}

/// Tracks the real-time state of a session for remote polling and bot streams.
pub struct RemoteSessionStateTracker {
    target_session_id: String,
    version: AtomicU64,
    state: RwLock<TrackerState>,
    event_tx: tokio::sync::broadcast::Sender<TrackerEvent>,
}

impl RemoteSessionStateTracker {
    pub fn new(session_id: String) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            target_session_id: session_id,
            version: AtomicU64::new(0),
            state: RwLock::new(TrackerState {
                session_state: "idle".to_string(),
                title: String::new(),
                turn_id: None,
                turn_status: String::new(),
                accumulated_text: String::new(),
                accumulated_thinking: String::new(),
                active_tools: Vec::new(),
                round_index: 0,
                active_items: Vec::new(),
                persistence_dirty: true,
                linked_subagent_sessions: HashMap::new(),
            }),
            event_tx,
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TrackerEvent> {
        self.event_tx.subscribe()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot_active_turn(&self) -> Option<ActiveTurnSnapshot> {
        let state = self.state.read().unwrap();
        let has_items = !state.active_items.is_empty();
        state.turn_id.as_ref().map(|turn_id| ActiveTurnSnapshot {
            turn_id: turn_id.clone(),
            status: state.turn_status.clone(),
            text: if has_items {
                String::new()
            } else {
                state.accumulated_text.clone()
            },
            thinking: if has_items {
                String::new()
            } else {
                state.accumulated_thinking.clone()
            },
            tools: state.active_tools.clone(),
            round_index: state.round_index,
            items: if has_items {
                Some(state.active_items.clone())
            } else {
                None
            },
        })
    }

    pub fn session_state(&self) -> String {
        self.state.read().unwrap().session_state.clone()
    }

    pub fn title(&self) -> String {
        self.state.read().unwrap().title.clone()
    }

    pub fn turn_status(&self) -> String {
        self.state.read().unwrap().turn_status.clone()
    }

    pub fn accumulated_text(&self) -> String {
        self.state.read().unwrap().accumulated_text.clone()
    }

    pub fn accumulated_thinking(&self) -> String {
        self.state.read().unwrap().accumulated_thinking.clone()
    }

    pub fn is_turn_finished(&self) -> bool {
        let state = self.state.read().unwrap();
        state.turn_id.is_some() && matches!(state.turn_status.as_str(), "completed" | "failed" | "cancelled")
    }

    pub fn initialize_active_turn(&self, turn_id: String) {
        let mut state = self.state.write().unwrap();
        if state.turn_id.is_none() {
            state.turn_id = Some(turn_id);
            state.turn_status = "active".to_string();
            state.session_state = "running".to_string();
        }
        drop(state);
        self.bump_version();
    }

    pub fn finalize_completed_turn(&self) {
        let mut state = self.state.write().unwrap();
        if matches!(state.turn_status.as_str(), "completed" | "failed" | "cancelled") {
            state.turn_id = None;
            state.accumulated_text.clear();
            state.accumulated_thinking.clear();
            state.active_tools.clear();
            state.active_items.clear();
        }
    }

    pub fn is_persistence_dirty(&self) -> bool {
        self.state.read().unwrap().persistence_dirty
    }

    pub fn mark_persistence_clean(&self) {
        self.state.write().unwrap().persistence_dirty = false;
    }

    fn find_mergeable_item(
        items: &[ChatMessageItem],
        target_type: &str,
        subagent_marker: &Option<bool>,
    ) -> Option<usize> {
        for index in (0..items.len()).rev() {
            let item = &items[index];
            if item.item_type == "tool" {
                return None;
            }
            if item.item_type == target_type && &item.is_subagent == subagent_marker {
                return Some(index);
            }
        }
        None
    }

    fn upsert_active_tool(
        state: &mut TrackerState,
        tool_id: &str,
        tool_name: &str,
        status: &str,
        input_preview: Option<String>,
        tool_input: Option<serde_json::Value>,
        is_subagent: bool,
    ) {
        let resolved_id = if tool_id.is_empty() {
            format!("{}-{}", tool_name, state.active_tools.len())
        } else {
            tool_id.to_string()
        };
        let allow_name_fallback = tool_id.is_empty() && !tool_name.is_empty();
        let subagent_marker = if is_subagent { Some(true) } else { None };

        if let Some(tool) = state
            .active_tools
            .iter_mut()
            .rev()
            .find(|tool| tool.id == resolved_id || (allow_name_fallback && tool.name == tool_name))
        {
            tool.status = status.to_string();
            if input_preview.is_some() {
                tool.input_preview = input_preview.clone();
            }
            if tool_input.is_some() {
                tool.tool_input = tool_input.clone();
            }
        } else {
            let tool_status = RemoteToolStatus {
                id: resolved_id.clone(),
                name: tool_name.to_string(),
                status: status.to_string(),
                duration_ms: None,
                start_ms: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                ),
                input_preview,
                tool_input,
            };
            state.active_tools.push(tool_status.clone());
            state.active_items.push(ChatMessageItem {
                item_type: "tool".to_string(),
                content: None,
                tool: Some(tool_status),
                is_subagent: subagent_marker,
            });
            return;
        }

        if let Some(item) = state.active_items.iter_mut().rev().find(|item| {
            item.item_type == "tool"
                && item
                    .tool
                    .as_ref()
                    .is_some_and(|tool| tool.id == resolved_id || (allow_name_fallback && tool.name == tool_name))
        }) {
            if let Some(tool) = item.tool.as_mut() {
                tool.status = status.to_string();
                if input_preview.is_some() {
                    tool.input_preview = input_preview;
                }
                if tool_input.is_some() {
                    tool.tool_input = tool_input;
                }
            }
        }
    }

    pub fn handle_agentic_event(&self, event: &AgenticEvent) {
        use northhing_events::AgenticEvent as AE;

        if let AE::SubagentSessionLinked {
            session_id,
            parent_session_id,
            ..
        } = event
        {
            if parent_session_id != &self.target_session_id {
                return;
            }

            let mut state = self.state.write().unwrap();
            state
                .linked_subagent_sessions
                .insert(session_id.clone(), parent_session_id.clone());
            drop(state);
            self.bump_version();
            return;
        }

        let is_direct = event.session_id() == Some(self.target_session_id.as_str());
        let is_subagent = if !is_direct {
            match event {
                AE::TextChunk { session_id, .. }
                | AE::ThinkingChunk { session_id, .. }
                | AE::ToolEvent { session_id, .. } => self
                    .state
                    .read()
                    .unwrap()
                    .linked_subagent_sessions
                    .get(session_id)
                    .is_some_and(|parent_session_id| parent_session_id == &self.target_session_id),
                _ => false,
            }
        } else {
            false
        };

        if !is_direct && !is_subagent {
            return;
        }

        match event {
            AE::TextChunk { text, .. } => {
                let subagent_marker = if is_subagent { Some(true) } else { None };
                let mut state = self.state.write().unwrap();
                if !is_subagent {
                    state.accumulated_text.push_str(text);
                }
                if let Some(index) = Self::find_mergeable_item(&state.active_items, "text", &subagent_marker) {
                    let item = &mut state.active_items[index];
                    item.content.get_or_insert_with(String::new).push_str(text);
                } else {
                    state.active_items.push(ChatMessageItem {
                        item_type: "text".to_string(),
                        content: Some(text.clone()),
                        tool: None,
                        is_subagent: subagent_marker,
                    });
                }
                drop(state);
                self.bump_version();
                let _ = self.event_tx.send(TrackerEvent::TextChunk(text.clone()));
            }
            AE::ThinkingChunk { content, is_end, .. } => {
                let clean = content.replace("</thinking>", "").replace("<thinking>", "");
                let subagent_marker = if is_subagent { Some(true) } else { None };
                let mut state = self.state.write().unwrap();
                if !is_subagent {
                    state.accumulated_thinking.push_str(&clean);
                }
                if let Some(index) = Self::find_mergeable_item(&state.active_items, "thinking", &subagent_marker) {
                    let item = &mut state.active_items[index];
                    item.content.get_or_insert_with(String::new).push_str(&clean);
                } else {
                    state.active_items.push(ChatMessageItem {
                        item_type: "thinking".to_string(),
                        content: Some(clean),
                        tool: None,
                        is_subagent: subagent_marker,
                    });
                }
                drop(state);
                self.bump_version();
                if *is_end {
                    let _ = self.event_tx.send(TrackerEvent::ThinkingEnd);
                } else if !content.is_empty() {
                    let _ = self.event_tx.send(TrackerEvent::ThinkingChunk(content.clone()));
                }
            }
            AE::ToolEvent { tool_event, .. } => {
                if let Ok(value) = serde_json::to_value(tool_event) {
                    let event_type = value.get("event_type").and_then(|value| value.as_str()).unwrap_or("");
                    let tool_id = value
                        .get("tool_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string();
                    let tool_name = value
                        .get("tool_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string();

                    let mut state = self.state.write().unwrap();
                    let allow_name_fallback = tool_id.is_empty() && !tool_name.is_empty();
                    let mut pending_tool_event: Option<TrackerEvent> = None;
                    match event_type {
                        "EarlyDetected" => {
                            Self::upsert_active_tool(
                                &mut state,
                                &tool_id,
                                &tool_name,
                                "preparing",
                                None,
                                None,
                                is_subagent,
                            );
                        }
                        "ConfirmationNeeded" => {
                            let params = value.get("params").cloned();
                            let input_preview = params.as_ref().and_then(make_slim_tool_params);
                            Self::upsert_active_tool(
                                &mut state,
                                &tool_id,
                                &tool_name,
                                "pending_confirmation",
                                input_preview,
                                params,
                                is_subagent,
                            );
                        }
                        "Started" => {
                            let params = value.get("params").cloned();
                            let input_preview = params.as_ref().and_then(make_slim_tool_params);
                            let tool_input =
                                if tool_name == "AskUserQuestion" || tool_name == "Task" || tool_name == "TodoWrite" {
                                    params.clone()
                                } else {
                                    None
                                };
                            Self::upsert_active_tool(
                                &mut state,
                                &tool_id,
                                &tool_name,
                                "running",
                                input_preview,
                                tool_input,
                                is_subagent,
                            );
                            let _ = self.event_tx.send(TrackerEvent::ToolStarted {
                                tool_id: tool_id.clone(),
                                tool_name: tool_name.clone(),
                                params,
                            });
                        }
                        "Confirmed" => {
                            Self::upsert_active_tool(
                                &mut state,
                                &tool_id,
                                &tool_name,
                                "confirmed",
                                None,
                                None,
                                is_subagent,
                            );
                        }
                        "Rejected" => {
                            Self::upsert_active_tool(
                                &mut state,
                                &tool_id,
                                &tool_name,
                                "rejected",
                                None,
                                None,
                                is_subagent,
                            );
                        }
                        "Completed" | "Succeeded" => {
                            let duration = value.get("duration_ms").and_then(|value| value.as_u64());
                            if let Some(tool) = state.active_tools.iter_mut().rev().find(|tool| {
                                (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                    && tool.status == "running"
                            }) {
                                tool.status = "completed".to_string();
                                tool.duration_ms = duration;
                            }
                            if let Some(item) = state.active_items.iter_mut().rev().find(|item| {
                                item.item_type == "tool"
                                    && item.tool.as_ref().is_some_and(|tool| {
                                        (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                            && tool.status == "running"
                                    })
                            }) {
                                if let Some(tool) = item.tool.as_mut() {
                                    tool.status = "completed".to_string();
                                    tool.duration_ms = duration;
                                }
                            }
                            pending_tool_event = Some(TrackerEvent::ToolCompleted {
                                tool_id: tool_id.clone(),
                                tool_name: tool_name.clone(),
                                duration_ms: duration,
                                success: true,
                            });
                        }
                        "Failed" => {
                            if let Some(tool) = state.active_tools.iter_mut().rev().find(|tool| {
                                (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                    && tool.status == "running"
                            }) {
                                tool.status = "failed".to_string();
                            }
                            if let Some(item) = state.active_items.iter_mut().rev().find(|item| {
                                item.item_type == "tool"
                                    && item.tool.as_ref().is_some_and(|tool| {
                                        (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                            && tool.status == "running"
                                    })
                            }) {
                                if let Some(tool) = item.tool.as_mut() {
                                    tool.status = "failed".to_string();
                                }
                            }
                            pending_tool_event = Some(TrackerEvent::ToolCompleted {
                                tool_id: tool_id.clone(),
                                tool_name: tool_name.clone(),
                                duration_ms: None,
                                success: false,
                            });
                        }
                        "Cancelled" => {
                            if let Some(tool) = state.active_tools.iter_mut().rev().find(|tool| {
                                (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                    && matches!(tool.status.as_str(), "running" | "pending_confirmation" | "confirmed")
                            }) {
                                tool.status = "cancelled".to_string();
                            }
                            if let Some(item) = state.active_items.iter_mut().rev().find(|item| {
                                item.item_type == "tool"
                                    && item.tool.as_ref().is_some_and(|tool| {
                                        (tool.id == tool_id || (allow_name_fallback && tool.name == tool_name))
                                            && matches!(
                                                tool.status.as_str(),
                                                "running" | "pending_confirmation" | "confirmed"
                                            )
                                    })
                            }) {
                                if let Some(tool) = item.tool.as_mut() {
                                    tool.status = "cancelled".to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                    drop(state);
                    self.bump_version();
                    if let Some(event) = pending_tool_event {
                        let _ = self.event_tx.send(event);
                    }
                }
            }
            AE::DialogTurnStarted { turn_id, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.turn_id = Some(turn_id.clone());
                state.turn_status = "active".to_string();
                state.accumulated_text.clear();
                state.accumulated_thinking.clear();
                state.active_tools.clear();
                state.active_items.clear();
                state.round_index = 0;
                state.session_state = "running".to_string();
                state.persistence_dirty = true;
                drop(state);
                self.bump_version();
            }
            AE::DialogTurnCompleted { turn_id, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.turn_status = "completed".to_string();
                state.session_state = "idle".to_string();
                state.persistence_dirty = true;
                drop(state);
                self.bump_version();
                let _ = self.event_tx.send(TrackerEvent::TurnCompleted {
                    turn_id: turn_id.clone(),
                });
            }
            AE::DialogTurnFailed { turn_id, error, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.turn_status = "failed".to_string();
                state.session_state = "idle".to_string();
                state.persistence_dirty = true;
                drop(state);
                self.bump_version();
                let _ = self.event_tx.send(TrackerEvent::TurnFailed {
                    turn_id: turn_id.clone(),
                    error: error.clone(),
                });
            }
            AE::DialogTurnCancelled { turn_id, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.turn_status = "cancelled".to_string();
                state.session_state = "idle".to_string();
                state.persistence_dirty = true;
                drop(state);
                self.bump_version();
                let _ = self.event_tx.send(TrackerEvent::TurnCancelled {
                    turn_id: turn_id.clone(),
                });
            }
            AE::ModelRoundStarted { round_index, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.round_index = *round_index;
                drop(state);
                self.bump_version();
            }
            AE::SessionStateChanged { new_state, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.session_state = new_state.clone();
                drop(state);
                self.bump_version();
            }
            AE::SessionTitleGenerated { title, .. } if is_direct => {
                let mut state = self.state.write().unwrap();
                state.title = title.clone();
                drop(state);
                self.bump_version();
            }
            _ => {}
        }
    }
}

/// Host callbacks required to bind tracker lifecycle to the owning product runtime.
pub trait RemoteSessionTrackerHost {
    fn subscribe_tracker(&self, session_id: &str, tracker: Arc<RemoteSessionStateTracker>);
    fn unsubscribe_tracker(&self, session_id: &str);
    fn active_turn_id(&self, session_id: &str) -> Option<String>;
}

#[derive(Default)]
pub struct RemoteSessionTrackerRegistry {
    state_trackers: RwLock<HashMap<String, Arc<RemoteSessionStateTracker>>>,
}

impl RemoteSessionTrackerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_tracker_with_host<H: RemoteSessionTrackerHost>(
        &self,
        session_id: &str,
        host: &H,
    ) -> Arc<RemoteSessionStateTracker> {
        if let Some(tracker) = self.get_tracker(session_id) {
            return tracker;
        }

        let tracker = {
            let mut trackers = self.state_trackers.write().unwrap();
            if let Some(tracker) = trackers.get(session_id) {
                return tracker.clone();
            }
            let tracker = Arc::new(RemoteSessionStateTracker::new(session_id.to_string()));
            trackers.insert(session_id.to_string(), tracker.clone());
            tracker
        };

        host.subscribe_tracker(session_id, tracker.clone());
        if let Some(active_turn_id) = host.active_turn_id(session_id) {
            tracker.initialize_active_turn(active_turn_id);
        }

        tracker
    }

    pub fn get_tracker(&self, session_id: &str) -> Option<Arc<RemoteSessionStateTracker>> {
        self.state_trackers.read().unwrap().get(session_id).cloned()
    }

    pub fn remove_tracker_with_host<H: RemoteSessionTrackerHost>(
        &self,
        session_id: &str,
        host: &H,
    ) -> Option<Arc<RemoteSessionStateTracker>> {
        let removed = self.state_trackers.write().unwrap().remove(session_id);
        if removed.is_some() {
            host.unsubscribe_tracker(session_id);
        }
        removed
    }
}
