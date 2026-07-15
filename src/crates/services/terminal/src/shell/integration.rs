//! Shell integration facade.
//!
//! Re-exports `types` (OscSequence, CommandState, ShellIntegrationEvent, helper fns),
//! `shell_integration` (ShellIntegration struct + impl), and
//! `shell_integration_manager` (ShellIntegrationManager struct + impl).

mod shell_integration;
mod shell_integration_manager;
mod types;

pub use shell_integration::*;
pub use shell_integration_manager::*;
pub use types::*;
