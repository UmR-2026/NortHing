//! Session persistence: prompt cache + load/save + dialog turn lifecycle + maintenance
//!
//! (lines 517-803, 1194-1287, 1670-1829, 3266-3984 of original session_manager.rs)
//!
//! Contains:
//! - Message reconstruction from turns (build_messages_from_turns)
//! - Context snapshot persistence (per-turn + current-turn best-effort)
//! - Prompt cache lazy load + persist (system + user context)
//! - Listing diff sanitization for cutoff snapshots
//! - Dialog turn lifecycle (start/complete/fail/cancel + maintenance variants)

pub mod load;
pub mod prompt_cache;
pub mod save;
pub mod turn_lifecycle;

pub use load::*;
pub use prompt_cache::*;
pub use save::*;
pub use turn_lifecycle::*;
