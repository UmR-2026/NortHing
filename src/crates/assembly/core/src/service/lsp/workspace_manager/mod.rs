//! Workspace-level LSP manager
//!
//! Core responsibilities:
//! - Manage the lifecycle of all LSP servers within a workspace
//! - Automatically start and stop servers
//! - Manage document state
//! - Error recovery and health checks
//! - Integrate filesystem monitoring
//! - Push real-time events to the frontend

mod client;
mod diagnostics;
mod format;
mod workspace;

pub use client::*;
pub use diagnostics::*;
pub use format::*;
pub use workspace::*;
