//! Browser instance launch/control facade.
//!
//! Splits the original `browser_launcher.rs` god-file into focused sibling modules
//! while preserving the existing public import path.

pub mod launcher_dispatch;
pub mod launcher_lifecycle;
pub mod launcher_recovery;
pub mod launcher_state;
pub mod launcher_types;

pub use launcher_lifecycle::BrowserLauncher;
pub use launcher_types::{BrowserInfo, BrowserKind, LaunchResult, DEFAULT_CDP_PORT};
