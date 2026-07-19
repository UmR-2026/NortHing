//! Long-lived tokio runtime handle for turn dispatch (W4).
//!
//! The worker thread in `main.rs` owns the app's multi-thread runtime
//! (it runs `initialize_core_services` and stays alive until shutdown).
//! Turn dispatch MUST spawn onto this runtime: spawning onto a
//! throwaway per-callback runtime aborts the turn task (spawned inside
//! `scheduler.submit`) as soon as the callback's `block_on` returns.

use std::sync::OnceLock;
use tokio::runtime::Handle;

static TURN_RUNTIME: OnceLock<Handle> = OnceLock::new();

pub(crate) fn set_turn_runtime_handle(handle: Handle) {
    let _ = TURN_RUNTIME.set(handle);
}

pub(crate) fn turn_runtime() -> Option<Handle> {
    TURN_RUNTIME.get().cloned()
}
