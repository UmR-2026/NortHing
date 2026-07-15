#![cfg(feature = "mcp")]

//! Config And Server Lifecycle tests.

mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn mcp_config_service_orchestration_preserves_load_save_delete_contract() {
    let store = Arc::new(InMemoryMCPConfigStore::default());
    store.values.lock().await.insert(
        "mcp_servers".to_string(),
        serde_json::json!({
            "mcpServers": {
                "remote-docs": {
                    "type": "remote",
                    "url": "https://example.com/mcp",
                    "headers": {
                        "X-Existing": "kept"
                    }
                }
            }
        }),
    );

    let service = MCPConfigService::new(store.clone());

    let loaded = service.load_all_configs().await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "remote-docs");
    assert_eq!(loaded[0].location, ConfigLocation::User);

    let updated = service
        .set_remote_authorization("remote-docs", "plain-token")
        .await
        .unwrap();
    assert_eq!(
        updated.headers.get("Authorization").map(String::as_str),
        Some("Bearer plain-token")
    );

    let saved_value = store.values.lock().await.get("mcp_servers").cloned().unwrap();
    assert_eq!(
        saved_value["mcpServers"]["remote-docs"]["headers"]["Authorization"],
        "Bearer plain-token"
    );
    assert_eq!(
        saved_value["mcpServers"]["remote-docs"]["headers"]["X-Existing"],
        "kept"
    );

    let cleared = service.clear_remote_authorization("remote-docs").await.unwrap();
    assert!(!cleared.headers.contains_key("Authorization"));

    service.delete_server_config("remote-docs").await.unwrap();
    let deleted_value = store.values.lock().await.get("mcp_servers").cloned().unwrap();
    assert!(deleted_value["mcpServers"]
        .as_object()
        .unwrap()
        .get("remote-docs")
        .is_none());
}


#[tokio::test]
async fn mcp_config_service_keeps_load_failures_as_empty_baseline() {
    let service = MCPConfigService::new(Arc::new(FailingMCPConfigStore));

    let configs = service
        .load_all_configs()
        .await
        .expect("load failures are treated as empty config sources");
    assert!(configs.is_empty());

    let missing = service
        .get_server_config("missing")
        .await
        .expect("get_server_config also sees empty config sources");
    assert!(missing.is_none());

    let save_error = service
        .save_server_config(&make_mcp_config(
            "remote-docs",
            ConfigLocation::User,
            MCPServerType::Remote,
            None,
            Some("https://example.com/mcp"),
        ))
        .await
        .expect_err("writes must still surface config backend failures");
    assert_eq!(save_error.kind(), MCPRuntimeErrorKind::Configuration);
}


#[tokio::test]
async fn mcp_server_process_owner_preserves_unsupported_remote_transport_contract() {
    let mut config = make_mcp_config(
        "remote-sse",
        ConfigLocation::User,
        MCPServerType::Remote,
        None,
        Some("https://example.com/mcp"),
    );
    config.transport = Some(MCPServerTransport::Sse);

    let mut process = MCPServerProcess::new(
        "remote-sse".to_string(),
        "Remote SSE".to_string(),
        MCPServerType::Remote,
    );
    assert_eq!(process.status().await, MCPServerStatus::Uninitialized);
    assert_eq!(process.server_type(), MCPServerType::Remote);

    let error = process.start_remote(std::env::temp_dir(), &config).await.unwrap_err();
    assert_eq!(error.kind(), MCPRuntimeErrorKind::NotImplemented);
    assert!(error
        .to_string()
        .contains("Remote MCP transport 'sse' is not yet supported"));
    assert_eq!(process.status().await, MCPServerStatus::Uninitialized);

    let pool = MCPConnectionPool::new();
    assert!(pool.get_all_server_ids().await.is_empty());
}


#[test]
fn mcp_config_location_preserves_kebab_case_wire_contract() {
    assert_eq!(
        serde_json::to_value(ConfigLocation::BuiltIn).unwrap(),
        serde_json::json!("built-in")
    );
    assert_eq!(
        serde_json::from_value::<ConfigLocation>(serde_json::json!("user")).unwrap(),
        ConfigLocation::User
    );
    assert_eq!(
        serde_json::from_value::<ConfigLocation>(serde_json::json!("project")).unwrap(),
        ConfigLocation::Project
    );
}


