// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W7-2 (2026-08-28) — Settings provider edit modal UI (F7 surface).
//
// Modal dialog component for editing existing LLM provider configurations,
// supporting explicit provider_type selection, connection testing,
// two-step deletion confirmation, and keyring-integrated key updates.

use dioxus::prelude::*;
use northhing_kernel_api::settings::{ProviderConfigDto, ProviderFormDto};

use crate::app_state::settings::{KeyringBackend, PRODUCTION_KEYRING};

/// Supported provider types displayed in the dropdown.
pub const SUPPORTED_PROVIDER_TYPES: &[(&str, &str)] = &[
    ("anthropic", "Anthropic"),
    ("openai", "OpenAI"),
    ("gemini", "Gemini"),
    ("custom-openai", "自定义 (OpenAI Compatible)"),
    ("custom-anthropic", "自定义 (Anthropic Compatible)"),
];

/// Returns the default base URL for a given provider type string.
pub fn default_base_url_for_type(t: &str) -> &'static str {
    match t {
        "anthropic" => "https://api.anthropic.com/v1",
        "openai" => "https://api.openai.com/v1",
        "gemini" => "https://generativelanguage.googleapis.com",
        "custom-openai" | "custom-anthropic" => "",
        _ => "",
    }
}

/// Checks whether a given base URL is a recognized standard default base URL.
pub fn is_known_default_url(url: &str) -> bool {
    let u = url.trim();
    u.is_empty()
        || u == "https://api.anthropic.com/v1"
        || u == "https://api.anthropic.com"
        || u == "https://api.openai.com/v1"
        || u == "https://generativelanguage.googleapis.com"
        || u == "https://generativelanguage.googleapis.com/v1beta"
}

#[derive(Props, Clone)]
pub struct ProviderEditModalProps {
    pub provider: ProviderConfigDto,
    pub on_close: EventHandler<()>,
    pub on_saved: EventHandler<()>,
}

/// Equality is based on underlying provider config field values.
///
/// `ProviderConfigDto` does not implement `PartialEq` in the core contract crate,
/// and we intentionally avoid modifying cross-crate DTO contracts for UI-specific needs.
/// Instead, equality is checked field-by-field on the DTO properties.
///
/// The `EventHandler` callbacks (`on_close`, `on_saved`) are intentionally omitted
/// because closure instances are recreated on every render pass, making equality checks
/// meaningless. This follows the precedent in `registry.rs` (`ModuleAppProps`) where
/// callbacks and dynamic channels are excluded from prop diffing equality.
impl PartialEq for ProviderEditModalProps {
    fn eq(&self, other: &Self) -> bool {
        self.provider.id == other.provider.id
            && self.provider.name == other.provider.name
            && self.provider.base_url == other.provider.base_url
            && self.provider.model == other.provider.model
            && self.provider.enabled == other.provider.enabled
            && self.provider.provider_type == other.provider.provider_type
    }
}

