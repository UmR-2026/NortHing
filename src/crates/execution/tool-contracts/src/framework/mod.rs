//! Portable tool framework contracts (facade).
//!
//! R37b split of the former 2189-line `framework.rs` god-file into sub-domain
//! siblings. This facade re-exports every sibling so `framework::<Name>` paths
//! (and the crate-root re-exports in `lib.rs`) stay stable.
//!
//! Siblings:
//!   - `types`    -- DTOs, restrictions, validation, result
//!   - `manifest` -- manifest / exposure policy / GetToolSpec helpers
//!   - `catalog`  -- registry-item traits + context-aware resolution runtimes
//!   - `registry` -- static providers, decorators, `ToolRegistry`
//!   - `paths`    -- path resolution, runtime-URI contracts, path policy

mod catalog;
mod manifest;
mod paths;
mod registry;
mod types;

pub use catalog::*;
pub use manifest::*;
pub use paths::*;
pub use registry::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use indexmap::IndexMap;
    use serde_json::{json, Value};
    use std::sync::Arc;

    struct TestTool {
        name: &'static str,
        available: bool,
    }

    #[async_trait]
    impl ToolRegistryItem for TestTool {
        fn name(&self) -> &str {
            self.name
        }

        async fn description(&self) -> Result<String, String> {
            Ok(format!("{} description", self.name))
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {},
            })
        }
    }

    #[async_trait]
    impl ContextualToolManifestItem<()> for TestTool {
        async fn is_available_in_context(&self, _context: &()) -> bool {
            self.available
        }
    }

    #[tokio::test]
    async fn contextual_manifest_omits_unavailable_tools_from_model_definitions() {
        let task = Arc::new(TestTool {
            name: "Task",
            available: false,
        });
        let read = Arc::new(TestTool {
            name: "Read",
            available: true,
        });
        let tools: Vec<Arc<TestTool>> = vec![task, read];
        let allowed_tools = vec!["Task".to_string(), "Read".to_string()];

        let manifest =
            resolve_contextual_tool_manifest(&tools, &allowed_tools, &IndexMap::new(), &(), GET_TOOL_SPEC_TOOL_NAME)
                .await;

        assert!(!manifest
            .tool_definitions
            .iter()
            .any(|definition| definition.name == "Task"));
        assert!(manifest
            .tool_definitions
            .iter()
            .any(|definition| definition.name == "Read"));
    }

    #[test]
    fn get_tool_spec_description_preserves_prompt_contract() {
        let description = build_get_tool_spec_description();

        assert!(description.contains("Read full schema"));
        assert!(description.contains("Do not call GetToolSpec again"));
    }

    #[test]
    fn get_tool_spec_catalog_description_lists_names_only() {
        let description = build_get_tool_spec_catalog_description(&[
            GetToolSpecCollapsedToolSummary {
                name: "Git".to_string(),
                short_description: "Inspect repository state.".to_string(),
            },
            GetToolSpecCollapsedToolSummary {
                name: "WebFetch".to_string(),
                short_description: "Fetch a URL.".to_string(),
            },
        ])
        .expect("catalog description");

        assert!(description.contains("- Git"));
        assert!(description.contains("- WebFetch"));
        assert!(!description.contains("Inspect repository state."));
        assert!(!description.contains("Fetch a URL."));
    }

    #[tokio::test]
    async fn test_tool_registration_guard_unregisters_on_drop() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            name: "GuardedTool",
            available: true,
        });

        assert!(registry.get_tool("GuardedTool").is_none());
        let guard = registry.register_tool_guarded(tool);
        assert!(registry.get_tool("GuardedTool").is_some());
        assert_eq!(guard.name(), "GuardedTool");

        drop(guard);
        assert!(registry.get_tool("GuardedTool").is_none());
    }

    #[tokio::test]
    async fn test_tool_registration_guard_does_not_unregister_if_overwritten() {
        let mut registry = ToolRegistry::new();
        let tool_v1 = Arc::new(TestTool {
            name: "OverwrittenTool",
            available: true,
        });
        let tool_v2 = Arc::new(TestTool {
            name: "OverwrittenTool",
            available: true,
        });

        let guard1 = registry.register_tool_guarded(tool_v1.clone());
        let current1 = registry.get_tool("OverwrittenTool");
        assert!(current1.is_some_and(|t| Arc::ptr_eq(&t, &tool_v1)));

        let guard2 = registry.register_tool_guarded(tool_v2.clone());
        let current2 = registry.get_tool("OverwrittenTool");
        assert!(current2.is_some_and(|t| Arc::ptr_eq(&t, &tool_v2)));

        // Dropping guard1 must NOT remove tool_v2
        drop(guard1);
        assert!(registry.get_tool("OverwrittenTool").is_some());
        let current3 = registry.get_tool("OverwrittenTool");
        assert!(current3.is_some_and(|t| Arc::ptr_eq(&t, &tool_v2)));

        // Dropping guard2 removes tool_v2
        drop(guard2);
        assert!(registry.get_tool("OverwrittenTool").is_none());
    }

    #[tokio::test]
    async fn test_tool_registration_guard_manual_dispose_and_disarm() {
        let mut registry = ToolRegistry::new();
        let tool1 = Arc::new(TestTool {
            name: "Tool1",
            available: true,
        });
        let tool2 = Arc::new(TestTool {
            name: "Tool2",
            available: true,
        });

        let mut guard1 = registry.register_tool_guarded(tool1);
        let guard2 = registry.register_tool_guarded(tool2);

        assert!(registry.get_tool("Tool1").is_some());
        assert!(registry.get_tool("Tool2").is_some());

        // Explicit dispose
        guard1.dispose();
        assert!(registry.get_tool("Tool1").is_none());
        // Second dispose is no-op
        guard1.dispose();
        drop(guard1);
        assert!(registry.get_tool("Tool1").is_none());

        // Disarm guard2
        guard2.disarm();
        assert!(registry.get_tool("Tool2").is_some());
    }
}
