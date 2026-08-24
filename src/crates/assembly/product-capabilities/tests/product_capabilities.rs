use northhing_product_capabilities::{
    default_product_assembly_plan, default_product_capability_assembly, default_product_capability_registry,
    product_assembly_plan_for_profile, DeliveryProfile, ProductCapabilityId, ProductServiceCapabilityRequirement,
    ProductServiceCapabilityStatus,
};
use northhing_runtime_ports::RuntimeServiceCapability;

#[test]
fn capability_packs_describe_service_requirements() {
    let registry = default_product_capability_registry();

    let capability_ids = registry
        .capability_ids()
        .into_iter()
        .map(ProductCapabilityId::id)
        .collect::<Vec<_>>();
    assert_eq!(
        capability_ids,
        vec!["code-agent", "deep-review", "deep-research"]
    );

    let service_capabilities = registry.required_service_capabilities();
    assert!(service_capabilities.contains(&RuntimeServiceCapability::FileSystem));
    assert!(service_capabilities.contains(&RuntimeServiceCapability::Workspace));
    assert!(service_capabilities.contains(&RuntimeServiceCapability::Permission));
    assert!(service_capabilities.contains(&RuntimeServiceCapability::Events));
}

#[test]
fn product_assembly_plan_makes_delivery_profile_explicit_without_reducing_capabilities() {
    let expected_capabilities = vec!["code-agent", "deep-review", "deep-research"];

    for profile in DeliveryProfile::all_current_product_profiles().iter().copied() {
        let plan = product_assembly_plan_for_profile(profile);

        assert_eq!(plan.profile(), profile);
        assert_eq!(
            plan.capability_set()
                .ids()
                .iter()
                .map(|capability_id| capability_id.id())
                .collect::<Vec<_>>(),
            expected_capabilities,
            "{profile} must preserve the current product-full capability set until explicit trimming is proven"
        );
    }
}

#[test]
fn product_assembly_plan_reports_service_availability_by_capability() {
    let plan = default_product_assembly_plan();

    let unavailable = plan
        .service_availability_report(|capability| {
            !matches!(
                capability,
                RuntimeServiceCapability::Git | RuntimeServiceCapability::Network
            )
        })
        .into_iter()
        .filter(|entry| entry.status() == ProductServiceCapabilityStatus::Unavailable)
        .collect::<Vec<_>>();

    assert_eq!(unavailable.len(), 2);
    assert_eq!(
        unavailable[0].requirement(),
        ProductServiceCapabilityRequirement::new(ProductCapabilityId::DeepReview, RuntimeServiceCapability::Git,)
    );
    assert_eq!(
        unavailable[1].requirement(),
        ProductServiceCapabilityRequirement::new(ProductCapabilityId::DeepResearch, RuntimeServiceCapability::Network,)
    );
}

#[test]
fn default_capability_assembly_keeps_service_facts_together() {
    let assembly = default_product_capability_assembly();

    let capability_ids = assembly
        .capability_ids()
        .iter()
        .map(|capability_id| capability_id.id())
        .collect::<Vec<_>>();
    assert_eq!(
        capability_ids,
        vec!["code-agent", "deep-review", "deep-research"]
    );

    let service_capabilities = assembly.required_service_capabilities();
    assert_eq!(
        service_capabilities,
        vec![
            RuntimeServiceCapability::FileSystem,
            RuntimeServiceCapability::Workspace,
            RuntimeServiceCapability::SessionStore,
            RuntimeServiceCapability::Permission,
            RuntimeServiceCapability::Events,
            RuntimeServiceCapability::Clock,
            RuntimeServiceCapability::Terminal,
            RuntimeServiceCapability::Git,
            RuntimeServiceCapability::Network,
        ]
    );
}

#[test]
fn capability_assembly_reports_missing_services_without_concrete_runtime_dependency() {
    let assembly = default_product_capability_assembly();

    let missing = assembly.missing_service_requirements(|capability| {
        !matches!(
            capability,
            RuntimeServiceCapability::Git | RuntimeServiceCapability::Network
        )
    });

    assert_eq!(
        missing,
        vec![
            ProductServiceCapabilityRequirement::new(ProductCapabilityId::DeepReview, RuntimeServiceCapability::Git,),
            ProductServiceCapabilityRequirement::new(
                ProductCapabilityId::DeepResearch,
                RuntimeServiceCapability::Network,
            ),
        ]
    );

    assert!(
        assembly.missing_service_requirements(|_capability| true).is_empty(),
        "fully assembled product runtime must report no service capability gaps"
    );
}
