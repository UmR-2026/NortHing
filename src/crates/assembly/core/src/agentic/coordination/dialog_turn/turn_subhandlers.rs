//! Sub-domain facade for `start_dialog_turn_internal` 4-phase sub-handlers (R47b refactor).
//!
//! The 4 sub-handler phases (`prepare_turn` -> `dispatch_turn` ->
//! `finalize_turn` -> `cleanup_turn`) and the shared `TurnContext` state were
//! extracted out of this 806-line god-file into 4 sibling modules under the
//! `dialog_turn` directory (declared in `mod.rs`). This facade only re-exports
//! `TurnContext` so that existing callers (e.g. `turn_cancel.rs` via
//! `use super::turn_subhandlers::TurnContext;`) continue to work without
//! touching their import paths.
//!
//! Spec §2.1 R47b — split `turn_subhandlers.rs` 806 -> facade + 4 sibling:
//!   - sub_handle_types   TurnContext struct + impl TurnContext::new
//!   - sub_handle_in      prepare_turn (input/preparation phase)
//!   - sub_handle_state   dispatch_turn (state/dispatch phase)
//!   - sub_handle_out     finalize_turn + cleanup_turn (output/finalization phase)

pub(crate) use super::sub_handle_types::TurnContext;
