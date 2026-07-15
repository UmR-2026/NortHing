#![cfg(feature = "mcp")]

//! Dynamic Tools And Runtime tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn mcp_runtime_notification_and_backoff_helpers_preserve_manager_contract() {
    assert_eq!(
        detect_mcp_list_changed_kind("notifications/tools/list_changed"),
        Some(MCPListChangedKind::Tools)
    );
    assert_eq!(
        detect_mcp_list_changed_kind("notifications/prompts/listChanged"),
        Some(MCPListChangedKind::Prompts)
    );
    assert_eq!(
        detect_mcp_list_changed_kind("resources/list_changed"),
        Some(MCPListChangedKind::Resources)
    );
    assert_eq!(detect_mcp_list_changed_kind("notifications/other"), None);

    assert_eq!(
        compute_mcp_backoff_delay(Duration::from_secs(2), Duration::from_secs(60), 1),
        Duration::from_secs(2)
    );
    assert_eq!(
        compute_mcp_backoff_delay(Duration::from_secs(2), Duration::from_secs(60), 5),
        Duration::from_secs(32)
    );
    assert_eq!(
        compute_mcp_backoff_delay(Duration::from_secs(2), Duration::from_secs(60), 10),
        Duration::from_secs(60)
    );
}

#[test]
fn mcp_dynamic_tool_descriptor_and_result_rendering_preserve_tool_contract() {
    let tool = MCPTool {
        name: "search".to_string(),
        title: Some("Search".to_string()),
        description: Some("Find docs".to_string()),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: None,
        icons: None,
        annotations: Some(MCPToolAnnotations {
            title: Some("Search Docs".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(true),
        }),
        meta: None,
    };

    let descriptor = build_mcp_tool_descriptor("github", "GitHub", &tool);
    assert_eq!(
        descriptor,
        McpDynamicToolDescriptor {
            full_name: "mcp__github__search".to_string(),
            title: "Search Docs".to_string(),
            user_facing_name: "Search Docs (GitHub)".to_string(),
            description: "Tool 'Search Docs' from MCP server 'GitHub': Find docs [Hints: read-only, open-world]"
                .to_string(),
            provider_id: "github".to_string(),
            provider_kind: "mcp".to_string(),
            tool_info: McpToolInfo {
                server_id: "github".to_string(),
                server_name: "GitHub".to_string(),
                tool_name: "search".to_string(),
            },
            read_only: true,
        }
    );

    let rendered = render_mcp_tool_result_for_assistant(
        "search",
        &MCPToolResult {
            content: Some(vec![
                MCPToolResultContent::Text {
                    text: "done".to_string(),
                },
                MCPToolResultContent::Image {
                    data: "base64".to_string(),
                    mime_type: "image/png".to_string(),
                },
                MCPToolResultContent::ResourceLink {
                    uri: "file:///tmp/output.json".to_string(),
                    name: Some("output".to_string()),
                    description: None,
                    mime_type: Some("application/json".to_string()),
                },
            ]),
            is_error: false,
            structured_content: Some(serde_json::json!({ "ignored": "content wins" })),
            meta: None,
        },
        12_000,
    );
    assert_eq!(
        rendered,
        "done\n[Image: image/png]\n[Resource: output (file:///tmp/output.json)]"
    );

    assert_eq!(
        render_mcp_tool_result_for_assistant(
            "search",
            &MCPToolResult {
                content: None,
                is_error: true,
                structured_content: None,
                meta: None,
            },
            12_000,
        ),
        "Error executing MCP tool 'search'"
    );
}

#[tokio::test]
async fn mcp_dynamic_tool_provider_preserves_manifest_contract() {
    let provider = MCPDynamicToolProvider::new("github", "GitHub");
    let definitions = provider
        .load_tool_definitions(&FakeMCPToolCatalogClient {
            tools: vec![MCPTool {
                name: "search".to_string(),
                title: Some("Search".to_string()),
                description: Some("Search repositories".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    }
                }),
                output_schema: None,
                icons: None,
                annotations: Some(MCPToolAnnotations {
                    title: Some("Search".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                }),
                meta: None,
            }],
        })
        .await
        .unwrap();

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].mcp_tool.name, "search");
    assert_eq!(definitions[0].descriptor.full_name, "mcp__github__search");
    assert_eq!(definitions[0].descriptor.provider_id, "github");
    assert_eq!(definitions[0].descriptor.tool_info.server_name, "GitHub");
    assert!(definitions[0].descriptor.read_only);
}

