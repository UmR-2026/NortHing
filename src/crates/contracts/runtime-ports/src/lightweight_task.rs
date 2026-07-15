//! Lightweight task / actor dispatch port — Phase 1 stub.
//!
//! Pattern source: `.agents/reference/actor/02-tool-dispatcher-trait.rs` (design
//! doc, 2026-06-19). The plan for Phase 1 is **port only**: define the trait
//! shape so the agent-dispatch crate can compile against it, but defer the
//! production body (and the `SkillActor` companion trait) to Phase 2 of
//! `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`.
//!
//! **Status: PORT-ONLY.** No behavior ships behind this trait in Phase 1. The
//! `agent-dispatch` crate exposes four const flags (all default `false`) that
//! gate when the production dispatcher is allowed to take over from the
//! existing `ConversationCoordinator::execute_hidden_subagent_internal` path.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// Minimal telemetry boundary that this port requires. The producer side
/// (`northhing-agent-dispatch::telemetry::TelemetrySink`) has a richer event
/// enum; the consumer side here only needs to know "an event happened with
/// this stable string id". This is the one-way boundary the runtime-ports
/// crate allows: the dispatcher depends on a port-shape trait, the agent
/// runtime is free to implement it with any richer backend.
pub trait LightweightTelemetrySink: Send + Sync + std::fmt::Debug {
    /// Stable, lowercase, machine-friendly event identifier (e.g. `"dispatch_completed"`).
    /// Phase 1 ports never inspect the rest of the event; future phases may
    /// widen the boundary if more structure is required.
    fn emit_event(&self, event_kind: &'static str);
}

/// A single dispatch request — the input shape for
/// [`ToolDispatcherPort::dispatch_once`].
///
/// Field semantics mirror `.agents/reference/actor/02-tool-dispatcher-trait.rs`:
/// one-shot only. Multi-round LLM ↔ tool loops must continue to use
/// `ConversationCoordinator::execute_hidden_subagent_internal` at
/// `coordinator.rs:4173` (see `.agents/reference/actor/NOTES.md` ⛔ #2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightweightTaskRequest {
    pub dispatch_id: String,
    pub user_prompt: String,
    #[serde(default)]
    pub prepended_context: Vec<String>,
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    #[serde(default)]
    pub timeout: Option<Duration>,
    #[serde(skip)]
    pub cancel: Option<CancellationToken>,
    /// Caller-provided telemetry sink. The dispatcher must treat this as
    /// required: refusing to emit would hide actor failures. Phase 1 ships a
    /// `NoopTelemetrySink` as the default — see
    /// `northhing-agent-dispatch::telemetry::NoopTelemetrySink`.
    #[serde(skip)]
    pub telemetry: Option<Arc<dyn LightweightTelemetrySink>>,
}

/// The output of a one-shot dispatch.
///
/// `NoToolMatched` and `Cancelled` are first-class — they are not errors and
/// must be propagated as-is to the caller. Only the `Backend` arm is an
/// error condition that callers should retry on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LightweightTaskOutput {
    /// The dispatcher matched a tool and produced a structured result.
    #[serde(rename = "toolResult")]
    ToolResult {
        #[serde(rename = "toolName")]
        tool_name: String,
        output: String,
    },
    /// No tool matched the request (e.g. empty allowlist, no plugin found).
    /// The caller may fall back to a regular subagent path.
    #[serde(rename = "noToolMatched")]
    NoToolMatched { reason: String },
    /// The dispatch was cancelled before completion (via `req.cancel`).
    #[serde(rename = "cancelled")]
    Cancelled,
    /// The dispatch exceeded its timeout.
    #[serde(rename = "timeout")]
    Timeout,
    /// The backend (LLM, tool provider, etc.) returned an error.
    #[serde(rename = "backend")]
    Backend { message: String },
}

/// Port trait for the one-shot dispatcher.
///
/// Implementations must be `Send + Sync` and non-blocking on the call path.
/// Per `.agents/reference/actor/NOTES.md` ⛔ #2, implementations must NOT
/// add a `multi_round: bool` parameter; multi-round work goes through the
/// coordinator path, not through this port.
#[async_trait::async_trait]
pub trait ToolDispatcherPort: Send + Sync {
    /// Dispatch a single one-shot request and await its result.
    async fn dispatch_once(&self, req: LightweightTaskRequest) -> LightweightTaskOutput;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The request shape must round-trip through JSON (callers on the other
    /// side of the port boundary expect stable wire format).
    #[test]
    fn request_round_trips_with_stable_camel_case() {
        let req = LightweightTaskRequest {
            dispatch_id: "d-1".into(),
            user_prompt: "find all *.rs".into(),
            prepended_context: vec!["ctx".into()],
            tool_allowlist: vec!["file_search".into()],
            timeout: Some(Duration::from_secs(30)),
            cancel: None,
            telemetry: None,
        };

        let json = serde_json::to_value(&req).expect("serialize request");
        assert_eq!(json["dispatchId"], "d-1");
        assert_eq!(json["userPrompt"], "find all *.rs");
        assert_eq!(json["prependedContext"][0], "ctx");
        assert_eq!(json["toolAllowlist"][0], "file_search");
        assert_eq!(json["timeout"]["secs"], 30);
        // `cancel` and `telemetry` are intentionally skipped — they are not
        // wire-payload. Verify they round-trip as null when serialized.
        assert!(json.get("cancel").is_none());
        assert!(json.get("telemetry").is_none());
    }

    #[test]
    fn output_tag_is_stable() {
        let matched = LightweightTaskOutput::ToolResult {
            tool_name: "file_search".into(),
            output: "ok".into(),
        };
        let json = serde_json::to_value(&matched).expect("serialize matched");
        assert_eq!(json["kind"], "toolResult");
        assert_eq!(json["toolName"], "file_search");
        assert_eq!(json["output"], "ok");

        let no_match = LightweightTaskOutput::NoToolMatched {
            reason: "empty allowlist".into(),
        };
        let json = serde_json::to_value(&no_match).expect("serialize no-match");
        assert_eq!(json["kind"], "noToolMatched");
        assert_eq!(json["reason"], "empty allowlist");
    }

    /// Sanity-check the port's behavior contract via a trivial impl.
    #[tokio::test]
    async fn port_trait_is_implementable() {
        struct Echo;
        #[async_trait::async_trait]
        impl ToolDispatcherPort for Echo {
            async fn dispatch_once(&self, req: LightweightTaskRequest) -> LightweightTaskOutput {
                LightweightTaskOutput::ToolResult {
                    tool_name: "echo".into(),
                    output: req.user_prompt,
                }
            }
        }

        let dispatcher: Arc<dyn ToolDispatcherPort> = Arc::new(Echo);
        let out = dispatcher
            .dispatch_once(LightweightTaskRequest {
                dispatch_id: "d".into(),
                user_prompt: "hi".into(),
                prepended_context: vec![],
                tool_allowlist: vec![],
                timeout: None,
                cancel: None,
                telemetry: None,
            })
            .await;
        assert_eq!(
            out,
            LightweightTaskOutput::ToolResult {
                tool_name: "echo".into(),
                output: "hi".into(),
            }
        );
    }
}