#[component]
pub fn ProviderEditModal(props: ProviderEditModalProps) -> Element {
    let initial_provider = props.provider.clone();
    let initial_type = initial_provider
        .provider_type
        .clone()
        .unwrap_or_else(|| "openai".to_string());

    let mut name_input = use_signal(|| initial_provider.name.clone());
    let mut provider_type_input = use_signal(|| initial_type);
    let mut base_url_input = use_signal(|| initial_provider.base_url.clone());
    let mut model_input = use_signal(|| initial_provider.model.clone());
    let mut api_key_input = use_signal(String::new);
    let mut enabled_input = use_signal(|| initial_provider.enabled.unwrap_or(true));

    let mut testing = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let mut confirming_delete = use_signal(|| false);

    // Test connection feedback: (success_flag, display_text)
    let mut test_message = use_signal(|| Option::<(bool, String)>::None);
    // General error message for save/delete operations
    let mut error_message = use_signal(|| Option::<String>::None);

    let is_busy = testing() || saving() || deleting();

    // Test connection handler
    let run_test = {
        let id = initial_provider.id.clone();
        move |_| {
            if is_busy {
                return;
            }
            testing.set(true);
            test_message.set(Some((true, "测试连接中...".to_string())));
            error_message.set(None);

            let id = id.clone();
            let type_val = provider_type_input.read().clone();
            let url_val = base_url_input.read().trim().to_string();
            let key_val = api_key_input.read().clone();
            let model_val = model_input.read().trim().to_string();

            let mut testing = testing;
            let mut test_message = test_message;

            spawn(async move {
                // If API key is blank in UI, fall back to stored key from keyring
                let effective_key = if !key_val.trim().is_empty() {
                    Some(key_val)
                } else {
                    PRODUCTION_KEYRING.get(&id).ok()
                };

                let form = ProviderFormDto {
                    provider_id: id,
                    base_url: if url_val.is_empty() { None } else { Some(url_val) },
                    api_key: effective_key,
                    model: if model_val.is_empty() { None } else { Some(model_val) },
                    provider_type: Some(crate::app_state::settings::provider_wire_format_from_str(&type_val).to_string()),
                };

                match super::api::test_provider_config(form).await {
                    Ok(res) if res.success => {
                        test_message.set(Some((true, "✓ 连接成功".to_string())));
                    }
                    Ok(res) => {
                        let err = res.error.unwrap_or_else(|| "连接失败".to_string());
                        let first_line = err.lines().next().unwrap_or(&err).trim().to_string();
                        test_message.set(Some((false, format!("✗ 测试失败: {first_line}"))));
                    }
                    Err(err) => {
                        let err_str = err.to_string();
                        let first_line = err_str.lines().next().unwrap_or(&err_str).trim().to_string();
                        test_message.set(Some((false, format!("✗ 测试失败: {first_line}"))));
                    }
                }
                testing.set(false);
            });
        }
    };

    // Save handler
    let run_save = {
        let id = initial_provider.id.clone();
        let on_saved = props.on_saved;
        move |_| {
            if is_busy {
                return;
            }
            saving.set(true);
            error_message.set(None);
            test_message.set(None);

            let id = id.clone();
            let name_val = name_input.read().trim().to_string();
            let type_val = provider_type_input.read().clone();
            let url_val = base_url_input.read().trim().to_string();
            let key_val = api_key_input.read().clone();
            let model_val = model_input.read().trim().to_string();
            let enabled_val = enabled_input();

            let mut saving = saving;
            let mut error_message = error_message;

            spawn(async move {
                match super::api::edit_provider(
                    &id,
                    &name_val,
                    &type_val,
                    &url_val,
                    &key_val,
                    &model_val,
                    enabled_val,
                )
                .await
                {
                    Ok(()) => {
                        saving.set(false);
                        on_saved.call(());
                    }
                    Err(err) => {
                        let first_line = err.lines().next().unwrap_or(&err).trim().to_string();
                        error_message.set(Some(format!("保存失败: {first_line}")));
                        saving.set(false);
                    }
                }
            });
        }
    };

    // Delete handler
    let run_delete = {
        let id = initial_provider.id.clone();
        let on_saved = props.on_saved;
        move |_| {
            if is_busy {
                return;
            }
            deleting.set(true);
            error_message.set(None);
            test_message.set(None);

            let id = id.clone();
            let mut deleting = deleting;
            let mut error_message = error_message;
            let mut confirming_delete = confirming_delete;

            spawn(async move {
                match super::api::delete_provider(&id).await {
                    Ok(()) => {
                        deleting.set(false);
                        on_saved.call(());
                    }
                    Err(err) => {
                        let first_line = err.lines().next().unwrap_or(&err).trim().to_string();
                        error_message.set(Some(format!("删除失败: {first_line}")));
                        deleting.set(false);
                        confirming_delete.set(false);
                    }
                }
            });
        }
    };

    let on_close_cb = props.on_close;

    rsx! {
        // Modal overlay backdrop
        div {
            class: "provider-edit-overlay",
            style: "position: fixed; inset: 0; background: rgba(0, 0, 0, 0.65); display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: blur(2px);",
            onclick: move |_| {
                if !is_busy {
                    on_close_cb.call(());
                }
            },
            // Modal dialog container
            div {
                class: "provider-edit-dialog",
                style: "background: var(--bg1); border: 1px solid var(--line); border-radius: 8px; width: 440px; max-width: 90vw; padding: 20px; box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45); display: flex; flex-direction: column; gap: 12px; color: var(--text); font-family: inherit;",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--line); padding-bottom: 8px;",
                    span {
                        style: "font-size: 14px; font-weight: 600; color: var(--text);",
                        "编辑 AI 服务"
                    }
                    button {
                        style: "background: none; border: none; color: var(--faint); font-size: 14px; cursor: pointer; padding: 0 4px; line-height: 1;",
                        title: "关闭",
                        disabled: is_busy,
                        onclick: move |_| on_close_cb.call(()),
                        "✕"
                    }
                }

                // Error feedback banner (save / delete / validation error)
                if let Some(err) = error_message() {
                    div {
                        style: "background: rgba(239, 68, 68, 0.12); border: 1px solid var(--danger, #ef4444); color: var(--danger, #ef4444); padding: 8px 10px; border-radius: 4px; font-size: 12px; word-break: break-word;",
                        "{err}"
                    }
                }

                // Test connection feedback banner
                if let Some((is_ok, msg)) = test_message() {
                    div {
                        style: if is_ok {
                            "background: rgba(34, 197, 94, 0.12); border: 1px solid #22c55e; color: #22c55e; padding: 8px 10px; border-radius: 4px; font-size: 12px; word-break: break-word;"
                        } else {
                            "background: rgba(239, 68, 68, 0.12); border: 1px solid var(--danger, #ef4444); color: var(--danger, #ef4444); padding: 8px 10px; border-radius: 4px; font-size: 12px; word-break: break-word;"
                        },
                        "{msg}"
                    }
                }

                // Form fields
                div {
                    style: "display: flex; flex-direction: column; gap: 10px;",

                    // Name
                    div {
                        label {
                            style: "font-size: 12px; color: var(--faint); margin-bottom: 3px; display: block;",
                            "名称"
                        }
                        input {
                            style: "width: 100%; box-sizing: border-box; background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; color: var(--text); padding: 6px 10px; font-size: 12px; outline: none;",
                            r#type: "text",
                            value: "{name_input}",
                            disabled: is_busy,
                            oninput: move |e| name_input.set(e.value()),
                        }
                    }

                    // Provider Type Dropdown
                    div {
                        label {
                            style: "font-size: 12px; color: var(--faint); margin-bottom: 3px; display: block;",
                            "类型"
                        }
                        select {
                            style: "width: 100%; box-sizing: border-box; background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; color: var(--text); padding: 6px 10px; font-size: 12px; outline: none;",
                            value: "{provider_type_input}",
                            disabled: is_busy,
                            onchange: move |e| {
                                let old_type = provider_type_input.read().clone();
                                let new_type = e.value();
                                provider_type_input.set(new_type.clone());

                                let cur_url = base_url_input.read().clone();
                                let old_def = default_base_url_for_type(&old_type);
                                if cur_url.trim().is_empty() || cur_url.trim() == old_def || is_known_default_url(&cur_url) {
                                    let new_def = default_base_url_for_type(&new_type);
                                    base_url_input.set(new_def.to_string());
                                }
                            },
                            for (val, label_text) in SUPPORTED_PROVIDER_TYPES {
                                option {
                                    key: "{val}",
                                    value: "{val}",
                                    selected: provider_type_input() == *val,
                                    "{label_text}"
                                }
                            }
                        }
                    }

                    // Base URL
                    div {
                        label {
                            style: "font-size: 12px; color: var(--faint); margin-bottom: 3px; display: block;",
                            "Base URL"
                        }
                        input {
                            style: "width: 100%; box-sizing: border-box; background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; color: var(--text); padding: 6px 10px; font-size: 12px; outline: none;",
                            r#type: "text",
                            value: "{base_url_input}",
                            placeholder: "https://api.openai.com/v1",
                            disabled: is_busy,
                            oninput: move |e| base_url_input.set(e.value()),
                        }
                    }

                    // Model
                    div {
                        label {
                            style: "font-size: 12px; color: var(--faint); margin-bottom: 3px; display: block;",
                            "模型"
                        }
                        input {
                            style: "width: 100%; box-sizing: border-box; background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; color: var(--text); padding: 6px 10px; font-size: 12px; outline: none;",
                            r#type: "text",
                            value: "{model_input}",
                            placeholder: "例如 claude-3-7-sonnet / gpt-4o",
                            disabled: is_busy,
                            oninput: move |e| model_input.set(e.value()),
                        }
                    }

                    // API Key
                    div {
                        label {
                            style: "font-size: 12px; color: var(--faint); margin-bottom: 3px; display: block;",
                            "API Key"
                        }
                        input {
                            style: "width: 100%; box-sizing: border-box; background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; color: var(--text); padding: 6px 10px; font-size: 12px; outline: none;",
                            r#type: "password",
                            value: "{api_key_input}",
                            placeholder: "留空 = 保持不变",
                            disabled: is_busy,
                            oninput: move |e| api_key_input.set(e.value()),
                        }
                        div {
                            style: "font-size: 11px; color: var(--faint); margin-top: 2px;",
                            "留空 = 保持已有密钥不变"
                        }
                    }

                    // Enabled toggle
                    div {
                        style: "display: flex; align-items: center; gap: 8px; margin-top: 4px; cursor: pointer;",
                        onclick: move |_| {
                            if !is_busy {
                                enabled_input.toggle();
                            }
                        },
                        input {
                            r#type: "checkbox",
                            checked: enabled_input(),
                            disabled: is_busy,
                            style: "cursor: pointer;",
                        }
                        span {
                            style: "font-size: 12px; color: var(--text);",
                            "启用此服务"
                        }
                    }
                }

                // Action buttons / Two-step delete confirmation
                div {
                    style: "border-top: 1px solid var(--line); padding-top: 12px; margin-top: 4px;",
                    if confirming_delete() {
                        div {
                            style: "display: flex; flex-direction: column; gap: 8px;",
                            div {
                                style: "font-size: 12px; color: var(--danger, #ef4444);",
                                "确定删除该服务？此操作不可撤销。"
                            }
                            div {
                                style: "display: flex; justify-content: flex-end; gap: 8px;",
                                button {
                                    style: "background: var(--bg3); border: 1px solid var(--line); color: var(--text); padding: 5px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;",
                                    disabled: is_busy,
                                    onclick: move |_| confirming_delete.set(false),
                                    "取消"
                                }
                                button {
                                    style: "background: var(--danger, #ef4444); border: 1px solid transparent; color: #fff; padding: 5px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;",
                                    disabled: is_busy,
                                    onclick: run_delete,
                                    if deleting() { "删除中..." } else { "确定删除" }
                                }
                            }
                        }
                    } else {
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center;",
                            button {
                                style: "background: transparent; border: 1px solid var(--danger, #ef4444); color: var(--danger, #ef4444); padding: 5px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;",
                                disabled: is_busy,
                                onclick: move |_| confirming_delete.set(true),
                                "删除"
                            }
                            div {
                                style: "display: flex; gap: 8px;",
                                button {
                                    style: "background: var(--bg3); border: 1px solid var(--line); color: var(--text); padding: 5px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;",
                                    disabled: is_busy,
                                    onclick: run_test,
                                    if testing() { "测试中..." } else { "测试连接" }
                                }
                                button {
                                    style: "background: var(--accent-solid, #3b82f6); border: 1px solid transparent; color: #fff; padding: 5px 14px; border-radius: 4px; font-size: 12px; font-weight: 500; cursor: pointer;",
                                    disabled: is_busy,
                                    onclick: run_save,
                                    if saving() { "保存中..." } else { "保存" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_base_url_mapping() {
        assert_eq!(default_base_url_for_type("anthropic"), "https://api.anthropic.com/v1");
        assert_eq!(default_base_url_for_type("openai"), "https://api.openai.com/v1");
        assert_eq!(default_base_url_for_type("gemini"), "https://generativelanguage.googleapis.com");
        assert_eq!(default_base_url_for_type("custom-openai"), "");
        assert_eq!(default_base_url_for_type("custom-anthropic"), "");
        assert_eq!(default_base_url_for_type("unknown"), "");
    }

    #[test]
    fn test_is_known_default_url() {
        assert!(is_known_default_url(""));
        assert!(is_known_default_url("  "));
        assert!(is_known_default_url("https://api.anthropic.com/v1"));
        assert!(is_known_default_url("https://api.anthropic.com"));
        assert!(is_known_default_url("https://api.openai.com/v1"));
        assert!(is_known_default_url("https://generativelanguage.googleapis.com"));
        assert!(is_known_default_url("https://generativelanguage.googleapis.com/v1beta"));
        assert!(!is_known_default_url("https://my-custom-proxy.com/v1"));
    }

    #[test]
    fn test_supported_provider_types_coverage() {
        let types: Vec<&str> = SUPPORTED_PROVIDER_TYPES.iter().map(|(t, _)| *t).collect();
        assert!(types.contains(&"anthropic"));
        assert!(types.contains(&"openai"));
        assert!(types.contains(&"gemini"));
        assert!(types.contains(&"custom-openai"));
        assert!(types.contains(&"custom-anthropic"));
    }
}
