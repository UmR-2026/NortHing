use super::keyring::{resolve_api_key, KeyringBackend};
use super::{ProviderConfig, ProviderType};

// 2026-07-18 (D2e): edit-flow key inheritance — empty incoming key on edit
// keeps the stored one; add-flow or non-empty key passes through.
pub fn resolve_effective_api_key(stored: Option<&str>, incoming: &str) -> String {
    if incoming.trim().is_empty() {
        stored.unwrap_or("").to_string()
    } else {
        incoming.to_string()
    }
}

// ===== Core sync helpers =====

/// Map a `ProviderType` to the wire-format `provider` string used by
/// `northhing-core`'s `AIModelConfig`.
#[allow(dead_code)]
pub fn provider_wire_format(t: &ProviderType) -> &'static str {
    match t {
        ProviderType::Anthropic => "anthropic",
        ProviderType::Openai => "openai",
        ProviderType::Gemini => "gemini",
        ProviderType::CustomOpenaiCompatible => "openai",
        ProviderType::CustomAnthropicCompatible => "anthropic",
    }
}

/// Map a provider type string to the wire-format `provider` string.
pub fn provider_wire_format_from_str(s: &str) -> &'static str {
    match s {
        "anthropic" | "custom-anthropic" => "anthropic",
        "openai" | "custom-openai" => "openai",
        "gemini" => "gemini",
        _ => "openai",
    }
}

/// Convert a desktop `ProviderConfig` into a facade `AIModelConfigDto`.
#[allow(dead_code)]
pub fn provider_to_ai_model_config(
    p: &ProviderConfig,
    keyring: &dyn KeyringBackend,
) -> northhing_kernel_api::settings::AIModelConfigDto {
    let resolved_key = resolve_api_key(keyring, &p.id, &p.api_key).unwrap_or_else(|_| p.api_key.clone());
    northhing_kernel_api::settings::AIModelConfigDto {
        id: p.id.clone(),
        provider_id: provider_wire_format(&p.provider_type).to_string(),
        model: p.model.clone(),
        display_name: Some(p.name.clone()),
        max_tokens: None,
        temperature: None,
        base_url: Some(p.base_url.clone()),
        api_key: Some(resolved_key),
        enabled: Some(p.enabled),
        category: Some("general_chat".to_string()),
        capabilities: Some(vec!["text_chat".to_string(), "function_calling".to_string()]),
        auth: Some("api_key".to_string()),
        inline_think_in_text: Some(true),
    }
}

/// Push resolved keyring keys to core in-memory models on startup or change (Scheme C).
/// Reads model list from core facade, resolves any missing / empty API keys
/// from the OS keyring for each model id, and pushes them into core memory
/// via `facade.upsert_model_config`.
pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::KernelSettingsApi;
    let facade = kernel_facade();
    let models = facade.list_model_configs().await?;
    let mut count = 0;
    for mut m in models {
        if let Ok(key) = keyring.get(&m.id) {
            if !key.is_empty() {
                m.api_key = Some(key);
                facade.upsert_model_config(m).await?;
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Validate user input from the provider form. Returns `Ok(())` when the
/// input is acceptable, or `Err(msg)` with a Chinese error message.
pub fn validate_provider_input(
    name: &str,
    type_str: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("名称不能为空".to_string());
    }
    if api_key.trim().is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    if model.trim().is_empty() {
        return Err("模型不能为空".to_string());
    }
    match type_str {
        "anthropic" | "openai" | "gemini" => {}
        "custom-openai" | "custom-anthropic" => {
            if base_url.trim().is_empty() {
                return Err("自定义服务需要提供 Base URL".to_string());
            }
        }
        _ => {
            return Err(format!("不支持的服务类型: {type_str}"));
        }
    }
    Ok(())
}
