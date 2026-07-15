// REFERENCE — extracted from
//   docs/superpowers/specs/2026-06-18-lightweight-actor-design.md (lines 101-135)
// Last synced: 2026-06-19 (design doc only)
// DESIGN DOC — NOT IMPLEMENTED.

#![allow(dead_code)]

//! ToolDispatcher trait — designed but NOT implemented.
//!
//! One-shot subagent path: 1 LLM call → pick 1 tool → execute → return.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agentic::tools::ToolCall;
use crate::agentic::tools::ToolResult;

pub trait TelemetrySink: Send + Sync {
    fn record(&self, event: DispatchTelemetryEvent);
}

pub enum DispatchTelemetryEvent {
    DispatchStarted { dispatch_id: String, tool: String },
    DispatchCompleted { dispatch_id: String, tool: String, duration: Duration },
    DispatchTimedOut { dispatch_id: String, tool: String },
    DispatchCancelled { dispatch_id: String, tool: String },
    NoToolMatched { dispatch_id: String, prompt_chars: usize },
}

/// A single dispatch request. The runtime fills these in; the dispatcher
/// executes the LLM call, picks a tool, runs it, and returns the result.
pub struct DispatchRequest {
    pub dispatch_id: String,
    /// The user's prompt (or a synthesized prompt for an actor).
    pub user_prompt: String,
    /// Optional prepended context (e.g. system message).
    pub prepended_context: Vec<String>,
    /// Tool allowlist; the LLM may only pick from this set.
    pub tool_allowlist: Vec<String>,
    /// Per-dispatch timeout (default 60s for one-shot).
    pub timeout: Duration,
    pub cancel: CancellationToken,
    pub telemetry: Arc<dyn TelemetrySink>,
}

/// Output of a single dispatch. Exactly one variant is returned.
pub enum DispatchOutput {
    /// The LLM picked a tool and it ran. Contains the tool result.
    ToolResult(ToolResult),
    /// The LLM didn't pick a tool (refused, said no, etc.). Treat as soft
    /// failure; do not retry automatically.
    NoToolMatched { reason: String },
    /// The dispatch was cancelled (caller or runtime).
    Cancelled,
    /// The dispatch exceeded its timeout.
    Timeout,
}

#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    /// ★ Single dispatch. One LLM call, one tool call, one result.
    async fn dispatch_once(&self, req: DispatchRequest) -> DispatchOutput;
}

// ═══════════════════════════════════════════════════════════════════════
// NOTE: This trait is for ONE-SHOT work only. Multi-round loops
// (LLM → tool → LLM → tool ...) go through the existing
// `ConversationCoordinator::execute_hidden_subagent_internal` at
// coordinator.rs:4173. Do not extend this trait to support multi-round.
// ═══════════════════════════════════════════════════════════════════════
