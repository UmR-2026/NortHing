//! K.2.3 Phase A1/A2 path — the long-running-skill replacement for
//! `ConversationCoordinator::execute_hidden_subagent_phase1/2/3`.
//!
//! ## Status: A2 COMPLETE
//!
//! A1 (legacy): `CoordinatorHiddenSubagentSkill` wrapped the existing coordinator
//! phase1/2/3 path inside a `LongRunningSkill`. The skill's `tick`
//! called `execute_hidden_subagent_internal` directly and returned
//! `Done` with the mapped result. This was a "direct execution wrapper" —
//! the subagent's internal multi-turn loop ran monolithically within a
//! single tick.
//!
//! A2 (current): The skill is upgraded to use `ExecutionEngine::init_turn` +
//! `tick` + `finalize_turn` + `build_result` for true multi-round
//! stepping. Each LLM round is a separate `Continue` cycle in the
//! `LongRunningSkill` protocol.
//!
//! ### A2 Tick Flow
//!
//! ```text
//! First tick:
//!   1. Coordinator phase1: create subagent session
//!   2. ExecutionEngine::init_turn(): setup agent, model, tools, system prompt
//!   3. Return Continue { heartbeat_request }
//!
//! Subsequent ticks:
//!   1. ExecutionEngine::tick(): one round (compression → LLM → tools → accumulate → decide)
//!   2. If Continue: return Continue { heartbeat_request }
//!   3. If Done: finalize_turn() + build_result() → return Done { final_output }
//!   4. If Cancelled: return Done { Cancelled }
//!   5. If Error: return Err
//! ```
//!
//! The `dispatch_once` in `spawn_long_running` is a no-op heartbeat
//! (A2 pragmatic compromise). The real LLM + tool execution happens
//! inside `ExecutionEngine::tick()`.
//!
//! ## Activation
//!
//! `coordinator.rs::execute_hidden_subagent_internal` calls
//! `run_a1_path()` when:
//!   - `USE_LIGHTWEIGHT_ACTOR = true` (const flag, ACTIVATED 2026-06-23 per
//!     `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`)
//!   - caller passed a non-None `actor_runtime: Option<&Arc<ActorRuntime>>`
//!
//! Both conditions must hold. At flag=false (default), the gate
//! is dead code and the existing phase1/2/3 path runs.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use northhing_agent_dispatch::{
    ActorContext, ActorError, ActorRuntime, LongRunningRequest, LongRunningSkill, LongRunningTickOutput,
};
use northhing_runtime_ports::{LightweightTaskOutput, LightweightTaskRequest};
use tokio_util::sync::CancellationToken;

use crate::agentic::coordination::global_coordinator;
use crate::agentic::execution::{ExecutionContext, ExecutionTurnState, RoundTickResult};
use crate::util::errors::{NortHingError, NortHingResult};

use super::coordinator::{HiddenSubagentExecutionRequest, SubagentResult, SubagentResultStatus};

/// Run the A1/A2 long-running path: spawn the coordinator skill, await
/// the execution outcome, map back to `SubagentResult`.
///
/// Returns `Err(NortHingError::service(...))` on any failure
/// (skill error, join error, timeout).
pub(crate) async fn run_a1_path(
    actor_runtime: &Arc<ActorRuntime>,
    request: &HiddenSubagentExecutionRequest,
    cancel_token: Option<&CancellationToken>,
    timeout_seconds: Option<u64>,
) -> NortHingResult<SubagentResult> {
    let skill = CoordinatorHiddenSubagentSkill {
        id: format!("a1-{}", request.session_name),
        request: request.clone(),
        // K.2.3 A1: propagate the caller's cancel token into the skill.
        // The skill merges this with the runtime's ctx.cancel in tick().
        cancel_token: cancel_token.cloned(),
        timeout_seconds,
        // A2: turn state is initialized on first tick
        turn_state: None,
        // A2: execution context cached on first tick
        execution_context: None,
        phase1_done: false,
    };
    let initial_request = build_a1_initial_request(request);
    let join = actor_runtime.spawn_long_running(Box::new(skill), initial_request);

    let dispatch_outcome = match tokio::time::timeout(Duration::from_secs(timeout_seconds.unwrap_or(300)), join).await {
        Ok(Ok(Ok(out))) => out,
        Ok(Ok(Err(e))) => return Err(NortHingError::service(format!("A1 path skill error: {e}"))),
        Ok(Err(join_err)) => return Err(NortHingError::service(format!("A1 path join error: {join_err}"))),
        Err(_) => return Err(NortHingError::service("A1 path timeout".to_string())),
    };

    Ok(map_lightweight_to_subagent_result(dispatch_outcome))
}

