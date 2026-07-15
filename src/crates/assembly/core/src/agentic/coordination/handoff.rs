//! SubAgentHandoff: explicit handoff contract for sub-agent invocation.
//!
//! Workstream B-2 of the sub-agent orchestration hardening spec
//! (see `docs/superpowers/specs/2026-07-11-sub-agent-orchestration-hardening.md`).
//!
//! ## Why
//!
//! K.2.3 A1/A2 introduced `LongRunningSkill` for sub-agent context isolation,
//! but the handoff protocol between main agent and sub-agent remained implicit:
//!
//! - "When can a sub-agent be invoked?" (any time within a turn)
//! - "How does the sub-agent return?" (result embedded in tool result)
//! - "What's the input contract?" (concrete `HiddenSubagentExecutionRequest`
//!   was already type-safe, but the *dispatch path* called the legacy
//!   `execute_hidden_subagent_internal` directly with no protocol layer).
//!
//! The trait formalizes the contract: each handoff has typed input/output,
//! enforces one call per turn, and reports per-turn violations explicitly.
//!
//! ## Migration
//!
//! Per `docs/audit/2026-07-11-b2-handoff-callers.md`:
//! - `so_dispatch::execute_subagent` -> calls `SubAgentHandoff::handoff`
//! - `so_dispatch::start_background_subagent` -> wraps the same in a `tokio::spawn`
//! - `a1_path::CoordinatorHiddenSubagentSkill::tick` first tick -> uses
//!   `CoordinatorHiddenSubagentHandoff` (the canonical impl)
//! - `so_lifecycle::mod.rs:27` `execute_hidden_subagent_internal` ->
//!   `#[deprecated]` (target removal post-0.1.0; A1 fallback path keeps
//!   using it when `USE_LIGHTWEIGHT_ACTOR=true && actor_runtime.is_none()`)

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::agentic::coordination::coordinator::{
    ConversationCoordinator, HiddenSubagentExecutionRequest, SubagentResult,
};
use crate::util::errors::{NortHingError, NortHingResult};

// ═══════════════════════════════════════════════════════════════════
// Trait
// ═══════════════════════════════════════════════════════════════════

/// Handoff contract: sub-agent invocation is a single, terminal action
/// within the invoking turn. After handoff, the invoking turn ends; the
/// main agent sees only the handoff result, not interleaved with other
/// tool calls.
///
/// # Per-turn enforcement
///
/// Implementations MUST call `TurnHandoffCounter::try_record` before
/// dispatching. A second handoff in the same turn returns
/// `HandoffError::TooManyCallsInTurn` and the second invocation is rejected.
///
/// # Type safety
///
/// The trait uses associated `Input`/`Output` types instead of `serde_json::Value`.
/// This means each handoff implementation declares its own domain contract;
/// cross-domain handoffs require an explicit conversion, which the type
/// checker catches.
///
/// # Async
///
/// Uses `#[async_trait]` (matches the K.2.3 `LongRunningSkill` pattern in
/// the same layer). The `Box<dyn SubAgentHandoff>` shape is required for
/// the background path (`start_background_subagent`) where the handoff
/// future is moved into a `tokio::spawn`.
///
/// # Visibility
///
/// `pub(crate)` because `HiddenSubagentExecutionRequest` and `SubagentResult`
/// (the canonical `Input`/`Output`) are both `pub(crate)`. The trait
/// itself is the assembly-internal abstraction; per the B-2 spec the
/// long-term goal is to lift it to `pub` once the input/output types
/// are also public-stable.
#[async_trait]
pub(crate) trait SubAgentHandoff: Send + Sync {
    /// Concrete input shape (type-safe; not `Value`).
    /// `Send + 'static` so the handoff can be moved into a `tokio::spawn`
    /// for the background path (`start_background_subagent`).
    /// Note: northing's handoffs are in-process; no `Serialize`/`Deserialize`
    /// bound is required (the spec's wire-format variant was deliberately
    /// dropped after surveying existing types — most do not derive serde).
    type Input: Send + 'static;
    /// Concrete output shape (type-safe; not `Value`).
    type Output: Send + 'static;

