// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room settings API (W10-1 split).
// Wrappers over `northhing_core::kernel_facade()` plus keyring-integrated onboarding.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::agents::{KernelAgentsApi, SkillInfoDto, SkillScopeDto};
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::settings::{
    AIModelConfigDto, GlobalConfigDto, KernelSettingsApi, MCPServerDto, ProviderFormDto,
    ProviderTestResultDto,
};
use crate::app_state::settings::{
    infer_provider_wire_format, store_api_key, KeyringBackend, PRODUCTION_KEYRING,
};

/// Retrieves global configuration including providers and default provider id.
pub async fn get_global_config() -> Result<GlobalConfigDto, KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("get_global_config", async move {
        kernel_facade().get_global_config().await
    })
    .await
}

/// Lists all configured AI models.
pub async fn list_model_configs() -> Result<Vec<AIModelConfigDto>, KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("list_model_configs", async move {
        kernel_facade().list_model_configs().await
    })
    .await
}

/// Sets the default AI provider / model ID.
pub async fn set_default_provider(id: &str) -> Result<(), KernelError> {
    let id = id.to_string();
    crate::ui_dioxus::api::kernel_dispatch("set_default_provider", async move {
        kernel_facade().set_default_provider(&id).await
    })
    .await
}

/// Lists all configured MCP servers.
pub async fn list_mcp_servers() -> Result<Vec<MCPServerDto>, KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("list_mcp_servers", async move {
        kernel_facade().list_mcp_servers().await
    })
    .await
}

/// Sets the enabled state of an MCP server and updates its configuration.
pub async fn set_mcp_enabled(mut server: MCPServerDto, enabled: bool) -> Result<(), KernelError> {
    server.enabled = Some(enabled);
    crate::ui_dioxus::api::kernel_dispatch("set_mcp_enabled", async move {
        kernel_facade().upsert_mcp_server(server).await
    })
    .await
}

/// Lists all skills, overlaying user-scope overrides on the `enabled` flag.
pub async fn list_skills() -> Result<Vec<SkillInfoDto>, KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("list_skills", async move {
        let mut skills = kernel_facade().list_skills().await?;
        let overrides = kernel_facade().load_skill_overrides().await?;
        let map: std::collections::HashMap<String, bool> = overrides
            .overrides
            .iter()
            .filter_map(|o| o.value.as_bool().map(|v| (o.skill_id.clone(), v)))
            .collect();
        for s in skills.iter_mut() {
            s.enabled = map.get(&s.id).copied().unwrap_or(s.enabled);
        }
        Ok(skills)
    })
    .await
}

pub async fn set_skill_enabled(skill_id: &str, enabled: bool) -> Result<(), KernelError> {
    let skill_id = skill_id.to_string();
    crate::ui_dioxus::api::kernel_dispatch("set_skill_enabled", async move {
        #[rustfmt::skip]
        let scope = SkillScopeDto { scope_type: "user".into(), workspace_path: None, mode_id: None };
        kernel_facade().set_skill_enabled(&skill_id, scope, enabled).await
    })
    .await
}

/// Tests a provider configuration without modifying persistent global config.
pub async fn test_provider_config(form: ProviderFormDto) -> Result<ProviderTestResultDto, KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("test_provider_config", async move {
        kernel_facade().test_provider_config(form).await
    })
    .await
}

/// Stores an API key in the specified keyring for the onboarding flow.
pub async fn store_provider_api_key_with_keyring(
    keyring: &dyn KeyringBackend,
    provider_id: &str,
    plaintext: &str,
) -> anyhow::Result<String> {
    store_api_key(keyring, provider_id, plaintext)
}

/// Stores an API key in the OS keyring for the onboarding flow.
pub async fn store_provider_api_key(provider_id: &str, plaintext: &str) -> anyhow::Result<String> {
    store_provider_api_key_with_keyring(
        &*PRODUCTION_KEYRING,
        provider_id,
        plaintext,
    )
    .await
}

/// Adds or updates an AI model / provider configuration in the kernel facade.
pub async fn upsert_model_config(config: AIModelConfigDto, api_key: Option<String>) -> Result<(), KernelError> {
    crate::ui_dioxus::api::kernel_dispatch("upsert_model_config", async move {
        kernel_facade().upsert_model_config(config, api_key).await
    })
    .await
}

