use super::keyring::{resolve_api_key, KeyringBackend, PRODUCTION_KEYRING};
use super::{AppSettings, ProviderConfig, ProviderType};

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
pub fn provider_wire_format(t: &ProviderType) -> &'static str {
    match t {
        ProviderType::Anthropic => "anthropic",
        ProviderType::Openai => "openai",
        ProviderType::Gemini => "gemini",
        ProviderType::CustomOpenaiCompatible => "openai",
        ProviderType::CustomAnthropicCompatible => "anthropic",
    }
}

/// Convert a desktop `ProviderConfig` into a facade `AIModelConfigDto`.
/// The actual API key is resolved from the OS keyring when the stored
/// field is the sentinel value (`API_KEY_SENTINEL`).
///
/// ## Logging discipline
///
/// This function never logs the API key itself. The keyring lookup
/// failure is handled by the caller (`sync_providers_to_core`).
pub fn provider_to_ai_model_config(p: &ProviderConfig, keyring: &dyn KeyringBackend) -> northhing_kernel_api::settings::AIModelConfigDto {
    let resolved_key = resolve_api_key(keyring, &p.id, &p.api_key).unwrap_or_else(|_| {
        // Keyring unavailable — fall back to the stored field value.
        // This path is only reached when the sentinel is present but the
        // keyring entry is missing (should not happen in normal operation
        // after a successful migration). We log a warning without the key.
        tracing::warn!(
            target: "app_state",
            "keyring entry missing for provider '{}' ({}) — falling back to stored field",
            p.id, p.name
        );
        p.api_key.clone()
    });
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

/// Compute which core model ids are "stale" — present in the core config
/// but no longer referenced by any desktop provider. These are leftovers
/// from providers that were edited into a new identity or deleted entirely.
/// Returns the list of ids that should be removed from core to keep the
/// two stores in sync (mirror semantics).
pub(crate) fn compute_stale_core_model_ids(existing_ids: &[String], providers: &[ProviderConfig]) -> Vec<String> {
    let provider_ids: std::collections::HashSet<&str> = providers.iter().map(|p| p.id.as_str()).collect();
    existing_ids
        .iter()
        .filter(|id| !provider_ids.contains(id.as_str()))
        .cloned()
        .collect()
}

// 2026-07-18 (D2f): pick the provider id that should become core
// `default_models.primary` — the desktop default when it exists and is
// enabled; otherwise None (leave core reconcile semantics untouched).
pub(crate) fn desired_primary_model_id(s: &AppSettings) -> Option<String> {
    let dm = s.default_model.as_ref()?;
    s.providers
        .iter()
        .any(|p| p.id == dm.provider_id && p.enabled)
        .then(|| dm.provider_id.clone())
}

/// Sync all desktop providers into the core `GlobalConfig.ai.models` list,
/// then run `reconcile_models` so `default_models.primary` / `.fast` point
/// at the first enabled model. Returns the number of providers synced.
///
/// This is the "adapt-push" path: desktop owns the provider UI + storage,
/// but the runtime reads from core — so on every provider change we push
/// the corresponding `AIModelConfig` into core and let it reconcile.
///
/// 2026-07-18 (D2d): mirror semantics — after add/update, delete any core
/// model whose id no longer appears in `settings.providers` so the two
/// stores stay consistent (fixes the "10 models configured" stale-entry
/// leak caused by provider edits that changed their id).
pub async fn sync_providers_to_core(settings: &AppSettings) -> anyhow::Result<usize> {
    use northhing_core::kernel_facade::kernel_facade;
    use northhing_kernel_api::KernelSettingsApi;
    let facade = kernel_facade();
    let existing = facade.list_model_configs().await?;
    let mut count = 0;
    for p in &settings.providers {
        let model = provider_to_ai_model_config(p, &*PRODUCTION_KEYRING);
        facade.upsert_model_config(model).await?;
        count += 1;
    }
    // 2026-07-18 (D2d): delete stale core models that no longer match any
    // desktop provider, then reconcile default slots.
    let existing_ids: Vec<String> = existing.iter().map(|m| m.id.clone()).collect();
    let stale_ids = compute_stale_core_model_ids(&existing_ids, &settings.providers);
    for stale_id in &stale_ids {
        if let Err(e) = facade.delete_model_config(stale_id).await {
            tracing::warn!(target: "app_state", "delete stale core model '{stale_id}' failed: {e}");
        }
    }
    // 2026-07-18 (D2f): push desktop default_model to core if it points at an
    // enabled provider, so core.default_models.primary is never left null.
    if let Some(pid) = desired_primary_model_id(settings) {
        let pid_for_log = pid.clone();
        if let Err(e) = facade.set_default_provider(&pid).await {
            tracing::warn!(target: "app_state", "set default_models.primary to '{pid_for_log}' failed: {e}");
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
