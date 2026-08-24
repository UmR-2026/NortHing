use super::refresh_settings_lists;
use super::update_app_settings_quiet;
use crate::app_state::error_banners::set_banner_message;
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::KernelSettingsApi;
use slint::ComponentHandle;
use std::sync::Arc;

// 2026-07-18 (D2b): set-default-model handler. Calls facade to set default
// provider, then refreshes the settings lists and shows a success banner.
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
                let facade = kernel_facade();
                if let Err(e) = facade.set_default_provider(&pid).await {
                    tracing::warn!(target: "app_state", "set-default-model set_default_provider failed: {e}");
                    set_banner_message(ui_weak.clone(), "未找到已启用的指定 AI 服务".to_string(), "");
                    return;
                }
                set_banner_message(ui_weak.clone(), "已设置默认模型".to_string(), "");
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
                // 2026-07-31 (H-9): load → mutate → save as one transaction
                // under the settings single-writer lock.
                if let Err(e) = update_app_settings_quiet(|s| {
                    s.onboarding_completed = true;
                    Ok(())
                })
                .await
                {
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

/// Global skill toggle callback for Settings > Skills panel.
pub(crate) fn register_set_skill_global_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_set_skill_global(move |skill_id, enabled| {
        let skill_id_str = skill_id.to_string();
        let ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(target: "app_state", "set-skill-global: failed to build runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                use northhing_kernel_api::agents::SkillScopeDto;
                use northhing_kernel_api::KernelAgentsApi;

                let facade = kernel_facade();
                let scope = SkillScopeDto {
                    scope_type: "user".to_string(),
                    workspace_path: None,
                    mode_id: Some("agentic".to_string()),
                };

                if let Err(e) = facade.set_skill_enabled(&skill_id_str, scope, enabled).await {
                    tracing::warn!(target: "app_state", "set_skill_enabled failed: {e}");
                    set_banner_message(ui_weak.clone(), format!("设置技能失败: {e}"), "");
                    return;
                }

                refresh_settings_lists(ui_weak.clone()).await;
                crate::app_state::skills::refresh_skills_ui(ui_weak).await;
            });
        });
    });
}

/// Per-workspace skill override cycle callback for Settings > Skills panel.
pub(crate) fn register_set_skill_workspace_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_set_skill_workspace(move |skill_id, override_val| {
        let skill_id_str = skill_id.to_string();
        let override_str = override_val.to_string();
        let ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(target: "app_state", "set-skill-workspace: failed to build runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                use northhing_kernel_api::agents::ProjectSkillEntry;
                use northhing_kernel_api::KernelAgentsApi;

                let facade = kernel_facade();
                let mut doc = match facade.load_project_skills().await {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(target: "app_state", "load_project_skills failed: {e}");
                        set_banner_message(ui_weak.clone(), format!("读取工作区技能配置失败: {e}"), "");
                        return;
                    }
                };

                doc.skills.retain(|s| s.skill_id != skill_id_str);
                if override_str == "on" {
                    doc.skills.push(ProjectSkillEntry {
                        skill_id: skill_id_str.clone(),
                        enabled: true,
                        config: None,
                    });
                } else if override_str == "off" {
                    doc.skills.push(ProjectSkillEntry {
                        skill_id: skill_id_str.clone(),
                        enabled: false,
                        config: None,
                    });
                }

                if let Err(e) = facade.save_project_skills(doc).await {
                    tracing::warn!(target: "app_state", "save_project_skills failed: {e}");
                    set_banner_message(ui_weak.clone(), format!("保存工作区技能配置失败: {e}"), "");
                    return;
                }

                refresh_settings_lists(ui_weak.clone()).await;
                crate::app_state::skills::refresh_skills_ui(ui_weak).await;
            });
        });
    });
}
