#![cfg(feature = "mcp")]

//! Request Builders And Adapters tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn mcp_protocol_jsonrpc_helpers_preserve_wire_shape() {
    let request = MCPRequest::new(
        serde_json::json!(7),
        "tools/list".to_string(),
        Some(serde_json::json!({ "cursor": "next" })),
    );

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/list",
            "params": {
                "cursor": "next"
            }
        })
    );

    assert_eq!(
        serde_json::to_value(MCPError::method_not_found("tools/call")).unwrap(),
        serde_json::json!({
            "code": -32601,
            "message": "Method not found: tools/call"
        })
    );
}


#[test]
fn mcp_protocol_request_builders_preserve_wire_shape() {
    assert_eq!(
        serde_json::to_value(create_initialize_request(9, "northhing", "0.2.6")).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
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
                },
                "clientInfo": {
                    "name": "northhing",
                    "version": "0.2.6",
                    "description": "northhing MCP Client",
                    "vendor": "northhing"
                }
            }
        })
    );

    assert_eq!(
        serde_json::to_value(create_tools_list_request(10, None)).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/list"
        })
    );

    assert_eq!(
        serde_json::to_value(create_tools_list_request(11, Some("cursor-1".to_string()))).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/list",
            "params": {
                "cursor": "cursor-1"
            }
        })
    );

    assert_eq!(
        serde_json::to_value(create_tools_call_request(
            12,
            "search",
            Some(serde_json::json!({ "query": "rust" }))
        ))
        .unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": "rust"
                }
            }
        })
    );

    assert_eq!(
        serde_json::to_value(create_ping_request(13)).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "ping",
            "params": {}
        })
    );
}


#[test]
fn mcp_protocol_prompt_content_helpers_preserve_legacy_text_behavior() {
    let mut content = MCPPromptMessageContent::Plain("Review {{target}}".to_string());
    content.substitute_placeholders(&std::collections::HashMap::from([(
        "target".to_string(),
        "src/main.rs".to_string(),
    )]));

    assert_eq!(content.text_or_placeholder(), "Review src/main.rs");

    let image = MCPPromptMessageContent::Block(Box::new(MCPPromptMessageContentBlock::Image {
        data: "base64".to_string(),
        mime_type: "image/png".to_string(),
    }));
    assert_eq!(image.text_or_placeholder(), "[Image: image/png]");
}


#[test]
fn mcp_resource_and_prompt_adapters_preserve_context_rendering_contract() {
    let resource = MCPResource {
        title: Some("Design Notes".to_string()),
        metadata: Some(HashMap::from([("source".to_string(), serde_json::json!("fixture"))])),
        ..make_resource("notes", Some("project notes"), "file:///workspace/notes.md")
    };
    let content = MCPResourceContent {
        uri: resource.uri.clone(),
        content: Some("alpha beta".to_string()),
        blob: None,
        mime_type: Some("text/markdown".to_string()),
        annotations: None,
        meta: None,
    };

    assert_eq!(
        ResourceAdapter::to_context_block(&resource, Some(&content)),
        serde_json::json!({
            "type": "resource",
            "uri": "file:///workspace/notes.md",
            "name": "notes",
            "title": "Design Notes",
            "displayName": "Design Notes",
            "description": "project notes",
            "mimeType": "text/plain",
            "size": 12,
            "content": "alpha beta",
            "metadata": {
                "source": "fixture"
            }
        })
    );
    assert_eq!(
        ResourceAdapter::to_text(&content),
        "Resource: file:///workspace/notes.md\n\nalpha beta\n"
    );

    let ranked = ResourceAdapter::filter_and_rank(
        vec![
            make_resource("readme", Some("install guide"), "file:///README.md"),
            make_resource("report", Some("quarterly guide"), "file:///report.md"),
            make_resource("other", Some("misc"), "file:///other.md"),
        ],
        "guide",
        0.3,
        2,
    );
    assert_eq!(
        ranked
            .iter()
            .map(|(resource, _)| resource.name.as_str())
            .collect::<Vec<_>>(),
        vec!["readme", "report"]
    );

    let prompt = MCPPrompt {
        name: "review".to_string(),
        title: None,
        description: None,
        arguments: Some(vec![MCPPromptArgument {
            name: "target".to_string(),
            title: None,
            description: None,
            required: true,
        }]),
        icons: None,
    };
    assert!(!PromptAdapter::is_applicable(&prompt, &HashMap::new()));
    assert!(PromptAdapter::is_applicable(
        &prompt,
        &HashMap::from([("target".to_string(), "src/lib.rs".to_string())])
    ));

    let messages = PromptAdapter::substitute_arguments(
        vec![MCPPromptMessage {
            role: "user".to_string(),
            content: MCPPromptMessageContent::Plain("Review {{target}}".to_string()),
        }],
        &HashMap::from([("target".to_string(), "src/lib.rs".to_string())]),
    );
    let prompt_text = PromptAdapter::to_system_prompt(&MCPPromptContent {
        name: "review".to_string(),
        messages,
    });
    assert_eq!(prompt_text, "User: Review src/lib.rs");
}


