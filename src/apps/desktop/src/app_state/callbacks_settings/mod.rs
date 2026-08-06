//! Settings panel Slint callback wirings (R37a split from mod.rs)
//!
//! Each `register_X_callback` function takes a `&AppWindow` +
//! `&Arc<AppState>` and wires the matching `ui.on_X(...)` closure.
//! Bodies + comments are preserved verbatim from the original
//! `mod.rs` (R37a spec: preserve all comments + bodies).
//!
//! Note: the setup line `Arc::clone(&app_state)` is rewritten to
//! `Arc::clone(app_state)` to match the `&Arc<AppState>` parameter;
//! semantics are identical (clone the Arc, no behavior change).

mod misc;
mod provider;
mod provider_test;
mod refresh;
mod workspace;

pub(crate) use misc::*;
pub(crate) use provider::*;
pub(crate) use provider_test::*;
pub(crate) use refresh::*;
pub(crate) use workspace::*;

// 2026-06-26 (Phase 5 wire-up): helpers that load / save the
// on-disk `AppSettings` without panicking on IO errors. The Q6/Q7
// callbacks (delete-provider, remove-workspace) call these from a
// `tokio` runtime; failures route to the banner via
// `set_banner_message` rather than crashing the UI. These wrap
// `load_app_settings` / `update_app_settings` in `settings/io.rs` (which
// return `anyhow::Result`). The `_quiet` suffix means "swallow the
// Result, only log on failure" — the caller decides what to do.

pub(super) async fn load_app_settings_quiet() -> Result<crate::app_state::settings::AppSettings, String> {
    match crate::app_state::settings::load_app_settings().await {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::warn!(target: "app_state", "load_app_settings failed: {e}");
            Err(format!("加载设置失败: {e}"))
        }
    }
}

/// 2026-07-31 (H-9): transactional write entry for the settings callbacks.
/// Runs the whole load → mutate → save cycle under the settings single-writer
/// lock (see `settings::io::update_app_settings`), so concurrent settings
/// actions serialize instead of silently overwriting each other. On failure
/// the error is formatted for the banner / inline error, same as the quiet
/// load/save helpers it replaces.
pub(super) async fn update_app_settings_quiet<T>(
    f: impl FnOnce(&mut crate::app_state::settings::AppSettings) -> anyhow::Result<T>,
) -> Result<T, String> {
    match crate::app_state::settings::update_app_settings(f).await {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::warn!(target: "app_state", "update_app_settings failed: {e}");
            Err(format!("保存设置失败: {e}"))
        }
    }
}
