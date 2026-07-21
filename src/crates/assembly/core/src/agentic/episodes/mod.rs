//! Episode Log module: growth/learning experience storage for dialog turns.
//!
//! This module provides diary-style storage of agent execution experiences.
//! The data is stored for analysis and learning but is NOT read by agents
//! for decision-making (prevents self-validation loops).
//!
//! # Storage
//!
//! Episodes are stored in `dirs::data_dir()/northhing/episodes/<workspace_slug>.jsonl`
//! as append-only JSON Lines, with automatic rotation when the file exceeds 5MB.
//!
//! # Distillation
//!
//! Episodes are distilled from persisted `DialogTurnData` after turn completion,
//! extracting tool usage, failures, and outcomes.

pub mod distill;
pub mod store;
pub mod types;

pub use distill::distill_episode;
pub use store::{append_episode, read_episodes};
pub use types::{Episode, EpisodeOutcome, RedlineStatus, RedlineVerdict, ToolFailureRecord, ToolUseRecord};
