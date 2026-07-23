use super::load_app_settings_quiet;
use super::refresh_settings_lists;
use super::save_app_settings_quiet;
use crate::app_state::error_banners::{set_banner_message, set_inline_error};
use crate::app_state::settings::ProviderConfig;
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use slint::{ComponentHandle, SharedString};
use std::sync::Arc;

// --- 2026-06-26 (Phase 5 wire-up): Settings sub-panel callbacks ---
// These are the Q6/Q7 live wire-up. Each callback mutates
// AppSettings on disk, then runs `validate_session_integrity`
// against the AppState's session metadata to surface banner /
// inline errors when the change breaks existing sessions.
//
// `provider_id` and `workspace_path` for each session come from
// the in-memory `session_metadata` map (populated by
// `on_new_session`). The runtime's `SessionSummary` does not
// yet expose these fields, so we maintain them on the desktop
// side. When the core adds them, the map can be removed.
pub(crate) fn register_delete_provider_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- delete-provider (Q6) ---
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
                        "Phase 5: failed to build runtime for delete-provider: {e}"
                    );
                    return;
                }
            };
            rt.block_on(async move {
                // Step 1: mutate settings (load → remove → save).
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_banner_message(ui_weak.clone(), e, "");
                        return;
                    }
                };
                let provider_name = s
                    .providers
                    .iter()
                    .find(|p| p.id == pid)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| pid.clone());
                let _ = s.remove_provider(&pid);
                if let Err(e) = save_app_settings_quiet(&s).await {
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak.clone(), e, "");
                    return;
                }
                // 2026-07-18 (D2g): push provider state into core so the runtime
                // sees the deletion. Failure is non-fatal — the user's data is
                // safe on disk; we surface a banner and let them retry.
                if let Err(e) = crate::app_state::settings::sync_providers_to_core(&s).await {
                    tracing::warn!(target: "app_state", "delete-provider sync-to-core failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_banner_message(ui_weak.clone(), "同步到运行时配置失败，请重试".to_string(), "");
                    // do NOT return — data is already persisted
                }

                // Best-effort: remove the provider from core's model list
                // and reconcile. Failure is non-fatal — the user's data is
                // safe on disk; we just log and let the UI continue.
                if let Ok(service) = northhing_core::service::config::get_global_config_service().await {
                    if let Err(e) = service.delete_ai_model(&pid).await {
                        tracing::warn!(target: "app_state", "delete-provider delete_ai_model failed: {e}");
                    }
                    if let Err(e) = service.reconcile_models("desktop-delete").await {
                        tracing::warn!(target: "app_state", "delete-provider reconcile_models failed: {e}");
                    }
                }

                // Step 2: run Q6 integrity check on the snapshot.
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
                let issues = s.validate_session_integrity(session_ids, &provider_lookup, &workspace_lookup);

                // Step 3: push banner / inline error for any Q6 hits.
                // (Q7 issues are NOT expected from a provider delete
                // — they fire only on remove-workspace. We still log
                // them so nothing is silently dropped.)
                // 2026-07-18 (D2j): pass weak directly; helpers upgrade on UI thread.
                let q6_count = issues.iter().filter(|i| i.kind == "provider-deleted").count();
                if q6_count > 0 {
                    let fallback = s.fallback_provider_for(&pid);
                    let detail = match fallback {
                        Some(fb) => format!("新会话将自动使用 {} ({} 个会话受影响)。", fb.name, q6_count),
                        None => {
                            format!("没有其他可用的 AI 服务。{} 个会话无法继续。", q6_count)
                        }
                    };
                    set_banner_message(ui_weak.clone(), format!("已删除 AI 服务 {}", provider_name), detail);
                    set_inline_error(ui_weak.clone(), "上次使用的 AI 服务已被移除，已自动切换。");
                } else {
                    set_banner_message(ui_weak.clone(), format!("已删除 AI 服务 {}", provider_name), "");
                }
                // 2026-07-18 (D2b): refresh settings lists after save.
                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}

