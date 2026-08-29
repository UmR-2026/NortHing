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

mod api;
mod app;
mod approval_card;
mod color;
mod css;
mod entry;
mod i18n;
mod page_shell;
mod pages_archive;
mod pages_memory;
mod pages_onboarding;
mod pages_onboarding_css;
mod pages_settings;
mod pages_settings_provider_edit;
mod pages_settings_skills;
mod pages_space;
mod registry;
mod session_mock;
mod state;
mod turn_banner;
mod window_ops;
mod windows;