    /// Invokes the sub-agent in isolated context. The caller MUST guarantee
    /// this is the only sub-agent call in the current turn (enforced at
    /// runtime via `TurnHandoffCounter`).
    ///
    /// `turn_id` is a stable id used by the per-turn counter; the same
    /// `turn_id` from two consecutive calls in the same turn triggers
    /// `HandoffError::TooManyCallsInTurn`.
    async fn handoff(
        &self,
        turn_id: &str,
        input: Self::Input,
    ) -> NortHingResult<Self::Output>;
}

// ═══════════════════════════════════════════════════════════════════
// HandoffError
// ═══════════════════════════════════════════════════════════════════

/// Why a handoff failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffError {
    /// The current turn already had a successful handoff; the second
    /// call is rejected. Carries the turn_id for diagnostics.
    TooManyCallsInTurn { turn_id: String, first_count: u8 },
    /// The handoff was cancelled via the cancel token.
    Cancelled { turn_id: String },
    /// The global coordinator is not available (e.g. test fixture
    /// without one). Carries the agent_type for context.
    CoordinatorUnavailable { agent_type: String },
    /// Input failed pre-handoff validation. Carries the turn_id + reason.
    InvalidInput { turn_id: String, reason: String },
}

impl std::fmt::Display for HandoffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCallsInTurn { turn_id, first_count } => write!(
                f,
                "turn {} already has {} handoff call(s); only one is allowed per turn",
                turn_id, first_count
            ),
            Self::Cancelled { turn_id } => write!(f, "handoff cancelled for turn {}", turn_id),
            Self::CoordinatorUnavailable { agent_type } => {
                write!(f, "global coordinator not available for agent_type={}", agent_type)
            }
            Self::InvalidInput { turn_id, reason } => {
                write!(f, "invalid handoff input for turn {}: {}", turn_id, reason)
            }
        }
    }
}

impl std::error::Error for HandoffError {}

