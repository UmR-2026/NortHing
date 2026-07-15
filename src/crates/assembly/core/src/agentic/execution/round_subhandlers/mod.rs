//! Sub-handlers for `RoundExecutor::execute_round`.
//!
//! Split from `round_executor.rs` per Round 8b (Round 7 `start_dialog_turn_internal`
//! pattern). 4-stage lifecycle:
//! - `prepare_stream`: init round state (round_id, cancel_token, ModelRoundStarted)
//! - `dispatch_stream`: stream attempt loop with retry policy
//! - `process_result`: post-loop finalize + tool execution + result build
//! - `handle_error`: no-op (errors propagate via `?`)
//!
//! Round 45 split: facade (this file) + 4 sibling sub-modules.
//! - `round_state`: shared `RoundState` + `DispatchOutcome` types,
//!   `RoundState::new` constructor, and `RoundExecutor::handle_error` no-op.
//! - `prepare_stream`: `RoundExecutor::prepare_stream`.
//! - `dispatch_stream`: `RoundExecutor::dispatch_stream`.
//! - `process_result`: `RoundExecutor::process_result`.
//!
//! Sibling modules are declared here (private to `round_subhandlers`) and their
//! `pub(super)` items are re-exported via `pub(super) use ...::*` so the
//! parent `execution` module — specifically `round_executor.rs` — can continue
//! to call `RoundState::new(...)` and `self.prepare_stream(...)` etc. via the
//! existing `super::round_subhandlers::*` path.

mod dispatch_stream;
mod prepare_stream;
mod process_result;
mod round_state;

// Re-export `RoundState` so `super::round_subhandlers::RoundState::new(...)`
// continues to resolve from `round_executor.rs`. `DispatchOutcome` is only
// used internally between sibling files (via `super::round_state::...`) and
// does not need to be re-exported. Methods on `RoundExecutor` are inherent
// impl methods in each sibling file; they don't need re-export either
// because Rust resolves `self.method(...)` through the type's impl blocks.
pub(crate) use round_state::RoundState;