#[test]
fn mcp_json_config_helpers_preserve_load_format_and_save_validation_contract() {
    let legacy_array = serde_json::json!([
        {
            "id": "local",
            "name": "Local",
            "type": "local",
            "command": "npx"
        }
    ]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&format_mcp_json_config_value(Some(&legacy_array)).unwrap()).unwrap(),
        serde_json::json!({
            "mcpServers": {
                "local": {
                    "id": "local",
                    "name": "Local",
                    "type": "local",
                    "command": "npx"
                }
            }
        })
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&format_mcp_json_config_value(None).unwrap()).unwrap(),
        serde_json::json!({ "mcpServers": {} })
    );

    validate_mcp_json_config(&serde_json::json!({
        "mcpServers": {
            "remote": {
                "type": "sse",
                "url": "https://example.com/sse",
                "headers": {
                    "Authorization": "Bearer token"
                }
            }
        }
    }))
    .expect("valid remote SSE config");

    assert_eq!(
        validate_mcp_json_config(&serde_json::json!({}))
            .unwrap_err()
            .to_string(),
        "Config missing 'mcpServers' field"
    );
    assert_eq!(
        validate_mcp_json_config(&serde_json::json!({
            "mcpServers": {
                "bad": {
                    "type": "container",
                    "command": "docker"
                }
            }
        }))
        .unwrap_err()
        .to_string(),
        "Server 'bad' has unsupported 'type' value: 'container'"
    );
    assert_eq!(
        validate_mcp_json_config(&serde_json::json!({
            "mcpServers": {
                "bad": {
                    "source": "remote",
                    "command": "npx"
                }
            }
        }))
        .unwrap_err()
        .to_string(),
        "Server 'bad' source='remote' conflicts with command-based configuration"
    );
}


#[test]
fn mcp_config_merge_helpers_preserve_precedence_and_dedup_contract() {
    let merged = merge_mcp_server_config_sources([
        vec![make_mcp_config(
            "github-user",
            ConfigLocation::User,
            MCPServerType::Remote,
            None,
            Some("https://example.com/mcp"),
        )],
        vec![
            make_mcp_config(
                "github-user",
                ConfigLocation::Project,
                MCPServerType::Remote,
                None,
                Some("https://project.example.com/mcp"),
            ),
            make_mcp_config(
                "github-project",
                ConfigLocation::Project,
                MCPServerType::Remote,
                None,
                Some("https://example.com/mcp"),
            ),
        ],
    ]);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].id, "github-user");
    assert_eq!(merged[0].location, ConfigLocation::Project);
    assert_eq!(merged[0].url.as_deref(), Some("https://project.example.com/mcp"));
    assert_eq!(merged[1].id, "github-project");
    assert_eq!(merged[1].location, ConfigLocation::Project);

    let deduped = merge_mcp_server_config_sources([
        vec![make_mcp_config(
            "github-user",
            ConfigLocation::User,
            MCPServerType::Remote,
            None,
            Some("https://example.com/mcp"),
        )],
        vec![make_mcp_config(
            "github-project",
            ConfigLocation::Project,
            MCPServerType::Remote,
            None,
            Some("https://example.com/mcp"),
        )],
    ]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].id, "github-project");
    assert_eq!(deduped[0].location, ConfigLocation::Project);
}


#[test]
fn mcp_config_authorization_helpers_preserve_header_precedence_and_normalization() {
    let mut config = make_mcp_config(
        "remote-auth",
        ConfigLocation::User,
        MCPServerType::Remote,
        None,
        Some("https://example.com/mcp"),
    );
    config
        .env
        .insert("Authorization".to_string(), "legacy-token".to_string());
    config
        .headers
        .insert("Authorization".to_string(), "Bearer header-token".to_string());

    assert_eq!(
        get_mcp_remote_authorization_value(&config).as_deref(),
        Some("Bearer header-token")
    );
    assert_eq!(get_mcp_remote_authorization_source(&config), Some("headers"));
    assert!(has_mcp_remote_authorization(&config));
    assert!(!has_mcp_remote_oauth(&config));
    assert!(!has_mcp_remote_xaa(&config));
    assert_eq!(
        normalize_mcp_authorization_value("plain-token").as_deref(),
        Some("Bearer plain-token")
    );
    assert_eq!(
        normalize_mcp_authorization_value("Bearer existing").as_deref(),
        Some("Bearer existing")
    );
    assert_eq!(normalize_mcp_authorization_value("   "), None);

    remove_mcp_authorization_keys(&mut config.headers);
    remove_mcp_authorization_keys(&mut config.env);
    assert_eq!(get_mcp_remote_authorization_value(&config), None);
    assert_eq!(get_mcp_remote_authorization_source(&config), None);
}


#[test]
fn mcp_server_type_and_status_preserve_lowercase_wire_contract() {
    assert_eq!(
        serde_json::to_value(MCPServerType::Local).unwrap(),
        serde_json::json!("local")
    );
    assert_eq!(
        serde_json::from_value::<MCPServerType>(serde_json::json!("remote")).unwrap(),
        MCPServerType::Remote
    );
    assert_eq!(
        serde_json::to_value(MCPServerStatus::NeedsAuth).unwrap(),
        serde_json::json!("needsauth")
    );
    assert_eq!(
        serde_json::from_value::<MCPServerStatus>(serde_json::json!("reconnecting")).unwrap(),
        MCPServerStatus::Reconnecting
    );
}


