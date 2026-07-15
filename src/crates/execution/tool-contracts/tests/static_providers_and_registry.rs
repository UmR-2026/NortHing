//! Group 6: static_tool_provider_group, static_tool_materializer,
//! generic_tool_registry, generic_snapshot_tool_decorator,
//! generic_tool_runtime_assembly, materializes_plans,
//! preserves_exposure_catalog tests.

mod common;
use common::*;
use serde_json::json;

struct RegistryMarkerToolFactory;

impl StaticToolProviderFactory<RegistryMarkerTool> for RegistryMarkerToolFactory {
    fn materialize_tool(&self, tool_name: &str) -> Option<Arc<RegistryMarkerTool>> {
        match tool_name {
            "Read" | "Write" | "WebFetch" => Some(registry_marker_tool(tool_name, None)),
            _ => None,
        }
    }
}

struct RegistryMarkerDecorator;

impl ToolDecorator<Arc<RegistryMarkerTool>> for RegistryMarkerDecorator {
    fn decorate(&self, tool: Arc<RegistryMarkerTool>) -> Arc<RegistryMarkerTool> {
        Arc::new(RegistryMarkerTool {
            name: format!("decorated_{}", tool.name),
            provider_id: tool.provider_id.clone(),
            exposure: tool.exposure,
            readonly: tool.readonly,
            enabled: tool.enabled,
        })
    }
}

struct RegistryMarkerSnapshotWrapper;

impl northhing_agent_tools::SnapshotToolWrapper<RegistryMarkerTool> for RegistryMarkerSnapshotWrapper {
    fn wrap_for_snapshot_tracking(&self, tool: Arc<RegistryMarkerTool>) -> Arc<RegistryMarkerTool> {
        Arc::new(RegistryMarkerTool {
            name: format!("snapshot_{}", tool.name),
            provider_id: tool.provider_id.clone(),
            exposure: tool.exposure,
            readonly: tool.readonly,
            enabled: tool.enabled,
        })
    }
}

#[test]
fn static_tool_provider_group_preserves_provider_id_and_tool_order() {
    let provider = StaticToolProviderGroup::new(
        "core-basic",
        vec![registry_marker_tool("Read", None), registry_marker_tool("Write", None)],
    );

    assert_eq!(provider.provider_id(), "core-basic");
    assert_eq!(
        provider.tools().iter().map(|tool| tool.name()).collect::<Vec<_>>(),
        vec!["Read", "Write"]
    );
}

#[test]
fn static_tool_materializer_preserves_provider_and_tool_order() {
    let plans = [
        TestProviderPlan {
            provider_id: "core.basic",
            tool_names: &["Read", "Write"],
        },
        TestProviderPlan {
            provider_id: "core.integration",
            tool_names: &["WebFetch"],
        },
    ];

    let providers = materialize_static_tool_provider_groups(&plans, &RegistryMarkerToolFactory)
        .expect("static tools should materialize");

    assert_eq!(providers[0].provider_id(), "core.basic");
    assert_eq!(
        providers[0].tools().iter().map(|tool| tool.name()).collect::<Vec<_>>(),
        vec!["Read", "Write"]
    );
    assert_eq!(providers[1].provider_id(), "core.integration");
    assert_eq!(
        providers[1].tools().iter().map(|tool| tool.name()).collect::<Vec<_>>(),
        vec!["WebFetch"]
    );
}

#[test]
fn static_tool_materializer_rejects_unknown_tools() {
    let plans = [TestProviderPlan {
        provider_id: "core.basic",
        tool_names: &["Read", "Missing"],
    }];

    let error = materialize_static_tool_provider_groups(&plans, &RegistryMarkerToolFactory)
        .expect_err("unknown tool names must not be silently skipped");

    assert_eq!(
        error,
        StaticToolMaterializationError::UnknownTool {
            provider_id: "core.basic",
            tool_name: "Missing",
        }
    );
}

#[test]
fn generic_tool_registry_installs_static_provider_in_order() {
    let mut registry = ToolRegistry::new();
    let provider = RegistryMarkerProvider {
        provider_id: "core-basic",
        tools: vec![registry_marker_tool("Read", None), registry_marker_tool("Write", None)],
    };

    registry.install_static_provider(&provider);

    assert_eq!(provider.provider_id(), "core-basic");
    assert_eq!(registry.tool_names(), vec!["Read".to_string(), "Write".to_string()]);
}