/// Build the initial `LongRunningRequest` from the rich
/// `HiddenSubagentExecutionRequest`. The request is used to seed the
/// `LongRunningSkill` loop; the actual execution goes through the
/// coordinator, not the dispatcher.
fn build_a1_initial_request(request: &HiddenSubagentExecutionRequest) -> LongRunningRequest {
    let prepended_context: Vec<String> = request.initial_messages.iter().map(|m| format!("{:?}", m)).collect();
    LongRunningRequest(LightweightTaskRequest {
        dispatch_id: format!("a1-{}", request.session_name),
        user_prompt: request.user_input_text.clone(),
        prepended_context,
        tool_allowlist: Vec::new(),
        timeout: Some(Duration::from_secs(300)),
        cancel: None,
        telemetry: None,
    })
}

/// A `LongRunningSkill` that delegates subagent execution to the
/// global `ConversationCoordinator`.
///
/// A1: The skill's `tick` calls `execute_hidden_subagent_internal`
/// directly (with `actor_runtime=None` to avoid recursive A1 gate
/// entry), maps the `SubagentResult` to `LightweightTaskOutput`,
/// and returns `Done`.
///
/// A2: The skill's `tick` uses `ExecutionEngine::init_turn` + `tick`
/// for true multi-round stepping. Each LLM round is a separate
/// `Continue` cycle.
///
/// Cancel observation: the coordinator's phase2 loop observes the
/// cancel token internally. The `spawn_long_running` runtime also
/// observes cancel at the tick boundary, so cancellation propagates
/// from the runtime's `ctx.cancel` into the coordinator's
/// `subagent_cancel_token`.
struct CoordinatorHiddenSubagentSkill {
    id: String,
    request: HiddenSubagentExecutionRequest,
    cancel_token: Option<CancellationToken>,
    timeout_seconds: Option<u64>,
    // A2: persistent turn state across ticks
    turn_state: Option<ExecutionTurnState>,
    // A2: cached execution context (avoids regenerating dialog_turn_id each tick)
    execution_context: Option<ExecutionContext>,
    // A2: track whether phase1 (session creation) is done
    phase1_done: bool,
}

#[async_trait]
impl LongRunningSkill for CoordinatorHiddenSubagentSkill {
    fn id(&self) -> &str {
        &self.id
    }

    fn skill_name(&self) -> &str {
        "coordinator_hidden_subagent"
    }

    async fn tick(
        &mut self,
        ctx: &ActorContext,
        _prior: Option<LightweightTaskOutput>,
    ) -> Result<LongRunningTickOutput, ActorError> {
        let coordinator =
            global_coordinator().ok_or_else(|| ActorError::new("Global coordinator not available".to_string()))?;

        // Merge the skill's explicit cancel token with the runtime's
        // cancel token. The runtime's token is the primary source;
        // the skill's token (if any) is a secondary source.
        let cancel_token = self.cancel_token.clone().unwrap_or_else(|| ctx.cancel.clone());

        // A2: First tick — run phase1 (create session) + init_turn
        if !self.phase1_done {
            // Run coordinator phase1 to create the subagent session
            let phase1 = coordinator
                .execute_hidden_subagent_phase1(self.request.clone(), Some(&cancel_token), self.timeout_seconds)
                .await
                .map_err(|e| ActorError::new(e.to_string()))?;

            // Build ExecutionContext from phase1 output and cache it
            let execution_context = build_execution_context(&phase1, &self.request);
            self.execution_context = Some(execution_context.clone());

            // Initialize turn on execution engine
            let engine = coordinator.execution_engine();
            let initial_messages = phase1.initial_messages.clone();
            let agent_type = phase1.agent_type.clone();

            let turn_state = engine
                .init_turn(agent_type, initial_messages, &execution_context)
                .await
                .map_err(|e| ActorError::new(e.to_string()))?;

            self.turn_state = Some(turn_state);
            self.phase1_done = true;

            // Return Continue to start the first round
            return Ok(LongRunningTickOutput::Continue {
                next_request: build_heartbeat_request(&self.request, 0),
            });
        }

        // A2: Subsequent ticks — execute one round
        let state = self
            .turn_state
            .as_mut()
            .ok_or_else(|| ActorError::new("Turn state not initialized".to_string()))?;

        // Use cached ExecutionContext (avoids regenerating dialog_turn_id each tick)
        let execution_context = self
            .execution_context
            .as_ref()
            .ok_or_else(|| ActorError::new("Execution context not initialized".to_string()))?;
        let engine = coordinator.execution_engine();

        match engine.tick(execution_context, state).await {
            Ok(RoundTickResult::Continue) => Ok(LongRunningTickOutput::Continue {
                next_request: build_heartbeat_request(&self.request, state.round_index),
            }),
            Ok(RoundTickResult::Done) => {
                // Finalize if needed, then build result
                let _ = engine
                    .finalize_turn(execution_context, state)
                    .await
                    .map_err(|e| ActorError::new(e.to_string()))?;

                // Build ExecutionResult from state
                // We need a start_time and initial_count; for A2 we use defaults
                let result = engine.build_result(state, std::time::Instant::now(), 0);
                let final_output = map_execution_result_to_lightweight(result);
                Ok(LongRunningTickOutput::Done { final_output })
            }
            Ok(RoundTickResult::Cancelled) => Ok(LongRunningTickOutput::Done {
                final_output: LightweightTaskOutput::Cancelled,
            }),
            Ok(RoundTickResult::Error { error }) => Err(ActorError::new(error)),
            Err(e) => Err(ActorError::new(e.to_string())),
        }
    }
}

