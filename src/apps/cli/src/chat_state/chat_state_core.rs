// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chat state core: lifecycle, streaming, subagent forwarding.
//!
//! - [`ChatState`] struct and constructors (`new`, `from_core_messages`)
//! - Turn lifecycle: `handle_turn_started`, `handle_turn_completed`,
//!   `handle_turn_failed`, `handle_turn_cancelled`
//! - Streaming chunks: `handle_text_chunk`, `handle_thinking_chunk`
//! - Subagent progress forwarding: `handle_subagent_event`
//! - Misc: `handle_token_usage`, `add_system_message`,
//!   `add_assistant_message`, `clear_messages`, `current_turn_id`
//! - Private helpers: `rebuild_streaming_message`, `insert_or_update_tool`,
//!   `update_tool`

use std::collections::HashMap;
use std::time::SystemTime;

use northhing_core::agentic::core::message::{Message as CoreMessage, MessageContent, MessageRole as CoreMessageRole};
use northhing_core::agentic::core::strip_prompt_markup;
use northhing_events::ToolEventData;

use crate::ui::permission::PermissionPrompt;
use crate::ui::question::QuestionPrompt;

use super::display_types::{ChatMessage, ChatMetadata, FlowItem, MessageRole};
use super::display_types::{SubagentProgress, ToolDisplayState, ToolDisplayStatus};
use super::helpers::{extract_fallback_summary, extract_tool_title, truncate_string};

// ============ ChatState ============

/// Complete UI state for the chat interface.
/// This is the single source of truth for rendering — but NOT for persistence.
/// All persistence is handled by northhing-core's SessionManager.
pub struct ChatState {
    /// Core session ID (the real session managed by core)
    pub core_session_id: String,
    /// Session display name
    pub session_name: String,
    /// Agent type
    pub agent_type: String,
    /// Workspace path
    pub workspace: Option<String>,
    /// Current model display name (shown in shortcuts bar)
    pub current_model_name: String,
    /// Messages for UI rendering
    pub messages: Vec<ChatMessage>,
    /// Session statistics
    pub metadata: ChatMetadata,

    // -- Streaming state (transient, not persisted) --
    /// Current turn ID being processed
    pub(super) current_turn_id: Option<String>,
    /// Ordered flow items for the current streaming message.
    /// Text, thinking, and tool blocks are interleaved in chronological order,
    /// matching the actual conversation flow (inspired by opencode's Part model).
    pub(super) current_flow_items: Vec<FlowItem>,
    /// Index from tool_id to position in current_flow_items (for fast in-place updates)
    pub(super) tool_index: HashMap<String, usize>,
    /// Whether the assistant is currently processing
    pub is_processing: bool,

    // -- Permission state --
    /// Current pending permission prompt (if a tool needs user confirmation)
    pub permission_prompt: Option<PermissionPrompt>,

    // -- Question state --
    /// Current pending question prompt (if AskUserQuestion tool is waiting for answers)
    pub question_prompt: Option<QuestionPrompt>,
}

impl ChatState {
    /// Create a new ChatState for a fresh session
    pub fn new(core_session_id: String, session_name: String, agent_type: String, workspace: Option<String>) -> Self {
        Self {
            core_session_id,
            session_name,
            agent_type,
            workspace,
            current_model_name: String::new(),
            messages: Vec::new(),
            metadata: ChatMetadata::default(),
            current_turn_id: None,
            current_flow_items: Vec::new(),
            tool_index: HashMap::new(),
            is_processing: false,
            permission_prompt: None,
            question_prompt: None,
        }
    }

    /// Load historical messages from core and create ChatState.
    ///
    /// Tool results (ToolResult messages) are merged back into the corresponding
    /// tool calls (in Mixed messages) so that tool cards render with full result data.
    pub fn from_core_messages(
        core_session_id: String,
        session_name: String,
        agent_type: String,
        workspace: Option<String>,
        core_messages: &[CoreMessage],
    ) -> Self {
        // Step 1: Build tool_id -> (result_summary, metadata, is_error) lookup from ToolResult messages
        let mut tool_results: HashMap<String, (String, Option<serde_json::Value>, bool)> = HashMap::new();
        for msg in core_messages {
            if let MessageContent::ToolResult {
                tool_id,
                result,
                is_error,
                ..
            } = &msg.content
            {
                let result_str = extract_fallback_summary(result);
                tool_results.insert(tool_id.clone(), (result_str, Some(result.clone()), *is_error));
            }
        }

        // Step 2: Convert messages, merging tool results into tool call display states
        let messages: Vec<ChatMessage> = core_messages
            .iter()
            .filter(|msg| {
                // Skip tool result messages (merged into tool cards above)
                !matches!(msg.role, CoreMessageRole::Tool)
                // Skip system messages (internal)
                && !matches!(msg.role, CoreMessageRole::System)
            })
            .map(|msg| {
                let mut chat_msg = ChatMessage::from_core_message(msg);
                // Merge tool results into corresponding tool display states
                for item in &mut chat_msg.flow_items {
                    if let FlowItem::Tool { tool_state } = item {
                        if let Some((result_str, metadata, is_error)) = tool_results.get(&tool_state.tool_id) {
                            tool_state.result = Some(result_str.clone());
                            tool_state.metadata = metadata.clone();
                            if *is_error {
                                tool_state.status = ToolDisplayStatus::Failed;
                            }
                        }
                    }
                }
                chat_msg
            })
            .collect();

        let tool_count = tool_results.len();

        let mut state = Self::new(core_session_id, session_name, agent_type, workspace);
        state.metadata.message_count = messages.len();
        state.metadata.tool_calls = tool_count;
        state.messages = messages;
        state
    }

