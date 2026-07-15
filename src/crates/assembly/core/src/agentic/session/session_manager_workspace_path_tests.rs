//! Round 9b split: workspace_path tests
//!
//! Test fns moved from session_manager_tests.rs (1:1 with production sibling).
//! Helpers (TestWorkspace, test_manager, etc.) live in the facade
//! and are imported via super::.

#![cfg(test)]
#![allow(unused_imports)]

use super::super::session_store_port::CoreSessionStorePort;
use std::path::PathBuf;

#[tokio::test]
async fn core_session_store_port_resolves_unresolved_remote_storage_path() {
    use northhing_runtime_ports::{SessionStorageKind, SessionStoragePathRequest, SessionStorePort};

    let port = CoreSessionStorePort;
    let resolution = port
        .resolve_session_storage_path(SessionStoragePathRequest {
            workspace_path: PathBuf::from("/remote/project"),
            remote_connection_id: Some("conn-1".to_string()),
            remote_ssh_host: None,
        })
        .await
        .expect("storage path should resolve");

    assert_eq!(resolution.storage_kind(), SessionStorageKind::UnresolvedRemote);
    assert!(resolution.is_remote_storage());
    assert_eq!(resolution.remote_connection_id(), Some("conn-1"));
    assert_ne!(resolution.effective_storage_path(), &PathBuf::from("/remote/project"));
}
