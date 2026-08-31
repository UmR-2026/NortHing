// SPDX-License-Identifier: MIT OR Apache-2.0
//
// T1 Dioxus migration (2026-08-12) — Dioxus consult-room shell.
//
// This module is the entry point for the three-window Dioxus consult-room
// shell (room main window + inner + outer). Since the W4-1 Slint removal
// (commit `707e414`, 2026-08-28) the module is unconditionally compiled and
// `main.rs` calls `ui_dioxus::launch(...)` unconditionally — there is no
// `ui-dioxus` cargo feature or runtime flag any more (the previous
// `crate::flags::DIOXUS_SHELL` constant was deleted together with the Slint
// shell). The Dioxus shell brings up the three-window layout described in
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
mod api_events;
mod api_fs;
mod api_memory;
mod api_settings;
mod app;
mod approval_card;
mod color;
mod css;
mod entry;
mod i18n;
mod page_shell;
mod pages_archive;
mod pages_archive_search;
mod pages_memory;
mod pages_onboarding;
mod pages_onboarding_css;
mod pages_settings;
mod pages_settings_cards;
mod pages_settings_provider_edit;
mod pages_settings_skills;
mod pages_space;
mod panel_files;
mod registry;
mod session_mock;
mod state;
mod turn_banner;
mod window_ops;
mod windows;