#[tokio::test]
async fn mcp_dynamic_tool_provider_preserves_manifest_order_and_metadata_snapshot() {
    let provider = MCPDynamicToolProvider::new("docs-prod", "Docs Production");
    let definitions = provider
        .load_tool_definitions(&FakeMCPToolCatalogClient {
            tools: vec![
                MCPTool {
                    name: "lookup".to_string(),
                    title: None,
                    description: Some("Lookup docs".to_string()),
                    input_schema: serde_json::json!({ "type": "object" }),
                    output_schema: None,
                    icons: None,
                    annotations: Some(MCPToolAnnotations {
                        title: Some("Lookup".to_string()),
                        read_only_hint: Some(true),
                        destructive_hint: None,
                        idempotent_hint: Some(true),
                        open_world_hint: Some(false),
                    }),
                    meta: None,
                },
                MCPTool {
                    name: "write-note".to_string(),
                    title: Some("Write Note".to_string()),
                    description: None,
                    input_schema: serde_json::json!({ "type": "object" }),
                    output_schema: None,
                    icons: None,
                    annotations: Some(MCPToolAnnotations {
                        title: None,
                        read_only_hint: Some(false),
                        destructive_hint: Some(true),
                        idempotent_hint: Some(false),
                        open_world_hint: None,
                    }),
                    meta: None,
                },
            ],
        })
        .await
        .unwrap();

    let snapshot = definitions
        .iter()
        .map(|definition| {
            (
                definition.descriptor.full_name.as_str(),
                definition.descriptor.title.as_str(),
                definition.descriptor.provider_id.as_str(),
                definition.descriptor.provider_kind.as_str(),
                definition.descriptor.tool_info.tool_name.as_str(),
                definition.descriptor.read_only,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        snapshot,
        vec![
            ("mcp__docs-prod__lookup", "Lookup", "docs-prod", "mcp", "lookup", true),
            (
                "mcp__docs-prod__write-note",
                "Write Note",
                "docs-prod",
                "mcp",
                "write-note",
                false,
            ),
        ]
    );
}

#[test]
fn mcp_runtime_auth_error_classifier_preserves_process_status_contract() {
    assert!(is_mcp_auth_error_message("Handshake failed: Unauthorized (401)"));
    assert!(is_mcp_auth_error_message(
        "Ping failed: OAuth token refresh failed: no refresh token available"
    ));
    assert!(is_mcp_auth_error_message("remote server returned status code: 403"));
    assert!(!is_mcp_auth_error_message("Handshake failed: connection reset"));
}

#[test]
fn mcp_runtime_remote_header_merge_preserves_legacy_env_authorization_fallback() {
    let mut env = HashMap::new();
    env.insert("Authorization".to_string(), "legacy-token".to_string());
    env.insert("X-Env".to_string(), "env-only".to_string());

    let headers = HashMap::new();
    let merged = merge_mcp_remote_headers(&headers, &env);
    assert_eq!(merged.get("Authorization").map(String::as_str), Some("legacy-token"));
    assert!(!merged.contains_key("X-Env"));

    let mut explicit_headers = HashMap::new();
    explicit_headers.insert("authorization".to_string(), "Bearer header-token".to_string());
    let merged = merge_mcp_remote_headers(&explicit_headers, &env);
    assert_eq!(
        merged.get("authorization").map(String::as_str),
        Some("Bearer header-token")
    );
    assert!(!merged.contains_key("Authorization"));

    let mut empty_header = HashMap::new();
    empty_header.insert("AUTHORIZATION".to_string(), String::new());
    let merged = merge_mcp_remote_headers(&empty_header, &env);
    assert_eq!(merged.get("AUTHORIZATION").map(String::as_str), Some(""));
    assert!(!merged.contains_key("Authorization"));
}

#[test]
fn mcp_server_config_preserves_transport_defaults_and_validation_contract() {
    let local = MCPServerConfig {
        id: "local".to_string(),
        name: "Local".to_string(),
        server_type: MCPServerType::Local,
        transport: None,
        command: Some("npx".to_string()),
        args: vec!["server".to_string()],
        env: Default::default(),
        headers: Default::default(),
        url: None,
        auto_start: true,
        enabled: true,
        location: ConfigLocation::User,
        capabilities: Vec::new(),
        settings: Default::default(),
        oauth: None,
        xaa: None,
    };
    assert_eq!(local.resolved_transport(), MCPServerTransport::Stdio);
    local.validate().expect("local stdio config is valid");

    let mut remote = local.clone();
    remote.id = "remote".to_string();
    remote.name = "Remote".to_string();
    remote.server_type = MCPServerType::Remote;
    remote.command = None;
    remote.transport = None;
    assert_eq!(
        remote.validate().unwrap_err().to_string(),
        "Remote MCP server 'remote' must have a URL"
    );

    remote.url = Some("https://example.com/mcp".to_string());
    assert_eq!(remote.resolved_transport(), MCPServerTransport::StreamableHttp);
    remote.validate().expect("remote streamable-http config is valid");
}

#[test]
fn mcp_oauth_session_snapshot_preserves_camel_case_status_contract() {
    let snapshot = MCPRemoteOAuthSessionSnapshot::new(
        "remote-server",
        MCPRemoteOAuthStatus::AwaitingBrowser,
        Some("https://auth.example.com/start".to_string()),
        Some("http://127.0.0.1:49152/oauth/callback".to_string()),
        None,
    );

    assert_eq!(
        serde_json::to_value(&snapshot).unwrap(),
        serde_json::json!({
            "serverId": "remote-server",
            "status": "awaitingBrowser",
            "authorizationUrl": "https://auth.example.com/start",
            "redirectUri": "http://127.0.0.1:49152/oauth/callback"
        })
    );
}

// #[tokio::test] // rmcp 1.7: StoredCredentials is non-exhaustive
// async fn mcp_oauth_credential_vault_uses_injected_data_dir_and_roundtrips_credentials() {
//     let unique = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .unwrap()
//         .as_nanos();
//     let data_dir = std::env::temp_dir().join(format!(
//         "northhing-mcp-oauth-vault-contract-{}-{}",
//         std::process::id(),
//         unique
//     ));
//
//     let vault = MCPRemoteOAuthCredentialVault::new(data_dir.clone());
//     let credentials = StoredCredentials {
//         client_id: "client-123".to_string(),
//         token_response: None,
//     };
//
//     vault
//         .store("server-a", &credentials)
//         .await
//         .expect("store credentials");
//
//     assert!(data_dir.join(".mcp_oauth_vault.key").exists());
//     assert!(data_dir.join("mcp_oauth_vault.json").exists());
//
//     let loaded = vault
//         .load("server-a")
//         .await
//         .expect("load credentials")
//         .expect("stored credentials");
//     assert_eq!(loaded.client_id, "client-123");
//     assert!(loaded.token_response.is_none());
//
//     vault.clear("server-a").await.expect("clear credentials");
//     assert!(vault
//         .load("server-a")
//         .await
//         .expect("load after clear")
//         .is_none());
//
//     let _ = std::fs::remove_dir_all(data_dir);
// }

#[test]
fn mcp_cursor_format_helpers_preserve_cursor_compatibility_contract() {
    let remote = MCPServerConfig {
        id: "remote-sse".to_string(),
        name: "Remote SSE".to_string(),
        server_type: MCPServerType::Remote,
        transport: Some(MCPServerTransport::Sse),
        command: None,
        args: Vec::new(),
        env: Default::default(),
        headers: std::collections::HashMap::from([("Authorization".to_string(), "Bearer token".to_string())]),
        url: Some("https://example.com/sse".to_string()),
        auto_start: false,
        enabled: true,
        location: ConfigLocation::User,
        capabilities: Vec::new(),
        settings: Default::default(),
        oauth: None,
        xaa: None,
    };

    assert_eq!(
        config_to_cursor_format(&remote),
        serde_json::json!({
            "type": "sse",
            "name": "Remote SSE",
            "enabled": true,
            "autoStart": false,
            "headers": {
                "Authorization": "Bearer token"
            },
            "url": "https://example.com/sse"
        })
    );

    let parsed = parse_cursor_format(&serde_json::json!({
        "mcpServers": {
            "remote-sse": {
                "type": "sse",
                "url": "https://example.com/sse"
            },
            "unsupported": {
                "type": "container",
                "command": "docker",
                "args": ["run", "--rm", "-i", "example/server"]
            }
        }
    }));

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "remote-sse");
    assert_eq!(parsed[0].server_type, MCPServerType::Remote);
    assert_eq!(parsed[0].transport, Some(MCPServerTransport::Sse));
    assert_eq!(parsed[0].location, ConfigLocation::User);
}
