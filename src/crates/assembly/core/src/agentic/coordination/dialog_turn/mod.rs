//! `ConversationCoordinator` facade — `impl ConversationCoordinator` blocks split
//! into 8 sub-domain sibling files (R44a refactor).
//!
//! Public API surface (constructor, accessors, setters, session lifecycle,
//! bootstrap, turn start, thread goal, compact, cancel, restore, query, tool
//! control) is preserved by combining the facade impl blocks in each sibling.
//! Private / pub(crate) helper methods used to live here but were moved to
//! the `compaction`, `restore`, `session`, `thread_goal`, `turn`, `turn_cancel`,
//! `turn_persist`, `turn_subhandlers`, and `workspace` siblings in earlier rounds
//! (R21/R23/R34). This file is now a thin facade that wires the sub-domain
//! facade siblings together.
//!
//! Spec §2.1 — facade methods split by domain:
//!   - coordinator_init         constructor, accessors, setters
//!   - coordinator_session      session lifecycle (create/update/delete/list/...)
//!   - coordinator_bootstrap    ensure_assistant_bootstrap
//!   - coordinator_turn_start   start_dialog_turn_* wrappers
//!   - coordinator_thread_goal  12 thread_goal_* facade methods
//!   - coordinator_compact      compact_session_manually
//!   - coordinator_cancel       cancel_dialog_turn, cancel_active_turn_for_session
//!   - coordinator_restore      12 restore_* facade methods
//!
//! Wildcard re-export below keeps the historical flat import path
//! `crate::agentic::coordination::dialog_turn::*` working for downstream code.

// Import types from sibling coordinator.rs module (the struct,
// AssistantBootstrap enums, SubagentTimeoutAction, get_global_coordinator, etc.)
use super::coordinator::*;
// Import from sibling ports.rs (get_global_coordinator lives there)
use super::ports::{global_coordinator, is_ai_session_title_generation_enabled, GLOBAL_COORDINATOR};

pub use northhing_runtime_ports::DialogTriggerSource;

// Sub-domain facade impl blocks (facade methods only — public API).
pub mod coordinator_bootstrap;
pub mod coordinator_cancel;
pub mod coordinator_compact;
pub mod coordinator_init;
pub mod coordinator_restore;
pub mod coordinator_session;
pub mod coordinator_thread_goal;
pub mod coordinator_turn_start;

// Private/pub(crate) helper siblings (Round 6 spec §2.1, R21/R23/R34 refactors).
// Each sibling contains the helper methods that the facade siblings call via `self`.
pub mod compaction;
pub mod restore;
pub mod session;
pub mod sub_handle_in;
pub mod sub_handle_out;
pub mod sub_handle_state;
pub mod sub_handle_types;
pub mod thread_goal;
pub mod turn;
pub mod turn_cancel;
pub mod turn_persist;
pub mod turn_subhandlers;
pub mod workspace;
