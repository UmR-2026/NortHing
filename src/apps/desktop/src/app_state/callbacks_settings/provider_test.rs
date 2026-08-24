use crate::app_state::settings::provider_wire_format_from_str;
use crate::app_state::slint_glue::AppWindow;
use crate::app_state::state::AppState;
use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::settings::ProviderFormDto;
use northhing_kernel_api::KernelSettingsApi;
use slint::ComponentHandle;
use std::sync::Arc;

pub(crate) fn register_test_provider_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_test_provider(move |id| {
        let id_str = id.to_string();
        let ui_weak2 = ui_weak.clone();
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
                let facade = kernel_facade();
                let resolved_id = if id_str == "__last__" {
                    let models = facade.list_model_configs().await.unwrap_or_default();
                    let rid = models.last().map(|p| p.id.clone()).unwrap_or_default();
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

                if resolved_id.is_empty() {
                    let ui_weak3 = ui_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak3.upgrade() {
                            ui.set_provider_test_in_flight(false);
                            ui.set_provider_test_result(slint::SharedString::from("未找到要测试的服务"));
                        }
                    });
                    return;
                }

                match facade.test_provider(&resolved_id).await {
                    Ok(result) => {
                        let result_str = if result.success {
                            "ok".to_string()
                        } else {
                            let detail = result.error.unwrap_or_default();
                            let first_line = detail.lines().next().unwrap_or("").trim();
                            if first_line.is_empty() {
                                "连接失败".to_string()
                            } else {
                                first_line.chars().take(120).collect()
                            }
                        };
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

pub(crate) fn register_test_provider_config_callback(ui: &AppWindow, _app_state: &Arc<AppState>) {
    let ui_weak = ui.as_weak();
    ui.on_test_provider_config(move |name, ptype, base_url, api_key, model, _enabled| {
        let ui_weak2 = ui_weak.clone();
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
                let wire_provider = provider_wire_format_from_str(ptype.as_str());
                let facade = kernel_facade();
                let form = ProviderFormDto {
                    provider_id: name.to_string(),
                    base_url: Some(base_url.to_string()),
                    api_key: Some(api_key.to_string()),
                    model: Some(model.to_string()),
                    provider_type: Some(wire_provider.to_string()),
                };
                match facade.test_provider_config(form).await {
                    Ok(result) => {
                        let result_str = if result.success {
                            "ok".to_string()
                        } else {
                            let detail = result.error.unwrap_or_default();
                            let first_line = detail.lines().next().unwrap_or("").trim();
                            if first_line.is_empty() {
                                "连接失败".to_string()
                            } else {
                                first_line.chars().take(120).collect()
                            }
                        };
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
