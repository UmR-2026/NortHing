use std::sync::Arc;

use northhing_runtime_ports::FileSystemPort;
use northhing_runtime_ports::{RuntimeServiceCapability, SessionStorageKind, SessionStoragePathRequest};
use northhing_runtime_services::test_support::{FakeRuntimePort, FakeRuntimeServicesProvider};
use northhing_runtime_services::{
    CapabilityAvailability, RuntimeServicesBuilder, RuntimeServicesError, RuntimeServicesProvider,
    RuntimeServicesRegistry,
};

#[test]
fn builder_requires_mandatory_runtime_services() {
    let error = RuntimeServicesBuilder::new().build().unwrap_err();

    assert_eq!(
        error,
        RuntimeServicesError::MissingRequired {
            capability: RuntimeServiceCapability::FileSystem,
        }
    );
}

#[test]
fn fake_provider_registers_required_services_through_registry() {
    let registry =
        RuntimeServicesRegistry::new().with_provider(FakeRuntimeServicesProvider::with_all_required());
    let services = registry
        .build(RuntimeServicesBuilder::new())
        .expect("fake provider should satisfy runtime services");

    assert!(services.has_capability(RuntimeServiceCapability::FileSystem));
    assert!(services.has_capability(RuntimeServiceCapability::Workspace));
    assert!(services.has_capability(RuntimeServiceCapability::SessionStore));
    assert!(services.has_capability(RuntimeServiceCapability::Permission));
    assert!(services.has_capability(RuntimeServiceCapability::Events));
    assert!(services.has_capability(RuntimeServiceCapability::Clock));
}

#[test]
fn missing_optional_capability_returns_typed_unsupported_error() {
    let services = FakeRuntimeServicesProvider::with_all_required()
        .build_services()
        .expect("required fake services should build");

    let error = services
        .require_capability(RuntimeServiceCapability::Terminal)
        .unwrap_err();

    assert_eq!(
        error,
        RuntimeServicesError::Unsupported {
            capability: RuntimeServiceCapability::Terminal,
        }
    );
}

#[test]
fn capability_availability_reports_optional_service_status_without_side_effects() {
    let services = FakeRuntimeServicesProvider::with_all_required()
        .build_services()
        .expect("required fake services should build");

    assert_eq!(
        services.capability_availability(RuntimeServiceCapability::FileSystem),
        CapabilityAvailability {
            capability: RuntimeServiceCapability::FileSystem,
            available: true,
        }
    );
    assert_eq!(
        services.capability_availability(RuntimeServiceCapability::Terminal),
        CapabilityAvailability {
            capability: RuntimeServiceCapability::Terminal,
            available: false,
        }
    );
}

#[test]
fn builder_rejects_port_registered_under_the_wrong_capability() {
    let mismatched_filesystem: Arc<dyn FileSystemPort> = Arc::new(FakeRuntimePort::new(RuntimeServiceCapability::Git));
    let builder = FakeRuntimeServicesProvider::with_all_required()
        .register(RuntimeServicesBuilder::new())
        .with_filesystem(mismatched_filesystem);

    let error = builder.build().unwrap_err();

    assert_eq!(
        error,
        RuntimeServicesError::CapabilityMismatch {
            expected: RuntimeServiceCapability::FileSystem,
            actual: RuntimeServiceCapability::Git,
        }
    );
}

#[tokio::test]
async fn registered_session_store_port_exposes_storage_path_resolution() {
    let services = FakeRuntimeServicesProvider::with_all_required()
        .build_services()
        .expect("required fake services should build");

    let resolution = services
        .session_store
        .resolve_session_storage_path(SessionStoragePathRequest {
            workspace_path: "/workspace".into(),
            remote_connection_id: None,
            remote_ssh_host: None,
        })
        .await
        .expect("fake session store should resolve local path");

    assert_eq!(resolution.storage_kind(), SessionStorageKind::Local);
    assert_eq!(resolution.effective_storage_path().to_string_lossy(), "/workspace");
}
