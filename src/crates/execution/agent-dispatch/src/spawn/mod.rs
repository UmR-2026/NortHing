//! Spawn adapter surface.
//!
//! One adapter exists:
//! - [`tokio_adapter`] — in-process spawn on the current `tokio::runtime::Handle`.
//!   This is the production path (per `.agents/reference/actor/NOTES.md` ⛔ #4:
//!   "Do NOT use a separate tokio runtime for actors").
//!
//! Pattern source: `.agents/reference/actor/03-actor-runtime.rs` (the
//! `IpcSpawnAdapter` comment block) and `.agents/reference/_upstream/tokio-actor-pattern.md`.

pub mod tokio_adapter;

pub use tokio_adapter::TokioSpawnAdapter;
