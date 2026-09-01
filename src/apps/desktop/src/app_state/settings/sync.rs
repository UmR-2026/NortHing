use super::keyring::KeyringBackend;

/// Edit-flow key resolution (P1-2 fail-closed): `stored` is the raw keyring
/// read result. Blank incoming key inherits the stored one; a keyring error
/// propagates so the caller refuses the save instead of swallowing it.
pub fn resolve_edit_api_key(stored: anyhow::Result<String>, incoming: &str) -> anyhow::Result<String> {
    if incoming.trim().is_empty() {
        stored
    } else {
        Ok(incoming.to_string())
    }
}

// ===== Core sync helpers =====

/// Infer provider wire format ("anthropic", "gemini", or "openai") from base URL and model name.
pub fn infer_provider_wire_format(base_url: &str, model: &str) -> &'static str {
    let url_lower = base_url.to_ascii_lowercase();
    let model_lower = model.to_ascii_lowercase();
    if url_lower.contains("anthropic") || model_lower.starts_with("claude") {
        "anthropic"
    } else if url_lower.contains("google") || url_lower.contains("gemini") || model_lower.starts_with("gemini") {
        "gemini"
    } else {
        "openai"
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

/// Push resolved keyring keys to core in-memory models on startup or change (Scheme C).
/// Reads model configs from core facade (keyless contract shape), resolves
/// each model's key from the OS keyring, and pushes it into core memory via the
/// explicit `api_key` parameter on `upsert_model_config`.
pub async fn push_resolved_keys_to_core(keyring: &dyn KeyringBackend) -> anyhow::Result<usize> {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::KernelSettingsApi;
    let facade = kernel_facade();
    let models = facade.list_model_configs().await?;
    let mut count = 0;
    for m in models {
        if let Ok(key) = keyring.get(&m.id) {
            if !key.is_empty() {
                facade.upsert_model_config(m, Some(key)).await?;
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Prefix of the in-memory test models pushed by [`push_resolved_keys_to_core`] in unit tests.
#[cfg(test)]
pub const TEST_PUSH_MODEL_PREFIX: &str = "test-push-model-";

/// 测试专用 seam，release 构建不存在。清空 resolved-keys 内存态：把
/// [`push_resolved_keys_to_core`] 写入 core 内存的带前缀测试模型（及其内存态 key）
/// 移除；无残留时只读不写盘。
#[cfg(test)]
pub async fn _reset_resolved_keys_for_test() -> anyhow::Result<()> {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::KernelSettingsApi;
    let facade = kernel_facade();
    for m in facade.list_model_configs().await? {
        if m.id.starts_with(TEST_PUSH_MODEL_PREFIX) {
            facade.delete_model_config(&m.id).await?;
        }
    }
    Ok(())
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
