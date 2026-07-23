use super::load_app_settings_quiet;
use super::save_app_settings_quiet;
use crate::app_state::settings::{now_unix_secs, provider_to_ai_model_config, ProviderConfig};
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use northhing_core::infrastructure::ai::AIClient;
use northhing_core::util::types::AIConfig;
use slint::{ComponentHandle, SharedString};
use std::sync::Arc;

// 2026-06-26 (Phase 4 fix): test-provider handler. Resolves the provider
// id ("__last__" → the most recently saved provider), builds an AIClient
// from the stored config, and runs `test_connection()` on a background
// thread. Progress is surfaced via the bound `provider-test-in-flight` and
// `provider-test-result` properties.
pub(crate) fn register_test_provider_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_test_provider(move |id| {
        let id_str = id.to_string();
        let ui_weak2 = ui_weak.clone();
        // Flip to in-flight immediately on the UI thread (the callback
        // itself runs on the event loop, so a direct set is safe here).
        if let Some(ui) = ui_weak2.upgrade() {
            ui.set_provider_test_in_flight(true);
            ui.set_provider_test_result(slint::SharedString::from(""));
        }
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "test-provider: failed to build runtime: {e}"
                    );
                    let ui_weak3 = ui_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak3.upgrade() {
                            ui.set_provider_test_in_flight(false);
                            ui.set_provider_test_result(slint::SharedString::from("内部错误：无法启动运行时"));
                        }
                    });
                    return;
                }
            };
            rt.block_on(async move {
                let mut s = match load_app_settings_quiet().await {
                    Ok(s) => s,
                    Err(e) => {
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(e));
                            }
                        });
                        return;
                    }
                };
                // Resolve "__last__" sentinel to the last saved provider id.
                let resolved_id = if id_str == "__last__" {
                    let rid = s.providers.last().map(|p| p.id.clone()).unwrap_or_default();
                    let rid_for_ui = rid.clone();
                    let ui_weak3 = ui_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak3.upgrade() {
                            ui.set_last_saved_provider_id(slint::SharedString::from(rid_for_ui));
                        }
                    });
                    rid
                } else {
                    id_str.clone()
                };
                let provider = match s.providers.iter().find(|p| p.id == resolved_id) {
                    Some(p) => p.clone(),
                    None => {
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from("未找到要测试的服务"));
                            }
                        });
                        return;
                    }
                };
                // Build an AIClient from the stored provider config.
                let model_config = provider_to_ai_model_config(&provider);
                let ai_config = match AIConfig::try_from(model_config) {
                    Ok(c) => c,
                    Err(e) => {
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(e));
                            }
                        });
                        return;
                    }
                };
                let client = AIClient::new(ai_config);
                match client.test_connection().await {
                    Ok(result) => {
                        let result_str = if result.success {
                            "ok".to_string()
                        } else {
                            let detail = result.error_details.unwrap_or_default();
                            // Take the first line, cap at 120 chars.
                            let first_line = detail.lines().next().unwrap_or("").trim();
                            if first_line.is_empty() {
                                "连接失败".to_string()
                            } else {
                                first_line.chars().take(120).collect()
                            }
                        };
                        // Persist verification state on the provider.
                        if let Some(slot) = s.providers.iter_mut().find(|p| p.id == resolved_id) {
                            slot.last_verified_at = Some(now_unix_secs());
                            slot.last_verified_ok = Some(result.success);
                        }
                        let _ = save_app_settings_quiet(&s).await;
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(result_str));
                            }
                        });
                    }
                    Err(e) => {
                        let detail = format!("{e}");
                        let first_line = detail.lines().next().unwrap_or("").trim();
                        let result_str = if first_line.is_empty() {
                            "连接失败".to_string()
                        } else {
                            first_line.chars().take(120).collect()
                        };
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(result_str));
                            }
                        });
                    }
                }
            });
        });
    });
}

// 2026-07-18 (D2a+1): test-provider-config — race-free variant that tests
// an in-memory config directly without reading disk or resolving "__last__".
// The WelcomeView test button calls this instead of test-provider to avoid
// the race where test_provider tries to read a provider that upsert-provider
// has not yet flushed to disk.
pub(crate) fn register_test_provider_config_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_test_provider_config(move |name, ptype, base_url, api_key, model, enabled| {
        let ui_weak2 = ui_weak.clone();
        // Flip to in-flight immediately on the UI thread (the callback
        // itself runs on the event loop, so a direct set is safe here).
        if let Some(ui) = ui_weak2.upgrade() {
            ui.set_provider_test_in_flight(true);
            ui.set_provider_test_result(slint::SharedString::from(""));
        }
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!(
                        target: "app_state",
                        "test-provider-config: failed to build runtime: {e}"
                    );
                    let ui_weak3 = ui_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak3.upgrade() {
                            ui.set_provider_test_in_flight(false);
                            ui.set_provider_test_result(slint::SharedString::from("内部错误：无法启动运行时"));
                        }
                    });
                    return;
                }
            };
            rt.block_on(async move {
                // Parse provider type from string — same mapping as register_upsert_provider_callback.
                use crate::app_state::settings::ProviderType;
                let provider_type = match ptype.as_str() {
                    "anthropic" => ProviderType::Anthropic,
                    "openai" => ProviderType::Openai,
                    "gemini" => ProviderType::Gemini,
                    "custom-openai" => ProviderType::CustomOpenaiCompatible,
                    "custom-anthropic" => ProviderType::CustomAnthropicCompatible,
                    _ => {
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from("内部错误：未知的服务类型"));
                            }
                        });
                        return;
                    }
                };
                // Build an in-memory ProviderConfig — no disk read, no slot to write.
                let mut provider = ProviderConfig::new(name.to_string(), provider_type);
                provider.base_url = base_url.to_string();
                provider.api_key = api_key.to_string();
                provider.model = model.to_string();
                provider.enabled = enabled;
                // Reuse the existing test chain: provider → model_config → AIConfig → AIClient.
                let model_config = provider_to_ai_model_config(&provider);
                let ai_config = match AIConfig::try_from(model_config) {
                    Ok(c) => c,
                    Err(e) => {
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(e));
                            }
                        });
                        return;
                    }
                };
                let client = AIClient::new(ai_config);
                match client.test_connection().await {
                    Ok(result) => {
                        let result_str = if result.success {
                            "ok".to_string()
                        } else {
                            let detail = result.error_details.unwrap_or_default();
                            let first_line = detail.lines().next().unwrap_or("").trim();
                            if first_line.is_empty() {
                                "连接失败".to_string()
                            } else {
                                first_line.chars().take(120).collect()
                            }
                        };
                        // No disk write — result is returned to UI only.
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(result_str));
                            }
                        });
                    }
                    Err(e) => {
                        let detail = format!("{e}");
                        let first_line = detail.lines().next().unwrap_or("").trim();
                        let result_str = if first_line.is_empty() {
                            "连接失败".to_string()
                        } else {
                            first_line.chars().take(120).collect()
                        };
                        let ui_weak3 = ui_weak2.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak3.upgrade() {
                                ui.set_provider_test_in_flight(false);
                                ui.set_provider_test_result(slint::SharedString::from(result_str));
                            }
                        });
                    }
                }
            });
        });
    });
}
