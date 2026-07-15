#![cfg(feature = "mcp")]

//! Context Enhancer And Catalog tests.

mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn mcp_context_enhancer_preserves_resource_selection_contract() {
    let enhancer = MCPContextEnhancer::new(MCPContextEnhancerConfig {
        min_relevance: 0.1,
        max_resources: 1,
        max_total_size: 1024,
        enable_caching: true,
    });

    let context = enhancer
        .enhance(
            "rust mcp",
            vec![
                (
                    make_resource("Rust MCP Guide", Some("runtime docs"), "file://guide.md"),
                    MCPResourceContent {
                        uri: "file://guide.md".to_string(),
                        content: Some("A useful MCP runtime guide".to_string()),
                        blob: None,
                        mime_type: Some("text/plain".to_string()),
                        annotations: None,
                        meta: None,
                    },
                ),
                (
                    make_resource("Unrelated", None, "file://image.png"),
                    MCPResourceContent {
                        uri: "file://image.png".to_string(),
                        content: None,
                        blob: Some("base64".to_string()),
                        mime_type: Some("image/png".to_string()),
                        annotations: None,
                        meta: None,
                    },
                ),
            ],
        )
        .await
        .unwrap();

    assert_eq!(context["type"], "mcp_context");
    assert_eq!(context["query"], "rust mcp");
    assert_eq!(context["resources"].as_array().unwrap().len(), 1);
    assert_eq!(context["resources"][0]["name"], "Rust MCP Guide");
    assert!(context["resources"][0]["relevance_score"].as_f64().unwrap() > 0.0);
}


#[tokio::test]
async fn mcp_catalog_cache_preserves_resource_prompt_lifecycle_contract() {
    let cache = MCPCatalogCache::new();
    let resource = make_resource("readme", Some("docs"), "file:///README.md");
    let prompt = MCPPrompt {
        name: "summarize".to_string(),
        title: Some("Summarize".to_string()),
        description: None,
        arguments: None,
        icons: None,
    };

    cache.replace_resources("server-a", vec![resource.clone()]).await;
    cache.replace_prompts("server-a", vec![prompt.clone()]).await;

    assert_eq!(cache.get_resources("server-a").await[0].name, "readme");
    assert_eq!(cache.get_prompts("server-a").await[0].name, "summarize");
    assert!(cache.get_resources("missing").await.is_empty());

    cache.remove_server("server-a").await;
    assert!(cache.get_resources("server-a").await.is_empty());
    assert!(cache.get_prompts("server-a").await.is_empty());

    cache.replace_resources("server-b", vec![resource]).await;
    cache.replace_prompts("server-b", vec![prompt]).await;
    cache.clear().await;
    assert!(cache.get_resources("server-b").await.is_empty());
    assert!(cache.get_prompts("server-b").await.is_empty());
}


#[tokio::test]
async fn mcp_catalog_cache_replacement_invalidates_stale_entries() {
    let cache = MCPCatalogCache::new();
    let old_resource = make_resource("old", Some("stale"), "file:///old.md");
    let new_resource = make_resource("new", Some("fresh"), "file:///new.md");
    let old_prompt = MCPPrompt {
        name: "old-prompt".to_string(),
        title: None,
        description: Some("stale".to_string()),
        arguments: None,
        icons: None,
    };
    let new_prompt = MCPPrompt {
        name: "new-prompt".to_string(),
        title: None,
        description: Some("fresh".to_string()),
        arguments: None,
        icons: None,
    };

    cache.replace_resources("server-a", vec![old_resource]).await;
    cache.replace_prompts("server-a", vec![old_prompt]).await;
    cache.replace_resources("server-a", vec![new_resource]).await;
    cache.replace_prompts("server-a", vec![new_prompt]).await;

    let resources = cache.get_resources("server-a").await;
    let prompts = cache.get_prompts("server-a").await;
    assert_eq!(
        resources.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
        vec!["new"]
    );
    assert_eq!(
        prompts.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
        vec!["new-prompt"]
    );

    cache.replace_resources("server-a", Vec::new()).await;
    cache.replace_prompts("server-a", Vec::new()).await;
    assert!(cache.get_resources("server-a").await.is_empty());
    assert!(cache.get_prompts("server-a").await.is_empty());
}


