//! Spawn adapter surface.
//!
//! Two parallel adapters exist for Phase 1, both **stubs**:
//!
//! - [`tokio_adapter`] — in-process spawn on the current `tokio::runtime::Handle`.
//!   This is the production path (per `.agents/reference/actor/NOTES.md` ⛔ #4:
//!   "Do NOT use a separate tokio runtime for actors").
//! - [`ipc_adapter`] — placeholder that returns the literal string `"ipc-stub"`.
//!   Per the impl plan, the IPC body lands in Phase 3; until then the adapter
//!   is gated behind the `USE_ACTOR_IPC` and `USE_DISPATCHER_IPC` const flags
//!   (both default `false`) and any caller must treat `"ipc-stub"` as a no-op.
//!
//! Pattern source: `.agents/reference/actor/03-actor-runtime.rs` (the
//! `IpcSpawnAdapter` comment block) and `.agents/reference/_upstream/tokio-actor-pattern.md`.

pub mod ipc_adapter;
pub mod tokio_adapter;

pub use ipc_adapter::IpcSpawnAdapter;
pub use tokio_adapter::TokioSpawnAdapter;

/// Identifies which adapter was selected at runtime. Useful in logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnAdapterKind {
    Tokio,
    Ipc,
}