impl From<HandoffError> for NortHingError {
    fn from(err: HandoffError) -> Self {
        match err {
            HandoffError::TooManyCallsInTurn { turn_id, first_count } => NortHingError::Validation(format!(
                "too many handoff calls in turn {} (count={})",
                turn_id, first_count
            )),
            HandoffError::Cancelled { turn_id } => {
                NortHingError::Cancelled(format!("handoff cancelled for turn {}", turn_id))
            }
            HandoffError::CoordinatorUnavailable { agent_type } => NortHingError::service(format!(
                "global coordinator not available for agent_type={}",
                agent_type
            )),
            HandoffError::InvalidInput { turn_id, reason } => {
                NortHingError::Validation(format!("invalid handoff input for turn {}: {}", turn_id, reason))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TurnHandoffCounter
// ═══════════════════════════════════════════════════════════════════

/// Per-turn runtime counter enforcing one handoff per turn.
///
/// Shared across the call site (e.g. wrapped in an `Arc` and stored on
/// the `ConversationCoordinator`). The counter is reset implicitly: a
/// new `turn_id` starts a fresh entry, returning `Ok(())` on the first
/// call and `Err(HandoffError::TooManyCallsInTurn { first_count: 1 })`
/// on the second call with the same `turn_id`.
///
/// # Thread-safety
///
/// `std::sync::Mutex` with a `HashMap`; the critical section is two
/// lookups + one insert (constant time amortized). Lock contention is
/// negligible because handoff is a rare event (once per turn at most).
#[derive(Debug, Default, Clone)]
pub struct TurnHandoffCounter {
    inner: Arc<Mutex<HashMap<String, u8>>>,
}

impl TurnHandoffCounter {
    /// New counter with zero recorded handoffs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to record a handoff for `turn_id`. Returns `Ok(())` if this
    /// is the first handoff in the turn, `Err(HandoffError::TooManyCallsInTurn)`
    /// if the turn already has a handoff.
    pub fn try_record(&self, turn_id: &str) -> Result<(), HandoffError> {
        let mut map = self.inner.lock().expect("TurnHandoffCounter mutex poisoned");
        let count = map.entry(turn_id.to_string()).or_insert(0);
        if *count >= 1 {
            return Err(HandoffError::TooManyCallsInTurn {
                turn_id: turn_id.to_string(),
                first_count: *count,
            });
        }
        *count += 1;
        Ok(())
    }

    /// Reset the counter for `turn_id` (used at turn boundaries).
    /// Idempotent: resetting a turn that has no entry is a no-op.
    #[allow(dead_code)] // public API for callers that manage turn boundaries; tests use it.
    pub fn reset(&self, turn_id: &str) {
        let mut map = self.inner.lock().expect("TurnHandoffCounter mutex poisoned");
        map.remove(turn_id);
    }

    /// Test helper: returns the current count for `turn_id` (0 if none).
    #[allow(dead_code)] // public API for tests + future call-site introspection.
    pub fn count(&self, turn_id: &str) -> u8 {
        let map = self.inner.lock().expect("TurnHandoffCounter mutex poisoned");
        map.get(turn_id).copied().unwrap_or(0)
    }

    /// Test helper: returns the total number of turns with at least
    /// one recorded handoff.
    #[allow(dead_code)] // public API for tests + future call-site introspection.
    pub fn tracked_turn_count(&self) -> usize {
        let map = self.inner.lock().expect("TurnHandoffCounter mutex poisoned");
        map.len()
    }
}

// ═══════════════════════════════════════════════════════════════════
// CoordinatorHiddenSubagentHandoff
// ═══════════════════════════════════════════════════════════════════

/// Canonical `SubAgentHandoff` impl backed by the global `ConversationCoordinator`.
///
/// Wraps the existing `execute_hidden_subagent_internal` path with
/// per-turn enforcement. This is the trait-level entry point that
/// `so_dispatch::execute_subagent` and `so_dispatch::start_background_subagent`
/// route through after B-2.
///
/// # Deprecation note
///
/// `execute_hidden_subagent_internal` is marked `#[deprecated]` as part
/// of B-2; the `#[allow(deprecated)]` on the call below keeps the
/// canonical handoff impl working until the legacy fn is fully removed
/// (target: post-0.1.0, when A1 fallback is replaced by `run_a1_path`).
pub struct CoordinatorHiddenSubagentHandoff {
    counter: TurnHandoffCounter,
}

impl CoordinatorHiddenSubagentHandoff {
    /// New handoff impl with its own per-turn counter. Callers that
    /// share a counter across multiple handoff impls should construct
    /// the counter once and clone it in.
    pub fn new() -> Self {
        Self {
            counter: TurnHandoffCounter::new(),
        }
    }

    /// New handoff impl sharing an externally-managed counter.
    #[allow(dead_code)] // public API for tests + future foreground/background counter sharing.
    pub fn with_counter(counter: TurnHandoffCounter) -> Self {
        Self { counter }
    }

    /// Access to the per-turn counter (used by tests + `start_background_subagent`
    /// to share state with the foreground `execute_subagent`).
    #[allow(dead_code)] // public API for tests + future foreground/background counter sharing.
    pub fn counter(&self) -> &TurnHandoffCounter {
        &self.counter
    }
}

impl Default for CoordinatorHiddenSubagentHandoff {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SubAgentHandoff for CoordinatorHiddenSubagentHandoff {
    type Input = HiddenSubagentExecutionRequest;
    type Output = SubagentResult;

    async fn handoff(
        &self,
        turn_id: &str,
        input: Self::Input,
    ) -> NortHingResult<Self::Output> {
        // Per-turn enforcement: only one handoff per turn.
        self.counter.try_record(turn_id)?;

        // Pull the global coordinator (legacy entry point). Each call
        // clones the `Arc` (matches the `a1_path` pattern — the global
        // is a `OnceLock<Arc<T>>` so `&'static` borrows are not stable
        // through `.get()`).
        let coordinator: Arc<ConversationCoordinator> =
            global_coordinator().ok_or_else(|| HandoffError::CoordinatorUnavailable {
                agent_type: input.agent_type.clone(),
            })?;

        // B-2 migration: the trait now owns the call. The legacy fn
        // is `#[deprecated]`; allow here so the canonical impl
        // remains the single point of entry until the legacy fn
        // is removed (target: post-0.1.0).
        #[allow(deprecated)]
        let result = coordinator
            .execute_hidden_subagent_internal(input, None, None, None)
            .await
            .map_err(NortHingError::from)?;

        Ok(result)
    }
}

/// Local alias for `super::global_coordinator` so the call site in
/// `handoff` reads naturally without importing the full coordinator
/// surface into this module's scope.
fn global_coordinator() -> Option<Arc<ConversationCoordinator>> {
    super::global_coordinator()
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_first_call_succeeds_second_fails() {
        let counter = TurnHandoffCounter::new();
        assert!(counter.try_record("turn-1").is_ok());
        let err = counter.try_record("turn-1").unwrap_err();
        assert_eq!(
            err,
            HandoffError::TooManyCallsInTurn {
                turn_id: "turn-1".into(),
                first_count: 1
            }
        );
        assert_eq!(counter.count("turn-1"), 1);
    }

    #[test]
    fn counter_distinct_turns_do_not_interfere() {
        let counter = TurnHandoffCounter::new();
        assert!(counter.try_record("turn-1").is_ok());
        assert!(counter.try_record("turn-2").is_ok());
        assert!(counter.try_record("turn-3").is_ok());
        assert_eq!(counter.tracked_turn_count(), 3);
        // Each turn is independently single-shot.
        assert!(counter.try_record("turn-1").is_err());
        assert!(counter.try_record("turn-2").is_err());
        assert!(counter.try_record("turn-3").is_err());
    }

    #[test]
    fn counter_reset_clears_entry() {
        let counter = TurnHandoffCounter::new();
        counter.try_record("turn-1").unwrap();
        assert_eq!(counter.count("turn-1"), 1);
        counter.reset("turn-1");
        assert_eq!(counter.count("turn-1"), 0);
        // After reset, a new handoff in the same turn is allowed.
        assert!(counter.try_record("turn-1").is_ok());
    }

    #[test]
    fn counter_clone_shares_state() {
        let counter = TurnHandoffCounter::new();
        let counter2 = counter.clone();
        counter.try_record("turn-1").unwrap();
        // Both clones see the same state.
        assert_eq!(counter2.count("turn-1"), 1);
        assert!(counter2.try_record("turn-1").is_err());
    }

    #[test]
    fn handoff_error_display_includes_turn_id() {
        let err = HandoffError::TooManyCallsInTurn {
            turn_id: "abc".into(),
            first_count: 1,
        };
        let msg = err.to_string();
        assert!(msg.contains("abc"));
        assert!(msg.contains("1"));
    }

    #[test]
    fn handoff_error_into_northing_error_maps_to_validation() {
        let err = HandoffError::TooManyCallsInTurn {
            turn_id: "abc".into(),
            first_count: 1,
        };
        let nort_err: NortHingError = err.into();
        match nort_err {
            NortHingError::Validation(msg) => {
                assert!(msg.contains("abc"));
            }
            other => panic!("expected Validation, got {:?}", other),
        }
    }

    #[test]
    fn handoff_error_cancelled_maps_to_northing_cancelled() {
        let err = HandoffError::Cancelled { turn_id: "abc".into() };
        let nort_err: NortHingError = err.into();
        match nort_err {
            NortHingError::Cancelled(_) => {}
            other => panic!("expected Cancelled, got {:?}", other),
        }
    }

    #[test]
    fn handoff_error_coordinator_unavailable_maps_to_service() {
        let err = HandoffError::CoordinatorUnavailable {
            agent_type: "echo-agent".into(),
        };
        let nort_err: NortHingError = err.into();
        match nort_err {
            NortHingError::Service(msg) => {
                assert!(msg.contains("echo-agent"));
            }
            other => panic!("expected Service, got {:?}", other),
        }
    }

    #[test]
    fn handoff_impl_default_uses_fresh_counter() {
        let h = CoordinatorHiddenSubagentHandoff::default();
        assert_eq!(h.counter().tracked_turn_count(), 0);
    }

    #[test]
    fn handoff_impl_with_counter_shares_state() {
        let counter = TurnHandoffCounter::new();
        let h = CoordinatorHiddenSubagentHandoff::with_counter(counter.clone());
        h.counter().try_record("turn-1").unwrap();
        assert_eq!(counter.count("turn-1"), 1);
    }
}
