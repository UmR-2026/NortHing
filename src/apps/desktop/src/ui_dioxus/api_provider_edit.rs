// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room provider edit & delete API layer (W7-1).
// Keyring-integrated wrappers over `northhing_core::kernel_facade()`.

use northhing_core::kernel_facade::kernel_facade;
use northhing_kernel_api::settings::{AIModelConfigDto, KernelSettingsApi};

use crate::app_state::settings::{
    delete_api_key, provider_wire_format_from_str, resolve_edit_api_key, store_api_key, validate_provider_input,
    KeyringBackend, PRODUCTION_KEYRING,
};

/// Edits an existing provider configuration with explicit keyring integration.
///
/// 1. Loads the existing model config from kernel facade (fails if not found).
/// 2. Resolves effective API key via `resolve_edit_api_key` (blank incoming inherits stored; fail-closed).
/// 3. Validates inputs via `validate_provider_input`.
/// 4. If a non-blank key was provided, overwrites the keyring entry.
/// 5. Maps `provider_type` to wire format via `provider_wire_format_from_str`.
/// 6. Updates model config in kernel facade via `upsert_model_config`.
pub async fn edit_provider_with_keyring(
    keyring: &dyn KeyringBackend,
    id: &str,
    name: &str,
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    enabled: bool,
) -> Result<(), String> {
    // 1. Load existing config from kernel facade
    let facade = kernel_facade();
    let existing_models = facade
        .list_model_configs()
        .await
        .map_err(|e| format!("获取模型配置失败: {e}"))?;
    let existing = existing_models
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("未找到指定服务配置: {id}"))?;

    // 2. Key resolution (fail-closed: keyring read error refuses save)
    let effective_key = resolve_edit_api_key(keyring.get(id), api_key)
        .map_err(|e| format!("读取密钥库失败: {e}"))?;

    // 3. Validate user input
    validate_provider_input(name, provider_type, base_url, &effective_key, model)?;

    // 4. Overwrite keyring if incoming key is non-empty
    if !api_key.trim().is_empty() {
        store_api_key(keyring, id, &effective_key).map_err(|e| format!("密钥存储失败: {e}"))?;
    }

    // 5. Wire format mapping
    let wire_format = provider_wire_format_from_str(provider_type);

    // 6. Build updated AIModelConfigDto and persist to kernel
    let updated_dto = AIModelConfigDto {
        id: id.to_string(),
        provider_id: wire_format.to_string(),
        model: model.trim().to_string(),
        display_name: Some(if !name.trim().is_empty() {
            name.trim().to_string()
        } else {
            model.trim().to_string()
        }),
        max_tokens: existing.max_tokens,
        temperature: existing.temperature,
        base_url: if base_url.trim().is_empty() {
            None
        } else {
            Some(base_url.trim().to_string())
        },
        enabled: Some(enabled),
        category: existing.category.or_else(|| Some("general_chat".to_string())),
        capabilities: existing
            .capabilities
            .or_else(|| Some(vec!["text_chat".to_string(), "function_calling".to_string()])),
        auth: existing.auth.or_else(|| Some("api_key".to_string())),
        inline_think_in_text: existing.inline_think_in_text.or(Some(true)),
    };

    facade
        .upsert_model_config(updated_dto, Some(effective_key))
        .await
        .map_err(|e| format!("保存配置失败: {e}"))?;

    Ok(())
}

/// Edits an existing provider configuration using the production OS keyring.
pub async fn edit_provider(
    id: &str,
    name: &str,
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    enabled: bool,
) -> Result<(), String> {
    edit_provider_with_keyring(
        &*PRODUCTION_KEYRING,
        id,
        name,
        provider_type,
        base_url,
        api_key,
        model,
        enabled,
    )
    .await
}

/// Deletes a provider configuration with explicit keyring cleanup.
///
/// 1. Refuses deletion if `id` is the current default provider.
/// 2. Deletes model configuration from kernel facade.
/// 3. Deletes keyring entry (best-effort).
pub async fn delete_provider_with_keyring(keyring: &dyn KeyringBackend, id: &str) -> Result<(), String> {
    let facade = kernel_facade();

    // 1. Refuse deleting default provider
    let global_cfg = facade
        .get_global_config()
        .await
        .map_err(|e| format!("获取全局配置失败: {e}"))?;
    if global_cfg.default_provider_id.as_deref() == Some(id) {
        return Err("不能删除默认 AI 服务，请先切换默认服务后再删除".to_string());
    }

    // 2. Delete model config via facade
    facade
        .delete_model_config(id)
        .await
        .map_err(|e| format!("删除模型配置失败: {e}"))?;

    // 3. Best-effort keyring deletion
    // ponytail: no session-reference scan on delete; add when session metadata query lands
    if let Err(e) = delete_api_key(keyring, id) {
        tracing::warn!(target: "desktop::api", "delete_api_key failed for {id}: {e}");
    }

    Ok(())
}