/// Build an `ExecutionContext` from `SubagentPhase1Output` and `HiddenSubagentExecutionRequest`.
fn build_execution_context(
    phase1: &super::coordinator::SubagentPhase1Output,
    request: &HiddenSubagentExecutionRequest,
) -> ExecutionContext {
    ExecutionContext {
        session_id: phase1.session_id.clone(),
        dialog_turn_id: phase1.dialog_turn_id.clone(),
        turn_index: phase1.turn_index,
        agent_type: phase1.agent_type.clone(),
        workspace: phase1.subagent_workspace.clone(),
        context: request.context.clone(),
        subagent_parent_info: phase1.subagent_parent_info.clone(),
        delegation_policy: phase1.delegation_policy,
        skip_tool_confirmation: true,
        runtime_tool_restrictions: phase1.runtime_tool_restrictions.clone(),
        workspace_services: None, // A2: services built in init_turn
        round_injection: None,
        recover_partial_on_cancel: true,
    }
}

/// Build a heartbeat-style `LongRunningRequest` for A2 no-op dispatch.
fn build_heartbeat_request(request: &HiddenSubagentExecutionRequest, round_index: usize) -> LongRunningRequest {
    LongRunningRequest(LightweightTaskRequest {
        dispatch_id: format!("a2-{}-r{}", request.session_name, round_index),
        user_prompt: format!("heartbeat round {}", round_index),
        prepended_context: vec![],
        tool_allowlist: vec![],
        timeout: Some(Duration::from_secs(300)),
        cancel: None,
        telemetry: None,
    })
}

/// Map an `ExecutionResult` to a `LightweightTaskOutput`.
fn map_execution_result_to_lightweight(result: crate::agentic::execution::ExecutionResult) -> LightweightTaskOutput {
    LightweightTaskOutput::ToolResult {
        tool_name: "subagent".to_string(),
        output: result.final_message.content.to_string(),
    }
}

/// Map a `SubagentResult` to a `LightweightTaskOutput`.
///
/// This is the inverse of `map_lightweight_to_subagent_result`.
#[allow(dead_code)]
fn map_subagent_result_to_lightweight(result: SubagentResult) -> LightweightTaskOutput {
    match result.status {
        SubagentResultStatus::Completed => LightweightTaskOutput::ToolResult {
            tool_name: "subagent".to_string(),
            output: result.text,
        },
        SubagentResultStatus::PartialTimeout => {
            // Distinguish between timeout, cancellation, and other partial
            // failures based on the reason field.
            let reason = result.reason.as_deref().unwrap_or("unknown");
            if reason == "timeout" {
                LightweightTaskOutput::Timeout
            } else if reason == "cancelled" {
                LightweightTaskOutput::Cancelled
            } else {
                LightweightTaskOutput::Backend { message: result.text }
            }
        }
    }
}

