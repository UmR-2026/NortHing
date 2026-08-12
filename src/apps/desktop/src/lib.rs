#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![deny(rustdoc::broken_intra_doc_links)]
//! northhing Desktop Shell Library
//!
//! Re-exports for the desktop application.

pub mod app_state;
pub mod flags;
pub mod mcp_adapter;

/// Parallel Dioxus consult-room shell (R3' migration, 2026-08-13).
///
/// Compiled only when the `ui-dioxus` cargo feature is enabled (default).
/// When disabled, the module compiles out completely and the Slint shell
/// remains the only UI surface, byte-identical behavior.
/// See `flags::DIOXUS_SHELL` for the runtime gate that defaults to `false`.
#[cfg(feature = "ui-dioxus")]
pub mod ui_dioxus;
