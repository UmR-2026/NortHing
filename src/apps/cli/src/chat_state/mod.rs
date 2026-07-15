// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chat state module (facade).
//!
//! Pure UI rendering state for the chat interface.
//! All session lifecycle and persistence is handled by northhing-core.
//! This module only maintains transient state needed for TUI rendering.
//!
//! Split into siblings (R39b):
//! - [`display_types`]: pure data types and `ChatMessage::from_core_message`
//! - [`helpers`]: free helper functions for tool result summarization
//! - [`chat_state_core`]: [`ChatState`] struct, lifecycle, streaming, subagent forwarding
//! - [`chat_state_tool_events`]: `handle_tool_event` dispatch

mod chat_state_core;
mod chat_state_tool_events;
mod display_types;
mod helpers;

// Wildcard re-exports — preserve the original flat module's public API.
// Consumers (modes/chat/*.rs, ui/chat/state.rs, ui/tool_cards.rs, etc.) import
// these names directly via `use crate::chat_state::ChatState`.
pub use chat_state_core::ChatState;
pub use display_types::{ChatMessage, FlowItem, MessageRole, ToolDisplayState, ToolDisplayStatus};
