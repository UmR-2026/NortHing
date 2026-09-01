// 此文件因 A 类「未初始化断言」单测独占进程而独立成文件
// 不要向本文件添加任何会触发 init_core() / init_*() / 全局单例初始化的测试
// 违反即回归

use northhing::app_state::settings::MockKeyring;
use northhing::ui_dioxus::api_settings::{
    get_global_config, list_mcp_servers, list_model_configs, persist_onboarding_provider_with_keyring,
    set_default_provider, set_mcp_enabled, store_provider_api_key_with_keyring, test_provider_config,
    upsert_model_config,
};
use northhing_kernel_api::settings::{AIModelConfigDto, MCPServerDto, ProviderFormDto};

#[tokio::test]
async fn test_api_functions_fail_cleanly_before_init() {
    // Facade is uninitialized in isolated test environment, should return Err not panic
    let _ = northhing::ui_dioxus::api::submit_turn("test-session", "hello".into()).await;
    let _ = northhing::ui_dioxus::api::stop_turn(&"test-turn".to_string()).await;
    let _ = northhing::ui_dioxus::api::list_sessions().await;
    let _ = northhing::ui_dioxus::api::list_sessions_all_workspaces().await;
    let _ = northhing::ui_dioxus::api::get_session(&"test-session".to_string()).await;
    let _ = northhing::ui_dioxus::api::get_messages(&"test-session".to_string()).await;
    let _ = northhing::ui_dioxus::api::respond_to_tool_confirmation("call-1", true).await;
    let _ = northhing::ui_dioxus::api::ensure_room_session().await;
    let _ = get_global_config().await;
    let _ = list_model_configs().await;
    let _ = set_default_provider("test-model").await;
    let _ = list_mcp_servers().await;
    let mcp = MCPServerDto {
        id: "test".into(),
        name: "test".into(),
        config: northhing_kernel_api::settings::MCPServerConfigDto {
            command: "node".into(),
            args: vec![],
            env: None,
        },
        location: northhing_kernel_api::settings::ConfigLocationDto::User,
        enabled: Some(true),
    };
    let _ = set_mcp_enabled(mcp, false).await;
    let form = ProviderFormDto {
        provider_id: "onboarding".into(),
        base_url: Some("http://localhost".into()),
        api_key: Some("key".into()),
        model: Some("default".into()),
        provider_type: None,
    };
    let kr = MockKeyring::new();
    let _ = test_provider_config(form).await;
    let _ = store_provider_api_key_with_keyring(&kr, "onboarding", "key").await;
    let dummy_model = AIModelConfigDto {
        id: "test".into(),
        provider_id: "openai".into(),
        model: "test".into(),
        display_name: None,
        max_tokens: None,
        temperature: None,
        base_url: None,
        enabled: Some(true),
        category: None,
        capabilities: None,
        auth: None,
        inline_think_in_text: None,
    };
    let _ = upsert_model_config(dummy_model, None).await;
    let _ = persist_onboarding_provider_with_keyring(&kr, "claude", "http://localhost", "key", "Agent").await;
}
