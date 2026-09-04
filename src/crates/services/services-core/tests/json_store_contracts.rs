use northhing_services_core::json_store::{io_timeout, JsonFileStore, JsonFileStoreError};
use northhing_test_support::TestTempDir;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

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

#[tokio::test]
async fn json_store_write_bytes_atomic_round_trips_raw_bytes() {
    let root = TestTempDir::new("bytes-round-trip");
    let store = JsonFileStore::default();
    let path = root.path().join("nested").join("secret.key");
    let bytes = [42u8; 32];

    store
        .write_bytes_atomic(&path, &bytes)
        .await
        .expect("write_bytes_atomic should succeed");
    let loaded = tokio::fs::read(&path)
        .await
        .expect("written bytes should be readable");

    assert_eq!(loaded, bytes);
}

#[tokio::test]
async fn json_store_write_bytes_atomic_overwrites_and_cleans_up_temp_files() {
    let root = TestTempDir::new("bytes-overwrite");
    let store = JsonFileStore::default();
    let path = root.path().join("key.bin");

    store
        .write_bytes_atomic(&path, b"initial key A")
        .await
        .expect("first write should succeed");

    store
        .write_bytes_atomic(&path, b"updated key B")
        .await
        .expect("overwrite should succeed");

    let loaded = tokio::fs::read(&path).await.expect("file should be readable");
    assert_eq!(loaded, b"updated key B");

    let mut entries = tokio::fs::read_dir(root.path()).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(!name_str.ends_with(".tmp"), "found leftover temp file: {}", name_str);
    }
}

#[tokio::test]
async fn json_store_io_timeout_read_with_pending_future_triggers_timeout() {
    let result = io_timeout(
        Path::new("sessions/5da38044/state.json"),
        "read_to_string",
        Duration::from_millis(10),
        std::future::pending::<std::io::Result<String>>(),
    )
    .await;

    let err = result.expect_err("pending read future must time out");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(err.to_string().contains("read_to_string"));
    assert!(err.to_string().contains("timed out after 10ms"));
}

#[tokio::test]
async fn json_store_io_timeout_metadata_with_pending_future_triggers_timeout() {
    let result = io_timeout(
        Path::new("sessions/5da38044/state.json"),
        "metadata",
        Duration::from_millis(10),
        std::future::pending::<std::io::Result<std::fs::Metadata>>(),
    )
    .await;

    let err = result.expect_err("pending metadata future must time out");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(err.to_string().contains("metadata"));
    assert!(err.to_string().contains("timed out after 10ms"));
}

#[tokio::test]
async fn json_store_io_timeout_write_with_pending_future_triggers_timeout() {
    let result = io_timeout(
        Path::new("sessions/5da38044/state.json.tmp"),
        "write_temp",
        Duration::from_millis(10),
        std::future::pending::<std::io::Result<()>>(),
    )
    .await;

    let err = result.expect_err("pending write future must time out");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(err.to_string().contains("write_temp"));
    assert!(err.to_string().contains("timed out after 10ms"));
}

#[tokio::test]
async fn json_store_io_timeout_replace_with_pending_future_triggers_timeout() {
    let result = io_timeout(
        Path::new("sessions/5da38044/state.json"),
        "rename_replace",
        Duration::from_millis(10),
        std::future::pending::<std::io::Result<()>>(),
    )
    .await;

    let err = result.expect_err("pending replace future must time out");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(err.to_string().contains("rename_replace"));
    assert!(err.to_string().contains("timed out after 10ms"));
}

#[test]
fn json_store_timed_out_is_retryable() {
    let error = std::io::Error::new(std::io::ErrorKind::TimedOut, "operation timed out");
    assert!(JsonFileStore::is_retryable_write_error(&error));
}
