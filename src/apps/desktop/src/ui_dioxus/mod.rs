// SPDX-License-Identifier: MIT OR Apache-2.0
//
// T1 Dioxus migration (2026-08-12) — parallel Dioxus consult-room shell.
//
// This module is the entry point for the three-window Dioxus consult-room
// shell (room main window + inner + outer). It compiles only when the
// `ui-dioxus` cargo feature is enabled; when disabled it compiles out
// completely so the Slint shell remains byte-identical.
//
// Runtime gate: `crate::flags::DIOXUS_SHELL`. When `false` (the deliberate
// default), `main.rs` keeps launching the Slint shell. When `true` and the
// feature is on, `launch()` brings up the three-window shell described in
// `block-contract.md` §0/§3.1/§3.2 and the consult-room-main.html truth file.
//
// References:
//   * brief:     `.superpowers/sdd/consult-room/task-migrate-room-brief.md`
//   * contract:  `.superpowers/sdd/consult-room/block-contract.md`
//   * rulings:   `.superpowers/sdd/consult-room/truth-rulings-20260809.md`
//   * conversion: `.agents/skills/northhing-dioxus-frontend/references/conversion-annotations.md`
//   * patterns:  `.agents/skills/northhing-dioxus-frontend/references/desktop-patterns.md`

// Re-export only the entry point so `main.rs` can call `ui_dioxus::launch()`
// behind the same `cfg` guard the module is itself gated by.
pub use entry::launch;

mod entry;
mod state;
mod registry;
mod css;
mod i18n;
mod session_mock;
mod windows;
mod pages_archive;
mod pages_space;
mod pages_settings;
mod app;
