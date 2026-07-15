//! Slint UI module bridge.
//!
//! **Purpose**: This module exists so that the `slint::include_modules!()`
//! macro runs in a tiny, dedicated module rather than in `mod.rs`. The
//! macro generates Rust types (`AppWindow`, `SessionItem`, `MessageItem`,
//! `SkillItem`, ...) directly into the module it's invoked from. By
//! keeping that invocation in a 4-line module, we:
//!
//! - Slim `mod.rs` by ~2 lines of boilerplate (the macro call) and
//!   ~10 lines of expected re-exports that follow.
//! - Make the boundary explicit: if a future maintainer adds a new
//!   Slint-generated type, they know exactly where to re-export it.
//! - Avoid the historical pattern of `slint::include_modules!()` at
//!   the top of a 1500-line file (which mixes UI-generated types
//!   with hand-written code and makes grep noisy).
//!
//! **Visibility**: The macro injects generated items directly into
//! this module. Sibling modules (`actor`, `sessions`, `skills`, ...)
//! and `mod.rs` reach them via the standard `super::slint_glue::Foo`
//! path. No `pub use` is needed here — the items are visible to
//! `super::*` because the module itself is `pub(super)` in `mod.rs`.
//!
//! **Note on re-exports**: Earlier drafts of this file tried
//! `pub(super) use AppWindow;` to alias the type. That triggered
//! E0252 because the macro already declares `pub use ::AppWindow;`
//! at the top of its output — adding another `use` for the same name
//! creates a duplicate definition. Bare path imports work fine.
//!
//! **K.2.1 of roadmap `2026-06-19-post-reference-roadmap.md`**.

slint::include_modules!();
