//! T4 (2026-08-05) settings Skills search-filter callback.
//!
//! Background:
//! Slint 1.16 has no `string.contains()` (the brief's §3.3
//! pre-condition for a pure-UI filter fell through — see the
//! handoff-20260805.md and task-04-brief §3.3). The
//! orchestrator-approved pivot is: the `SettingsView` search
//! input forwards its text to Rust via a new
//! `set-skill-filter(string)` callback; Rust re-applies the
//! filter against the full skill list cached on `AppState`
//! (refreshed by `refresh_settings_lists` on every settings
//! reload), rebuilds the published `skills-list` model, and
//! pushes 9 partition-visibility booleans so the UI hides
//! empty partitions entirely (no empty title + height:0 hack).
//!
//! Filter rule (brief §3.3 + §10.1):
//!   - empty string -> show all
//!   - non-empty -> case-insensitive substring match on `name`
//!     OR `description`; rows that don't match are dropped.
//!   - 9 partition visibility bools (引擎/玩法/设计/工程/office/
//!     meta/computer-use/gstack/其他) tell the UI which
//!     partitions still have at least one matching row.

use super::apply_skill_filter;
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::sync::Arc;

/// T4 (2026-08-05): register the `set-skill-filter(string)`
/// callback. The callback is dispatched synchronously on the
/// UI thread (Slint's contract: callbacks run on the UI
/// thread). The actual work is cheap (one linear pass over
/// the cached full list, in-memory only), so no background
/// thread is needed — keeping the path synchronous also
/// keeps the search box feeling instant.
///
/// The full list is fetched from `AppState` (the
/// process-global handle installed by `create_ui` via
/// `AppState::install_global`). The filter text is
/// stashed back on `AppState` so the next
/// `refresh_settings_lists` (which fires after every
/// skill toggle) re-applies it before publishing.
pub(crate) fn register_set_skill_filter_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_handle = ui.as_weak();
    ui.on_set_skill_filter(move |filter| {
        let needle = filter.to_string();
        // Stash the filter text first so a concurrent
        // `refresh_settings_lists` (e.g. from a skill toggle
        // happening on another thread) picks it up. The
        // dispatch below is the immediate-render path; the
        // refresh path goes through `apply_skill_filter` in
        // `refresh_settings_lists` too, so the two stay in
        // lock-step.
        let app_state = AppState::global();
        app_state.set_skills_filter(needle.clone());
        let full = app_state.skills_full_snapshot();
        let (published, vis, rows) = apply_skill_filter(&full, &needle);
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_skills_list(ModelRc::new(VecModel::from(published)));
            // Push the 9 per-partition row models (same order as §10.1).
            ui.set_skill_rows_engine(ModelRc::new(VecModel::from(rows[0].clone())));
            ui.set_skill_rows_gameplay(ModelRc::new(VecModel::from(rows[1].clone())));
            ui.set_skill_rows_design(ModelRc::new(VecModel::from(rows[2].clone())));
            ui.set_skill_rows_engineering(ModelRc::new(VecModel::from(rows[3].clone())));
            ui.set_skill_rows_office(ModelRc::new(VecModel::from(rows[4].clone())));
            ui.set_skill_rows_meta(ModelRc::new(VecModel::from(rows[5].clone())));
            ui.set_skill_rows_computer_use(ModelRc::new(VecModel::from(rows[6].clone())));
            ui.set_skill_rows_gstack(ModelRc::new(VecModel::from(rows[7].clone())));
            ui.set_skill_rows_other(ModelRc::new(VecModel::from(rows[8].clone())));
            // Push the 9 partition visibility booleans.
            // Order matches brief §10.1.
            ui.set_skill_cat_engine(vis[0]);
            ui.set_skill_cat_gameplay(vis[1]);
            ui.set_skill_cat_design(vis[2]);
            ui.set_skill_cat_engineering(vis[3]);
            ui.set_skill_cat_office(vis[4]);
            ui.set_skill_cat_meta(vis[5]);
            ui.set_skill_cat_computer_use(vis[6]);
            ui.set_skill_cat_gstack(vis[7]);
            ui.set_skill_cat_other(vis[8]);
        }
    });
}

