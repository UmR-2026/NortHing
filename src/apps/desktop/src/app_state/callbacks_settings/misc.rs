use super::load_app_settings_quiet;
use super::refresh_settings_lists;
use super::save_app_settings_quiet;
use crate::app_state::error_banners::set_banner_message;
use crate::app_state::settings::ModelRef;
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use slint::{ComponentHandle, SharedString};
use std::sync::Arc;

// 2026-07-18 (D2b): set-default-model handler. Finds the provider by id,
// verifies it is enabled, persists the ModelRef, then refreshes the
// settings lists and shows a success banner.
pub(crate) fn register_set_default_model_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_set_default_model(move |provider_id| {
        let pid = provider_id.to_string();
        let ui_weak = ui_weak.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "set-default-model: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak.clone(), e, "");
                        return;
                    }
                };
                let provider = match s.providers.iter().find(|p| p.id == pid && p.enabled) {
                    Some(p) => p.clone(),
                    None => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak.clone(), "未找到已启用的指定 AI 服务", "");
                        return;
                    }
                };
                s.default_model = Some(ModelRef {
                    provider_id: pid,
                    model: provider.model.clone(),
                });
                if let Err(e) = save_app_settings_quiet(&s).await {
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak.clone(), e, "");
                    return;
                }
                // 2026-07-18 (D2g): push default model into core so the runtime
                // sees the updated primary. Failure is non-fatal — the user's
                // data is safe on disk; we surface a banner and let them retry.
                if let Err(e) = crate::app_state::settings::sync_providers_to_core(&s).await {
                    tracing::warn!(target: "app_state", "set-default-model sync-to-core failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak.clone(), "同步到运行时配置失败，请重试".to_string(), "");
                    // do NOT return — data is already persisted
                }
                // 2026-07-18 (D2j): pass weak directly; helpers upgrade on UI thread.
                set_banner_message(ui_weak.clone(), "已设置默认模型", "");
                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}

// 2026-06-26 (Phase 4 fix): onboarding-completed handler. Persists
// `onboarding_completed = true` so a fully-skipped flow does not
// reappear on the next launch, then flips the route back to "main".
pub(crate) fn register_onboarding_completed_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_onboarding_completed(move || {
        let ui_weak2 = ui_weak.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "onboarding-completed: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak2.clone(), e, "");
                        return;
                    }
                };
                s.onboarding_completed = true;
                if let Err(e) = save_app_settings_quiet(&s).await {
                    tracing::warn!(target: "app_state", "onboarding-completed save failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak2.clone(), e, "");
                    return;
                }
                let ui_weak3 = ui_weak2.clone();
                if let Err(e) = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak3.upgrade() {
                        ui.set_current_route(slint::SharedString::from("main"));
                    }
                }) {
                    tracing::warn!(
                        target: "app_state",
                        "onboarding-completed: failed to dispatch route change: {e}"
                    );
                }
            });
        });
    });
}

// 2026-07-18 (D2h): refresh-settings callback. Fires when the settings route
// is entered so the panel always reflects current on-disk data.
pub(crate) fn register_refresh_settings_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_refresh_settings(move || {
        let ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "refresh-settings: failed to build runtime: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                // 2026-07-18 (D2j): pass weak directly; function upgrades on UI thread.
                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}