    // ============ Event Handlers ============

    /// Handle the start of a new dialog turn
    pub fn handle_turn_started(&mut self, turn_id: &str, user_input: &str) {
        self.current_turn_id = Some(turn_id.to_string());
        self.current_flow_items.clear();
        self.tool_index.clear();
        self.is_processing = true;
        let user_display_input = strip_prompt_markup(user_input);

        // Add user message
        self.messages.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            timestamp: SystemTime::now(),
            flow_items: vec![FlowItem::Text {
                content: user_display_input,
                is_streaming: false,
            }],
            is_streaming: false,
            version: 0,
        });
        self.metadata.message_count += 1;

        // Add empty assistant message (will be filled by streaming)
        self.messages.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            timestamp: SystemTime::now(),
            flow_items: Vec::new(),
            is_streaming: true,
            version: 0,
        });
    }

    /// Handle a text chunk from the AI.
    /// Appends to the last Text flow item if it exists, otherwise creates a new one.
    /// This ensures text and tool blocks remain interleaved in chronological order.
    pub fn handle_text_chunk(&mut self, text: &str) {
        // Try to append to the last flow item if it's a Text block
        if let Some(FlowItem::Text { content, .. }) = self.current_flow_items.last_mut() {
            content.push_str(text);
        } else {
            // Last item is not Text (it's a Tool, Thinking, or empty) — create a new Text block
            self.current_flow_items.push(FlowItem::Text {
                content: text.to_string(),
                is_streaming: true,
            });
        }
        self.rebuild_streaming_message();
    }

    /// Handle a thinking/reasoning chunk from the AI.
    /// Thinking blocks typically appear at the start, before text/tool content.
    /// Appends to the last Thinking flow item if it exists, otherwise creates a new one.
    pub fn handle_thinking_chunk(&mut self, content: &str) {
        // Try to append to the last Thinking block
        // (Thinking usually comes before text, so check the last item)
        let appended = if let Some(FlowItem::Thinking { content: existing }) = self.current_flow_items.last_mut() {
            existing.push_str(content);
            true
        } else {
            false
        };

        if !appended {
            // Also check if there's a Thinking block earlier that we should append to
            // (e.g., if a Text block was inserted after Thinking but more thinking arrives)
            // For simplicity, just create a new Thinking block — this is rare in practice
            self.current_flow_items.push(FlowItem::Thinking {
                content: content.to_string(),
            });
        }
        self.rebuild_streaming_message();
    }

    /// Handle a subagent event by updating the parent Task tool's progress.
    ///
    /// When a subagent emits events (tool started, completed, etc.), we forward
    /// key information to the parent Task tool so the UI can show real-time progress.
    pub fn handle_subagent_event(&mut self, parent_tool_id: &str, event: &northhing_events::AgenticEvent) {
        use northhing_events::AgenticEvent;

        match event {
            AgenticEvent::ToolEvent { tool_event, .. } => match tool_event {
                ToolEventData::Started { tool_name, params, .. } => {
                    let title = extract_tool_title(tool_name, params);
                    self.update_tool(parent_tool_id, |tool| {
                        let progress = tool.subagent_progress.get_or_insert_with(SubagentProgress::default);
                        progress.tool_count += 1;
                        progress.current_tool_name = Some(tool_name.clone());
                        progress.current_tool_title = title;
                    });
                    self.rebuild_streaming_message();
                }
                ToolEventData::Completed {
                    tool_name,
                    result_for_assistant,
                    result: _,
                    ..
                } => {
                    let summary = result_for_assistant.clone().unwrap_or_else(|| tool_name.clone());
                    self.update_tool(parent_tool_id, |tool| {
                        let progress = tool.subagent_progress.get_or_insert_with(SubagentProgress::default);
                        progress.current_tool_name = Some(tool_name.clone());
                        progress.current_tool_title = Some(summary);
                    });
                    self.rebuild_streaming_message();
                }
                ToolEventData::Failed { tool_name, error, .. } => {
                    self.update_tool(parent_tool_id, |tool| {
                        let progress = tool.subagent_progress.get_or_insert_with(SubagentProgress::default);
                        progress.current_tool_name = Some(tool_name.clone());
                        progress.current_tool_title = Some(format!("Error: {}", truncate_string(error, 60)));
                    });
                    self.rebuild_streaming_message();
                }
                _ => {}
            },
            AgenticEvent::ModelRoundStarted { round_index, .. } => {
                if *round_index > 0 {
                    self.update_tool(parent_tool_id, |tool| {
                        let progress = tool.subagent_progress.get_or_insert_with(SubagentProgress::default);
                        progress.current_tool_name = None;
                        progress.current_tool_title = Some(format!("Round {}", round_index + 1));
                    });
                    self.rebuild_streaming_message();
                }
            }
            _ => {}
        }
    }

    /// Handle dialog turn completion
    pub fn handle_turn_completed(&mut self, total_rounds: usize, _total_tools: usize) {
        // Finalize the streaming message
        if let Some(last_msg) = self.messages.last_mut() {
            if last_msg.role == MessageRole::Assistant {
                last_msg.is_streaming = false;
                // Mark all text flow items as not streaming
                for item in &mut last_msg.flow_items {
                    if let FlowItem::Text { is_streaming, .. } = item {
                        *is_streaming = false;
                    }
                }
                last_msg.version += 1;
            }
        }

        self.metadata.total_rounds += total_rounds;
        self.current_turn_id = None;
        self.current_flow_items.clear();
        self.tool_index.clear();
        self.is_processing = false;
        self.permission_prompt = None;
        self.question_prompt = None;
    }

    /// Handle dialog turn failure
    pub fn handle_turn_failed(&mut self, error: &str) {
        // Add error to the last assistant message
        if let Some(last_msg) = self.messages.last_mut() {
            if last_msg.role == MessageRole::Assistant {
                last_msg.is_streaming = false;
                last_msg.flow_items.push(FlowItem::Text {
                    content: format!("[Error: {}]", error),
                    is_streaming: false,
                });
                last_msg.version += 1;
            }
        }

        self.current_turn_id = None;
        self.current_flow_items.clear();
        self.tool_index.clear();
        self.is_processing = false;
        self.permission_prompt = None;
        self.question_prompt = None;
    }

    /// Handle dialog turn cancellation
    pub fn handle_turn_cancelled(&mut self) {
        if let Some(last_msg) = self.messages.last_mut() {
            if last_msg.role == MessageRole::Assistant {
                last_msg.is_streaming = false;
                last_msg.flow_items.push(FlowItem::Text {
                    content: "[Cancelled]".to_string(),
                    is_streaming: false,
                });
                last_msg.version += 1;
            }
        }

        self.current_turn_id = None;
        self.current_flow_items.clear();
        self.tool_index.clear();
        self.is_processing = false;
        self.permission_prompt = None;
        self.question_prompt = None;
    }

    /// Handle token usage update
    pub fn handle_token_usage(&mut self, total_tokens: usize) {
        self.metadata.total_tokens = total_tokens;
    }

    /// Add a system message (for commands like /help, /clear, etc.)
    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            timestamp: SystemTime::now(),
            flow_items: vec![FlowItem::Text {
                content,
                is_streaming: false,
            }],
            is_streaming: false,
            version: 0,
        });
    }

    /// Add a local assistant message (for rendered reports and other UI-only content).
    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            timestamp: SystemTime::now(),
            flow_items: vec![FlowItem::Text {
                content,
                is_streaming: false,
            }],
            is_streaming: false,
            version: 0,
        });
    }

    /// Clear all messages (for /clear command)
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Get the current turn ID (if processing)
    pub fn current_turn_id(&self) -> Option<&str> {
        self.current_turn_id.as_deref()
    }

    // ============ Internal ============

    /// Rebuild the last assistant message from current streaming state.
    /// Simply clones the chronologically-ordered current_flow_items into the message.
    /// Text, thinking, and tool blocks are already interleaved in the correct order.
    pub(super) fn rebuild_streaming_message(&mut self) {
        let last_msg = match self.messages.last_mut() {
            Some(msg) if msg.role == MessageRole::Assistant && msg.is_streaming => msg,
            _ => return,
        };

        last_msg.flow_items = self.current_flow_items.clone();
        last_msg.version += 1;
    }

    /// Insert a new tool into current_flow_items (appended at end, preserving chronological order),
    /// or update an existing tool in-place if it already exists.
    pub(super) fn insert_or_update_tool(
        &mut self,
        tool_id: &str,
        update_fn: impl FnOnce(&mut ToolDisplayState),
        create_fn: impl FnOnce() -> ToolDisplayState,
    ) {
        if let Some(&idx) = self.tool_index.get(tool_id) {
            // Tool already exists — update in-place
            if let Some(FlowItem::Tool { tool_state }) = self.current_flow_items.get_mut(idx) {
                update_fn(tool_state);
            }
        } else {
            // New tool — append to flow items in chronological order
            let new_state = create_fn();
            let idx = self.current_flow_items.len();
            self.current_flow_items.push(FlowItem::Tool { tool_state: new_state });
            self.tool_index.insert(tool_id.to_string(), idx);
        }
    }

    /// Update an existing tool in current_flow_items via tool_index.
    /// No-op if the tool_id is not found (defensive).
    pub(super) fn update_tool(&mut self, tool_id: &str, update_fn: impl FnOnce(&mut ToolDisplayState)) {
        if let Some(&idx) = self.tool_index.get(tool_id) {
            if let Some(FlowItem::Tool { tool_state }) = self.current_flow_items.get_mut(idx) {
                update_fn(tool_state);
            }
        }
    }
}
