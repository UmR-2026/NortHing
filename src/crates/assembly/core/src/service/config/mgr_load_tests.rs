use super::*;
use crate::infrastructure::PathManager;
use crate::service::config::manager::ConfigManagerSettings;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn save_config_atomically_persists_content_and_leaves_no_temp_files() {
    let temp_root = std::env::temp_dir().join(format!("northhing-save-config-test-{}", Uuid::new_v4()));
    let path_manager = Arc::new(PathManager::with_user_root_for_tests(temp_root.join("user-root")));
    let settings = ConfigManagerSettings {
        path_manager: Some(path_manager),
        auto_save: false,
        backup_count: 5,
    };

    let mut manager = ConfigManager::new(settings)
        .await
        .expect("config manager should initialize with isolated temp root");

    manager.config.app.language = "zh-CN".to_string();
    manager.save_config().await.expect("save_config should succeed");

    let saved_content = tokio::fs::read_to_string(&manager.config_file)
        .await
        .expect("saved config file should be readable");
    let parsed_config: GlobalConfig =
        serde_json::from_str(&saved_content).expect("saved config should parse into GlobalConfig");
    assert_eq!(parsed_config.app.language, "zh-CN");
    assert_eq!(parsed_config.version, manager.config.version);

    let config_dir = manager
        .config_file
        .parent()
        .expect("config file must have a parent directory");
    let mut dir_entries = tokio::fs::read_dir(config_dir)
        .await
        .expect("config directory should be readable");

    let mut found_tmp_files = Vec::new();
    while let Some(entry) = dir_entries
        .next_entry()
        .await
        .expect("reading directory entries should succeed")
    {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".tmp") || file_name.starts_with(".app.json.") {
            found_tmp_files.push(file_name);
        }
    }

    assert!(
        found_tmp_files.is_empty(),
        "expected no leftover temp files, but found: {:?}",
        found_tmp_files
    );

    let _ = tokio::fs::remove_dir_all(&temp_root).await;
}

#[tokio::test]
async fn legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean() {
    let temp_root = std::env::temp_dir().join(format!("northhing-scrub-test-{}", Uuid::new_v4()));
    let path_manager = Arc::new(PathManager::with_user_root_for_tests(temp_root.join("user-root")));
    path_manager.initialize_user_directories().await.unwrap();

    let config_file = path_manager.app_config_file();
    let legacy_json = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "ai": {
            "models": [
                {
                    "id": "test-model-1",
                    "name": "Test Model",
                    "provider": "anthropic",
                    "model_name": "claude-sonnet-4-5",
                    "base_url": "https://api.anthropic.com",
                    "api_key": "sk-ant-plaintext-secret-12345",
                    "enabled": true,
                    "category": "general_chat",
                    "capabilities": ["text_chat"]
                }
            ],
            "default_models": {
                "primary": "test-model-1"
            },
            "agent_models": {},
            "func_agent_models": {}
        }
    });

    tokio::fs::create_dir_all(config_file.parent().unwrap()).await.unwrap();
    tokio::fs::write(&config_file, serde_json::to_string_pretty(&legacy_json).unwrap())
        .await
        .unwrap();

    let settings = ConfigManagerSettings {
        path_manager: Some(path_manager),
        auto_save: true,
        backup_count: 5,
    };

    let manager = ConfigManager::new(settings).await.unwrap();

    // 1. In-memory: api_key must be cleared
    assert_eq!(manager.config.ai.models.len(), 1);
    assert_eq!(manager.config.ai.models[0].api_key, "");

    // 2. On-disk: file must be re-saved and contain NO plaintext key
    let on_disk_raw = tokio::fs::read_to_string(&config_file).await.unwrap();
    assert!(
        !on_disk_raw.contains("sk-ant-plaintext-secret-12345"),
        "disk file must NOT contain plaintext key"
    );
    assert!(
        !on_disk_raw.contains("\"api_key\":"),
        "disk file must NOT serialize api_key field key"
    );

    let _ = tokio::fs::remove_dir_all(&temp_root).await;
}

#[tokio::test]
async fn scheme_c_in_memory_keys_never_persist_to_disk() {
    let temp_root = std::env::temp_dir().join(format!("northhing-scheme-c-test-{}", Uuid::new_v4()));
    let path_manager = Arc::new(PathManager::with_user_root_for_tests(temp_root.join("user-root")));
    path_manager.initialize_user_directories().await.unwrap();

    let config_file = path_manager.app_config_file();
    let settings = ConfigManagerSettings {
        path_manager: Some(path_manager),
        auto_save: true,
        backup_count: 5,
    };

    let mut manager = ConfigManager::new(settings).await.unwrap();

    // Push a live model with plaintext API key into core memory
    let live_model = AIModelConfig {
        id: "live-model-openai".to_string(),
        name: "OpenAI GPT-4o".to_string(),
        provider: "openai".to_string(),
        model_name: "gpt-4o".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        request_url: None,
        api_key: "sk-live-secret-never-touch-disk-12345".to_string(),
        context_window: None,
        max_tokens: None,
        temperature: None,
        top_p: None,
        enabled: true,
        category: Default::default(),
        capabilities: vec![],
        recommended_for: vec![],
        metadata: None,
        enable_thinking_process: false,
        reasoning_mode: None,
        inline_think_in_text: false,
        custom_headers: None,
        custom_headers_mode: None,
        skip_ssl_verify: false,
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: None,
        custom_request_body_mode: None,
        auth: Default::default(),
    };

    manager.config.ai.models.push(live_model);
    manager.save_config().await.unwrap();

    // 1. In-memory check: key exists in memory
    assert_eq!(manager.config.ai.models.len(), 1);
    assert_eq!(
        manager.config.ai.models[0].api_key,
        "sk-live-secret-never-touch-disk-12345"
    );

    // 2. On-disk check: read raw file bytes from disk
    let on_disk_raw = tokio::fs::read_to_string(&config_file).await.unwrap();
    assert!(
        !on_disk_raw.contains("sk-live-secret-never-touch-disk-12345"),
        "disk file must NOT contain plaintext key"
    );
    assert!(
        !on_disk_raw.contains("\"api_key\":"),
        "disk file must NOT serialize api_key field"
    );

    let _ = tokio::fs::remove_dir_all(&temp_root).await;
}