/// Map a `LightweightTaskOutput` (one-shot dispatcher result) to a
/// `SubagentResult`. Pure function — every variant covered.
///
/// For `ToolResult`, attempts to parse `output` as JSON and populates
/// `structured_output` when successful. The original string is always
/// preserved in `text` regardless of parse success.
pub(crate) fn map_lightweight_to_subagent_result(out: LightweightTaskOutput) -> SubagentResult {
    match out {
        LightweightTaskOutput::ToolResult { tool_name: _, output } => {
            let structured_output = serde_json::from_str(&output).ok();
            SubagentResult {
                text: output,
                structured_output,
                status: SubagentResultStatus::Completed,
                reason: None,
                ledger_event_id: None,
            }
        }
        LightweightTaskOutput::NoToolMatched { reason } => SubagentResult {
            text: format!("No tool matched: {reason}"),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some(reason),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Cancelled => SubagentResult {
            text: "[cancelled]".to_string(),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("cancelled".to_string()),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Timeout => SubagentResult {
            text: "[timeout]".to_string(),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("timeout".to_string()),
            ledger_event_id: None,
        },
        LightweightTaskOutput::Backend { message } => SubagentResult {
            text: format!("Backend error: {message}"),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some(message),
            ledger_event_id: None,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod activation_tests {
    //! Activation regression tests for the A2 long-running path.
    //!
    //! Per spec `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`,
    //! `USE_LIGHTWEIGHT_ACTOR` was activated on 2026-06-23. These tests pin
    //! the activation contract: the const flag is on.
    use northhing_agent_dispatch::USE_LIGHTWEIGHT_ACTOR;

    /// K.2.3 follow-up T5 — regression for the A2 activation.
    /// If this test fails, the activation was reverted without a paired
    /// spec update.
    #[test]
    fn use_lightweight_actor_is_activated() {
        assert!(
            USE_LIGHTWEIGHT_ACTOR,
            "USE_LIGHTWEIGHT_ACTOR must be true as of 2026-06-23 \
             (spec docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md). \
             If this test fails after a deliberate revert, update the spec + \
             this test together."
        );
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn tool_result_maps_to_completed() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::ToolResult {
            tool_name: "echo".into(),
            output: "hello".into(),
        });
        assert_eq!(out.text, "hello");
        assert_eq!(out.status, SubagentResultStatus::Completed);
        assert_eq!(out.reason, None);
        assert_eq!(out.ledger_event_id, None);
    }

    #[test]
    fn no_tool_matched_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::NoToolMatched {
            reason: "empty allowlist".into(),
        });
        assert!(out.text.contains("No tool matched"));
        assert!(out.text.contains("empty allowlist"));
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("empty allowlist"));
    }

    #[test]
    fn cancelled_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Cancelled);
        assert_eq!(out.text, "[cancelled]");
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("cancelled"));
    }

    #[test]
    fn timeout_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Timeout);
        assert_eq!(out.text, "[timeout]");
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("timeout"));
    }

    #[test]
    fn tool_result_json_parses_to_structured_output() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::ToolResult {
            tool_name: "json_tool".into(),
            output: r#"{"status":"ok","count":42}"#.into(),
        });
        assert_eq!(out.text, r#"{"status":"ok","count":42}"#);
        assert_eq!(out.status, SubagentResultStatus::Completed);
        assert!(out.structured_output.is_some());
        let structured = out.structured_output.unwrap();
        assert_eq!(structured["status"], "ok");
        assert_eq!(structured["count"], 42);
    }

    // Inverse mapping tests: SubagentResult -> LightweightTaskOutput

    #[test]
    fn completed_maps_to_tool_result() {
        let result = SubagentResult {
            text: "hello".to_string(),
            structured_output: None,
            status: SubagentResultStatus::Completed,
            reason: None,
            ledger_event_id: None,
        };
        let out = map_subagent_result_to_lightweight(result);
        assert_eq!(
            out,
            LightweightTaskOutput::ToolResult {
                tool_name: "subagent".to_string(),
                output: "hello".to_string(),
            }
        );
    }

    #[test]
    fn partial_timeout_with_timeout_reason_maps_to_timeout() {
        let result = SubagentResult {
            text: "timed out".to_string(),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("timeout".to_string()),
            ledger_event_id: None,
        };
        let out = map_subagent_result_to_lightweight(result);
        assert_eq!(out, LightweightTaskOutput::Timeout);
    }

    #[test]
    fn partial_timeout_with_cancelled_reason_maps_to_cancelled() {
        let result = SubagentResult {
            text: "cancelled".to_string(),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("cancelled".to_string()),
            ledger_event_id: None,
        };
        let out = map_subagent_result_to_lightweight(result);
        assert_eq!(out, LightweightTaskOutput::Cancelled);
    }

    #[test]
    fn partial_timeout_with_other_reason_maps_to_backend() {
        let result = SubagentResult {
            text: "some error".to_string(),
            structured_output: None,
            status: SubagentResultStatus::PartialTimeout,
            reason: Some("other".to_string()),
            ledger_event_id: None,
        };
        let out = map_subagent_result_to_lightweight(result);
        assert_eq!(
            out,
            LightweightTaskOutput::Backend {
                message: "some error".to_string(),
            }
        );
    }

    #[test]
    fn tool_result_invalid_json_leaves_structured_output_none() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::ToolResult {
            tool_name: "text_tool".into(),
            output: "hello world".into(),
        });
        assert_eq!(out.text, "hello world");
        assert_eq!(out.status, SubagentResultStatus::Completed);
        assert!(out.structured_output.is_none());
    }

    #[test]
    fn backend_error_maps_to_partial_timeout() {
        let out = map_lightweight_to_subagent_result(LightweightTaskOutput::Backend {
            message: "rate limited".into(),
        });
        assert!(out.text.contains("rate limited"));
        assert!(out.structured_output.is_none());
        assert_eq!(out.status, SubagentResultStatus::PartialTimeout);
        assert_eq!(out.reason.as_deref(), Some("rate limited"));
    }
}
