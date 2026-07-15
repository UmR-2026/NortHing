use northhing_services_core::json_store::{JsonFileStore, JsonFileStoreError};
use northhing_test_support::TestTempDir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TestPayload {
    label: String,
    count: u32,
}

#[tokio::test]
async fn json_store_returns_none_for_missing_file() {
    let root = TestTempDir::new("missing");
    let store = JsonFileStore::default();

    let value = store
        .read_optional::<TestPayload>(&root.path().join("missing.json"))
        .await
        .expect("missing file should not be an error");

    assert_eq!(value, None);
}

#[tokio::test]
async fn json_store_creates_parent_dirs_and_round_trips_payload() {
    let root = TestTempDir::new("round-trip");
    let store = JsonFileStore::default();
    let path = root.path().join("nested").join("payload.json");
    let payload = TestPayload {
        label: "session metadata".to_string(),
        count: 3,
    };

    store
        .write_atomic(&path, &payload)
        .await
        .expect("write should create parent dir");
    let loaded = store
        .read_optional::<TestPayload>(&path)
        .await
        .expect("written payload should be readable");

    assert_eq!(loaded, Some(payload));
}

#[tokio::test]
async fn json_store_reports_no_parent_directory() {
    let store = JsonFileStore::default();

    let error = store
        .write_atomic(
            Path::new(""),
            &TestPayload {
                label: "rootless".to_string(),
                count: 1,
            },
        )
        .await
        .expect_err("empty path has no parent component");

    assert!(matches!(error, JsonFileStoreError::NoParentDirectory { .. }));
}
