use super::load_app_settings_quiet;
use super::refresh_settings_lists;
use crate::app_state::error_banners::{set_banner_message, set_inline_error};
use crate::app_state::settings::{
    delete_api_key, provider_wire_format_from_str, resolve_effective_api_key, validate_provider_input, KeyringBackend,
    PRODUCTION_KEYRING,
};
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::KernelSettingsApi;
use slint::ComponentHandle;
use std::sync::Arc;

pub(crate) fn register_delete_provider_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    let app_state_arc_del_prov = std::sync::Arc::clone(&app_state);
    let ui_weak_del_prov = ui.as_weak();
    ui.on_delete_provider(move |provider_id| {
        let pid = provider_id.to_string();
        let app_state = Arc::clone(&app_state_arc_del_prov);
        let ui_weak = ui_weak_del_prov.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "failed to build runtime for delete-provider: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                let facade = kernel_facade();

                // Step 1: remove the model from core config via facade
                if let Err(e) = facade.delete_model_config(&pid).await {
                    tracing::warn!(target: "app_state", "delete-provider delete_model_config failed: {e}");
                    set_banner_message(ui_weak.clone(), "删除 AI 服务失败，请重试".to_string(), "");
                    return;
                }

                // Step 2: delete keyring entry for this provider
                if let Err(e) = delete_api_key(&*PRODUCTION_KEYRING, &pid) {
                    tracing::warn!(target: "app_state", "delete_api_key failed for {pid}: {e}");
                }

                // Step 3: run integrity check against remaining models
                let existing_models = facade.list_model_configs().await.unwrap_or_default();
                let known_ids: std::collections::HashSet<String> =
                    existing_models.iter().map(|m| m.id.clone()).collect();
                let s = load_app_settings_quiet().await.unwrap_or_default();

                let snapshot = app_state.session_metadata_snapshot();
                let session_ids: Vec<String> = snapshot.iter().map(|(id, _)| id.clone()).collect();
                let provider_lookup = |sid: &str| -> Option<String> {
                    snapshot
                        .iter()
                        .find(|(id, _)| id == sid)
                        .map(|(_, m)| m.provider_id.clone())
                };
                let workspace_lookup = |sid: &str| -> Option<std::path::PathBuf> {
                    snapshot
                        .iter()
                        .find(|(id, _)| id == sid)
                        .map(|(_, m)| m.workspace_path.clone())
                };
                let issues = s.validate_session_integrity(&known_ids, session_ids, &provider_lookup, &workspace_lookup);

                let q6_count = issues.iter().filter(|i| i.kind == "provider-deleted").count();
                if q6_count > 0 {
                    let fallback = existing_models.iter().find(|m| m.enabled == Some(true));
                    let detail = match fallback {
                        Some(fb) => format!(
                            "新会话将自动使用 {} ({} 个会话受影响)。",
                            fb.display_name.as_deref().unwrap_or(&fb.id),
                            q6_count
                        ),
                        None => {
                            format!("没有其他可用的 AI 服务。{} 个会话无法继续。", q6_count)
                        }
                    };
                    set_banner_message(ui_weak.clone(), "已删除 AI 服务".to_string(), detail);
                    set_inline_error(ui_weak.clone(), "上次使用的 AI 服务已被移除，已自动切换。");
                } else {
                    set_banner_message(ui_weak.clone(), "已删除 AI 服务".to_string(), "");
                }

                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}

pub(crate) fn register_upsert_provider_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    let app_state_arc_upsert_prov = std::sync::Arc::clone(&app_state);
    let ui_weak_upsert_prov = ui.as_weak();
    ui.on_upsert_provider(move |id, name, type_str, base_url, api_key, model, enabled| {
        let pid = if id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            id.to_string()
        };
        let pname = name.to_string();
        let ptype = type_str.to_string();
        let pbase = base_url.to_string();
        let pkey = api_key.to_string();
        let pmodel = model.to_string();
        let penabled = enabled;
        let app_state = Arc::clone(&app_state_arc_upsert_prov);
        let ui_weak = ui_weak_upsert_prov.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(_) => return,
            };
            rt.block_on(async move {
                // Resolve effective API key (if editing with empty key field, reuse stored keyring key)
                let effective_key = if !id.is_empty() && pkey.trim().is_empty() {
                    resolve_effective_api_key(PRODUCTION_KEYRING.get(&pid).ok().as_deref(), &pkey)
                } else {
                    pkey.clone()
                };

                if let Err(msg) = validate_provider_input(&pname, &ptype, &pbase, &effective_key, &pmodel) {
                    set_inline_error(ui_weak.clone(), msg.clone());
                    set_banner_message(ui_weak.clone(), msg, "");
                    return;
                }

                // Store in OS keyring (fail-closed)
                if !effective_key.is_empty() {
                    if let Err(e) = PRODUCTION_KEYRING.store(&pid, &effective_key) {
                        tracing::warn!(target: "app_state", "keyring store failed for {pid}: {e}");
                        set_inline_error(ui_weak.clone(), "密钥存储失败，请重试".to_string());
                        return;
                    }
                }

                let wire_provider = provider_wire_format_from_str(&ptype);
                let model_dto = northhing_kernel_api::settings::AIModelConfigDto {
                    id: pid.clone(),
                    provider_id: wire_provider.to_string(),
                    model: pmodel.clone(),
                    display_name: Some(pname.clone()),
                    max_tokens: None,
                    temperature: None,
                    base_url: Some(pbase.clone()),
                    enabled: Some(penabled),
                    category: Some("general_chat".to_string()),
                    capabilities: Some(vec!["text_chat".to_string(), "function_calling".to_string()]),
                    auth: Some("api_key".to_string()),
                    inline_think_in_text: Some(true),
                };

                let facade = kernel_facade();
                // Scheme C write-only key channel: the key rides the explicit
                // parameter, never the DTO shape.
                if let Err(e) = facade.upsert_model_config(model_dto, Some(effective_key)).await {
                    tracing::warn!(target: "app_state", "upsert-provider upsert_model_config failed: {e}");
                    set_inline_error(ui_weak.clone(), "保存配置失败，请重试".to_string());
                    return;
                }

                // If no default provider exists and this one is enabled, set default
                if penabled {
                    if let Ok(cfg) = facade.get_global_config().await {
                        if cfg.default_provider_id.is_none() {
                            let _ = facade.set_default_provider(&pid).await;
                        }
                    }
                }

                let _ = app_state; // kept for integrity / metadata
                set_banner_message(ui_weak.clone(), format!("已保存 AI 服务 {}", pname), "");

                let ui_weak_set_id = ui_weak.clone();
                let saved_id = pid.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_set_id.upgrade() {
                        ui.set_last_saved_provider_id(slint::SharedString::from(saved_id));
                    }
                });

                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}
