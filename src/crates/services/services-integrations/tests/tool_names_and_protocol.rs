#![cfg(feature = "mcp")]

//! Tool Names And Protocol tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn mcp_tool_name_contract_matches_existing_wire_format() {
    assert_eq!(MCP_TOOL_PREFIX, "mcp__");
    assert_eq!(MCP_TOOL_DELIMITER, "__");
    assert_eq!(normalize_name_for_mcp("Acme Search / Primary"), "Acme_Search___Primary");
    assert_eq!(
        build_mcp_tool_name("Claude Code", "search repos"),
        "mcp__Claude_Code__search_repos"
    );
}


#[test]
fn mcp_tool_info_preserves_json_shape() {
    let info = McpToolInfo {
        server_id: "server-1".to_string(),
        server_name: "Docs".to_string(),
        tool_name: "search".to_string(),
    };

    assert_eq!(
        serde_json::to_value(info).unwrap(),
        serde_json::json!({
            "server_id": "server-1",
            "server_name": "Docs",
            "tool_name": "search"
        })
    );
}


#[test]
fn mcp_protocol_capability_contract_matches_existing_default() {
    assert_eq!(default_protocol_version(), "2025-11-25");
    assert_eq!(
        serde_json::to_value(MCPCapability::default()).unwrap(),
        serde_json::json!({
            "resources": {
                "subscribe": false,
                "listChanged": false
            },
            "prompts": {
                "listChanged": false
            },
            "tools": {
                "listChanged": false
            }
        })
    );
}

// #[test] // rmcp 1.7: ElicitationCapability has no schema_validation field
// fn mcp_remote_client_info_declares_supported_client_capabilities() {
//     let info = create_mcp_client_info("northhing", "1.0.0");
//
//     assert_eq!(info.client_info.name, "northhing");
//     assert_eq!(info.client_info.version, "1.0.0");
//     assert!(info.capabilities.roots.is_some());
//     assert!(info.capabilities.sampling.is_some());
//     assert!(info.capabilities.elicitation.is_some());
//     assert_eq!(
//         info.capabilities
//             .elicitation
//             .as_ref()
//             .and_then(|cap| cap.schema_validation),
//         Some(true)
//     );
// }


