//! Chat mode TUI interface
//!
//! This module is split across multiple files under `ui/chat/` to keep individual files manageable.

include!("chat/state.rs");
include!("chat/render/layout.rs");
include!("chat/render/header.rs");
include!("chat/render/messages.rs");
include!("chat/render/status_bar.rs");
include!("chat/render/input.rs");
include!("chat/render/selectors.rs");
include!("chat/render/shortcuts.rs");
include!("chat/tools.rs");
include!("chat/input.rs");
include!("chat/popups.rs");
include!("chat/scroll.rs");
include!("chat/mouse.rs");

#[cfg(test)]
mod state_split_tests;