/// Deletes a provider configuration using the production OS keyring.
pub async fn delete_provider(id: &str) -> Result<(), String> {
    delete_provider_with_keyring(&*PRODUCTION_KEYRING, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_kernel_api::KernelBootstrapApi;
    use crate::app_state::settings::MockKeyring;
    use crate::ui_dioxus::api::TEST_GLOBAL_CONFIG_MUTEX;

    #[derive(Debug)]
    struct FailingKeyring;

    impl KeyringBackend for FailingKeyring {
        fn store(&self, _account: &str, _secret: &str) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("keyring store failure"))
        }

        fn get(&self, _account: &str) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("keyring read failure"))
        }

        fn delete(&self, _account: &str) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("keyring delete failure"))
        }
    }

    async fn setup_test_provider(id: &str, name: &str, provider_type: &str, model: &str) {
        let facade = kernel_facade();
        if let Err(err) = facade.init_core().await {
            tracing::warn!(error = %err, "test setup: init_core failed; continuing");
        }
        let dto = AIModelConfigDto {
            id: id.to_string(),
            provider_id: provider_type.to_string(),
            model: model.to_string(),
            display_name: Some(name.to_string()),
            max_tokens: None,
            temperature: None,
            base_url: Some("https://api.openai.com/v1".to_string()),
            enabled: Some(true),
            category: Some("general_chat".to_string()),
            capabilities: Some(vec!["text_chat".to_string()]),
            auth: Some("api_key".to_string()),
            inline_think_in_text: Some(true),
        };
        let _upsert = facade.upsert_model_config(dto, Some("initial-key".to_string())).await;
    }

    // ① 编辑留空 key=继承
    #[tokio::test]
    async fn test_edit_provider_blank_key_inherits_existing() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-inherit-key";
        setup_test_provider(id, "Old Name", "openai", "gpt-4").await;
        let kr = MockKeyring::new();
        kr.seed(id, "sk-stored-key-123");

        let res = edit_provider_with_keyring(
            &kr,
            id,
            "New Name",
            "openai",
            "https://api.openai.com/v1",
            "   ", // blank key -> inherit
            "gpt-4o",
            true,
        )
        .await;

        assert!(res.is_ok(), "edit failed: {:?}", res.err());
        kr.assert_contains(id, "sk-stored-key-123");

        let facade = kernel_facade();
        let models = facade.list_model_configs().await.unwrap();
        let updated = models.iter().find(|m| m.id == id).expect("model must exist");
        assert_eq!(updated.display_name.as_deref(), Some("New Name"));
        assert_eq!(updated.model, "gpt-4o");

        let _del = facade.delete_model_config(id).await;
    }

    // ② 编辑新 key=覆盖
    #[tokio::test]
    async fn test_edit_provider_new_key_overwrites_keyring() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-overwrite-key";
        setup_test_provider(id, "Old Name", "anthropic", "claude-3-haiku").await;
        let kr = MockKeyring::new();
        kr.seed(id, "sk-old-key");

        let res = edit_provider_with_keyring(
            &kr,
            id,
            "Updated Claude",
            "anthropic",
            "https://api.anthropic.com/v1",
            "sk-new-key-456",
            "claude-3-5-sonnet",
            true,
        )
        .await;

        assert!(res.is_ok(), "edit failed: {:?}", res.err());
        kr.assert_contains(id, "sk-new-key-456");

        let facade = kernel_facade();
        let models = facade.list_model_configs().await.unwrap();
        let updated = models.iter().find(|m| m.id == id).expect("model must exist");
        assert_eq!(updated.display_name.as_deref(), Some("Updated Claude"));
        assert_eq!(updated.model, "claude-3-5-sonnet");
        assert_eq!(updated.provider_id, "anthropic");

        let _del = facade.delete_model_config(id).await;
    }

    // ③ keyring 读失败=fail-closed 拒存
    #[tokio::test]
    async fn test_edit_provider_keyring_read_error_fails_closed() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-fail-closed";
        setup_test_provider(id, "Fail Closed Test", "openai", "gpt-4").await;
        let kr = FailingKeyring;

        let res = edit_provider_with_keyring(
            &kr,
            id,
            "New Name",
            "openai",
            "https://api.openai.com/v1",
            "", // blank -> attempts to read keyring
            "gpt-4o",
            true,
        )
        .await;

        assert!(res.is_err(), "should fail closed on keyring error");
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("读取密钥库失败"), "unexpected error message: {err_msg}");

        let _del = kernel_facade().delete_model_config(id).await;
    }

    // ④ 编辑不存在 id=Err
    #[tokio::test]
    async fn test_edit_provider_nonexistent_id_returns_error() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        if let Err(err) = kernel_facade().init_core().await {
            tracing::warn!(error = %err, "test setup: init_core failed; continuing");
        }
        let id = "test-w7-1-nonexistent-id-99999";
        let kr = MockKeyring::new();

        let res = edit_provider_with_keyring(
            &kr,
            id,
            "Ghost Provider",
            "openai",
            "https://api.openai.com/v1",
            "sk-some-key",
            "gpt-4o",
            true,
        )
        .await;

        assert!(res.is_err(), "editing nonexistent provider must fail");
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("未找到指定服务配置"), "unexpected error message: {err_msg}");
        kr.assert_not_contains(id);
    }

    // ⑤ 删除默认 provider=拒绝
    #[tokio::test]
    async fn test_delete_provider_default_provider_rejected() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-default-prov";
        setup_test_provider(id, "Default AI", "openai", "gpt-4").await;
        let facade = kernel_facade();
        let _set_def = facade.set_default_provider(id).await;

        let kr = MockKeyring::new();
        kr.seed(id, "sk-default-key");

        let res = delete_provider_with_keyring(&kr, id).await;
        assert!(res.is_err(), "deleting default provider must be rejected");
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("不能删除默认"), "unexpected error message: {err_msg}");

        // Verify provider still exists in core and keyring
        let models = facade.list_model_configs().await.unwrap();
        assert!(models.iter().any(|m| m.id == id));
        kr.assert_contains(id, "sk-default-key");

        // Cleanup: unset default and delete
        let _reset_def = facade.set_default_provider("").await;
        let _del = facade.delete_model_config(id).await;
    }

    // ⑥ 删除成功=config+keyring 双清
    #[tokio::test]
    async fn test_delete_provider_success_cleans_config_and_keyring() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-delete-success";
        setup_test_provider(id, "To Delete", "openai", "gpt-4").await;
        let facade = kernel_facade();

        let kr = MockKeyring::new();
        kr.seed(id, "sk-delete-key");

        let res = delete_provider_with_keyring(&kr, id).await;
        assert!(res.is_ok(), "delete failed: {:?}", res.err());

        // Verify deleted from facade
        let models = facade.list_model_configs().await.unwrap();
        assert!(!models.iter().any(|m| m.id == id), "provider must be removed from model configs");

        // Verify deleted from keyring
        kr.assert_not_contains(id);
    }

    // ⑦ 校验失败=零写入
    #[tokio::test]
    async fn test_edit_provider_validation_failure_zero_writes() {
        let _guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;
        let id = "test-w7-1-val-fail";
        setup_test_provider(id, "Original Name", "openai", "gpt-4").await;
        let kr = MockKeyring::new();
        kr.seed(id, "sk-original-key");

        // Pass empty name -> validation error
        let res = edit_provider_with_keyring(
            &kr,
            id,
            "   ", // empty name -> fails validation
            "openai",
            "https://api.openai.com/v1",
            "sk-attempted-new-key",
            "gpt-4o",
            true,
        )
        .await;

        assert!(res.is_err(), "validation should fail for empty name");
        let err_msg = res.unwrap_err();
        assert_eq!(err_msg, "名称不能为空");

        // Verify keyring was NOT overwritten
        kr.assert_contains(id, "sk-original-key");
        kr.assert_not_contains("sk-attempted-new-key");

        // Verify facade model was NOT modified
        let facade = kernel_facade();
        let models = facade.list_model_configs().await.unwrap();
        let model = models.iter().find(|m| m.id == id).expect("model must still exist");
        assert_eq!(model.display_name.as_deref(), Some("Original Name"));
        assert_eq!(model.model, "gpt-4");

        let _del = facade.delete_model_config(id).await;
    }
}