#[test]
fn generic_tool_registry_applies_decorator_to_static_provider_tools() {
    let mut registry = ToolRegistry::with_tool_decorator(Arc::new(RegistryMarkerDecorator));
    let provider = RegistryMarkerProvider {
        provider_id: "decorated-provider",
        tools: vec![registry_marker_tool("Read", None)],
    };

    registry.install_static_provider(&provider);

    assert_eq!(registry.tool_names(), vec!["decorated_Read".to_string()]);
}

#[test]
fn generic_snapshot_tool_decorator_delegates_to_snapshot_wrapper_port() {
    let decorator: ToolDecoratorRef<RegistryMarkerTool> = Arc::new(northhing_agent_tools::SnapshotToolDecorator::new(
        Arc::new(RegistryMarkerSnapshotWrapper),
    ));
    let providers = vec![StaticToolProviderGroup::new(
        "core-basic",
        vec![registry_marker_tool("Write", None)],
    )];

    let registry =
        ToolRuntimeAssembly::with_tool_decorator(decorator).create_registry_from_static_providers(&providers);

    assert_eq!(
        registry.tool_names(),
        vec!["snapshot_Write".to_string()],
        "snapshot decorator must delegate wrapping through the portable wrapper port"
    );
}

#[test]
fn generic_tool_runtime_assembly_installs_static_providers_with_decorator() {
    let decorator: ToolDecoratorRef<RegistryMarkerTool> = Arc::new(RegistryMarkerDecorator);
    let providers = vec![
        StaticToolProviderGroup::new("core-basic", vec![registry_marker_tool("Read", None)]),
        StaticToolProviderGroup::new(
            "core-integration",
            vec![registry_marker_tool_with_exposure(
                "WebFetch",
                None,
                ToolExposure::Collapsed,
            )],
        ),
    ];

    let registry =
        ToolRuntimeAssembly::with_tool_decorator(decorator).create_registry_from_static_providers(&providers);

    assert_eq!(
        registry.tool_names(),
        vec!["decorated_Read".to_string(), "decorated_WebFetch".to_string()],
        "runtime assembly must preserve static provider order while applying the decorator"
    );
    assert_eq!(
        registry.collapsed_tool_names(),
        vec!["decorated_WebFetch".to_string()],
        "runtime assembly must preserve collapsed exposure after decoration"
    );
}

#[test]
fn generic_tool_runtime_assembly_materializes_plans_before_registry_install() {
    let decorator: ToolDecoratorRef<RegistryMarkerTool> = Arc::new(RegistryMarkerDecorator);
    let plans = [
        TestProviderPlan {
            provider_id: "core.basic",
            tool_names: &["Read", "Write"],
        },
        TestProviderPlan {
            provider_id: "core.integration",
            tool_names: &["WebFetch"],
        },
    ];

    let registry = ToolRuntimeAssembly::with_tool_decorator(decorator)
        .create_registry_from_static_provider_plans(&plans, &RegistryMarkerToolFactory)
        .expect("plans should materialize into a registry");

    assert_eq!(
        registry.tool_names(),
        vec![
            "decorated_Read".to_string(),
            "decorated_Write".to_string(),
            "decorated_WebFetch".to_string()
        ],
        "assembly must own generic plan materialization plus registry installation"
    );
    assert_eq!(
        registry.collapsed_tool_names(),
        Vec::<String>::new(),
        "decorator-based plan materialization must not change exposure"
    );
}

#[test]
fn generic_tool_registry_preserves_exposure_catalog_contract() {
    let mut registry = ToolRegistry::new();
    registry.register_tool(registry_marker_tool("Read", None));
    registry.register_tool(registry_marker_tool_with_exposure(
        "WebFetch",
        None,
        ToolExposure::Collapsed,
    ));
    registry.register_tool(registry_marker_tool_with_exposure("Git", None, ToolExposure::Collapsed));

    assert!(!registry.is_tool_collapsed("Read"));
    assert!(registry.is_tool_collapsed("WebFetch"));
    assert_eq!(
        registry.collapsed_tool_names(),
        vec!["WebFetch".to_string(), "Git".to_string()]
    );
}