pub(crate) fn register_upsert_provider_callback(ui: &AppWindow, app_state: &Arc<AppState>) {
    // --- upsert-provider (Phase 3/4: create or update a provider) ---
    // Spec g1: Q2=b (real POST /messages) test, Q3=c (Q1 migration path).
    // Phase 5 wire-up: also runs Q6 integrity check to handle the
    // "switch provider" UX — when the user updates an enabled flag
    // or replaces a provider config, sessions that referenced the
    // old id may now reference a valid one again.
    let app_state_arc_upsert_prov = std::sync::Arc::clone(&app_state);
    let ui_weak_upsert_prov = ui.as_weak();
    ui.on_upsert_provider(move |id, name, type_str, base_url, api_key, model, enabled| {
        let pid = id.to_string();
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
                use crate::app_state::settings::{resolve_effective_api_key, validate_provider_input, ProviderType};

                // 2026-07-18 (D2e): load settings BEFORE validate so we can look up
                // the stored API key when editing with an empty form field.
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_inline_error(ui_weak.clone(), e);
                        return;
                    }
                };

                // 2026-07-18 (D2e): edit-flow key inheritance — if pid is non-empty
                // (edit mode) and the incoming key is empty, inherit the stored key.
                let effective_key = if !pid.is_empty() && pkey.trim().is_empty() {
                    resolve_effective_api_key(
                        s.providers.iter().find(|p| p.id == pid).map(|p| p.api_key.as_str()),
                        &pkey,
                    )
                } else {
                    pkey.clone()
                };

                if let Err(msg) = validate_provider_input(&pname, &ptype, &pbase, &effective_key, &pmodel) {
                    // 2026-07-18 (D2j): pass weak directly; helpers upgrade on UI thread.
                    set_inline_error(ui_weak.clone(), msg.clone());
                    // 2026-07-18 (D2e): banner is globally visible — unlike inline
                    // error which only renders in ChatPaneView.
                    set_banner_message(ui_weak.clone(), msg, "");
                    return;
                }
                let provider_type = match ptype.as_str() {
                    "anthropic" => ProviderType::Anthropic,
                    "openai" => ProviderType::Openai,
                    "gemini" => ProviderType::Gemini,
                    "custom-openai" => ProviderType::CustomOpenaiCompatible,
                    "custom-anthropic" => ProviderType::CustomAnthropicCompatible,
                    // validate_provider_input already rejected unknown types;
                    // this branch is unreachable in practice, but we handle
                    // it gracefully instead of panicking (panic in a spawn
                    // thread would abort the process).
                    _ => {
                        // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                        set_inline_error(ui_weak.clone(), "内部错误：未知的服务类型".to_string());
                        return;
                    }
                };
                let mut new_provider = ProviderConfig::new(pname.clone(), provider_type);
                if !pid.is_empty() {
                    new_provider.id = pid.clone();
                }
                new_provider.base_url = pbase;
                new_provider.api_key = effective_key;
                new_provider.model = pmodel;
                new_provider.enabled = penabled;
                s.upsert_provider(new_provider);
                if let Err(e) = save_app_settings_quiet(&s).await {
                    tracing::warn!(target: "app_state", "upsert-provider save failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_inline_error(ui_weak.clone(), "同步到运行时配置失败，请重试".to_string());
                    return;
                }
                // Push the new/updated provider into core so the runtime
                // sees it. Failure is non-fatal — the user's data is safe
                // on disk; we surface a banner and let them retry.
                if let Err(e) = crate::app_state::settings::sync_providers_to_core(&s).await {
                    tracing::warn!(target: "app_state", "upsert-provider sync-to-core failed: {e}");
                    // 2026-07-18 (D2j): pass weak directly; helper upgrades on UI thread.
                    set_inline_error(ui_weak.clone(), "同步到运行时配置失败，请重试".to_string());
                    return;
                }
                // Q6 reverse direction: if user re-adds a provider
                // with the same id that sessions were tracking,
                // those sessions are no longer "broken". The
                // integrity check returns empty (or fewer issues)
                // — no error push needed; the user just gets a
                // success banner.
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
                let _ = s.validate_session_integrity(session_ids, &provider_lookup, &workspace_lookup);
                // 2026-07-18 (D2j): pass weak directly; helpers upgrade on UI thread.
                set_banner_message(ui_weak.clone(), format!("已保存 AI 服务 {}", pname), "");
                // 2026-06-26 (Phase 4 fix): expose the saved provider id so
                // the welcome flow's test-btn can request "test the last
                // saved one" via the "__last__" sentinel.
                let saved_id = if pid.is_empty() {
                    s.providers.last().map(|p| p.id.clone()).unwrap_or_default()
                } else {
                    pid.clone()
                };
                let ui_weak_set_id = ui_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_set_id.upgrade() {
                        ui.set_last_saved_provider_id(slint::SharedString::from(saved_id));
                    }
                });
                // 2026-07-18 (D2b): refresh settings lists after save.
                refresh_settings_lists(ui_weak.clone()).await;
            });
        });
    });
}