/// Persists the onboarding provider configuration into the specified keyring and kernel facade,
/// and sets it as the default provider in the global configuration.
///
/// Returns `Ok(provider_id)` on success, or `Err(user_facing_chinese_error)` on failure.
pub async fn persist_onboarding_provider_with_keyring(
    keyring: &dyn KeyringBackend,
    model: &str,
    base_url: &str,
    api_key: &str,
    agent_name: &str,
) -> Result<String, String> {
    let provider_id = uuid::Uuid::new_v4().to_string();
    let wire_format = infer_provider_wire_format(base_url, model);

    // 1. Store API key in keyring under the provider id
    if let Err(e) = store_provider_api_key_with_keyring(keyring, &provider_id, api_key).await {
        let first_line = e
            .to_string()
            .lines()
            .next()
            .unwrap_or("Key 存储失败")
            .trim()
            .to_string();
        return Err(format!("Key 存储失败: {first_line}"));
    }

    // 2. Build model DTO and persist into core facade
    let model_dto = AIModelConfigDto {
        id: provider_id.clone(),
        provider_id: wire_format.to_string(),
        model: model.trim().to_string(),
        display_name: Some(if !agent_name.trim().is_empty() {
            agent_name.trim().to_string()
        } else {
            model.trim().to_string()
        }),
        max_tokens: None,
        temperature: None,
        base_url: if base_url.trim().is_empty() {
            None
        } else {
            Some(base_url.trim().to_string())
        },
        enabled: Some(true),
        category: Some("general_chat".to_string()),
        capabilities: Some(vec!["text_chat".to_string()]),
        auth: Some("api_key".to_string()),
        inline_think_in_text: Some(false),
    };

    if let Err(e) = upsert_model_config(model_dto, Some(api_key.to_string())).await {
        let first_line = e
            .to_string()
            .lines()
            .next()
            .unwrap_or("Provider 保存失败")
            .trim()
            .to_string();
        return Err(format!("Provider 保存失败: {first_line}"));
    }

    // 3. Set as default provider in core config
    if let Err(e) = set_default_provider(&provider_id).await {
        let first_line = e
            .to_string()
            .lines()
            .next()
            .unwrap_or("设为默认 Provider 失败")
            .trim()
            .to_string();
        return Err(format!("设为默认 Provider 失败: {first_line}"));
    }

    Ok(provider_id)
}

/// Persists the onboarding provider configuration into the OS keyring and kernel facade,
/// and sets it as the default provider in the global configuration.
///
/// Returns `Ok(provider_id)` on success, or `Err(user_facing_chinese_error)` on failure.
pub async fn persist_onboarding_provider(
    model: &str,
    base_url: &str,
    api_key: &str,
    agent_name: &str,
) -> Result<String, String> {
    persist_onboarding_provider_with_keyring(
        &*PRODUCTION_KEYRING,
        model,
        base_url,
        api_key,
        agent_name,
    )
    .await
}

#[cfg(test)]
pub(crate) static TEST_GLOBAL_CONFIG_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::KernelBootstrapApi;

    #[tokio::test]
    async fn test_persist_onboarding_provider_success_flow() -> anyhow::Result<()> {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let _ = kernel_facade().init_core().await;
        let kr = crate::app_state::settings::MockKeyring::new();
        let res = persist_onboarding_provider_with_keyring(
            &kr,
            "claude-3-7-sonnet",
            "https://api.anthropic.com/v1",
            "sk-ant-test-key-9999",
            "TestAgent",
        )
        .await;
        assert!(res.is_ok(), "persist_onboarding_provider failed: {:?}", res.err());
        let provider_id = res.unwrap();

        kr.assert_contains(&provider_id, "sk-ant-test-key-9999");

        let global_cfg = get_global_config().await?;
        assert_eq!(global_cfg.default_provider_id.as_deref(), Some(provider_id.as_str()));
        let provider = global_cfg
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .expect("persisted provider must be in global config");
        assert_eq!(provider.model, "claude-3-7-sonnet");
        assert_eq!(provider.provider_type.as_deref(), Some("anthropic"));

        let models = list_model_configs().await?;
        let model = models
            .iter()
            .find(|m| m.id == provider_id)
            .expect("persisted model must be in model configs");
        assert_eq!(model.model, "claude-3-7-sonnet");
        assert_eq!(model.provider_id, "anthropic");

        let _ = northhing_core::kernel_facade::kernel_facade().delete_model_config(&provider_id).await;
        let _ = kr.delete(&provider_id);
        kr.assert_not_contains(&provider_id);
        Ok(())
    }
}
