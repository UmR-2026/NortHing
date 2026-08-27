#![cfg(feature = "file-watch")]

use std::sync::Arc;
use tokio::sync::Mutex;
use northhing_events::EventEmitter;
use northhing_services_integrations::file_watch::{FileWatchEventKind, FileWatchService, FileWatcherConfig};
use northhing_test_support::TestTempDir;

struct TestEmitter {
    events: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

#[async_trait::async_trait]
impl EventEmitter for TestEmitter {
    async fn emit(&self, event_name: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        let mut events = self.events.lock().await;
        events.push((event_name.to_string(), payload));
        Ok(())
    }
}

#[tokio::test]
async fn file_watch_preserves_missing_path_error() {
    let service = FileWatchService::new(FileWatcherConfig::default());

    let error = service
        .watch_path("__northhing_missing_watch_path_for_services_integrations_test__", None)
        .await
        .expect_err("missing paths should keep the existing error contract");

    assert_eq!(error, "Path does not exist");
}

#[test]
fn file_watch_event_kind_serializes_snake_case() {
    let value = serde_json::to_value(FileWatchEventKind::Modify).expect("serialize event kind");

    assert_eq!(value, "modify");
}

#[tokio::test]
async fn file_watch_incremental_watch_and_unwatch_delivers_events() {
    let dir_a = TestTempDir::new("file-watch-a");
    let dir_b = TestTempDir::new("file-watch-b");

    let emitted_events = Arc::new(Mutex::new(Vec::new()));
    let emitter = Arc::new(TestEmitter {
        events: emitted_events.clone(),
    });

    let config = FileWatcherConfig {
        watch_recursively: true,
        ignore_hidden_files: false,
        debounce_interval_ms: 20,
        max_events_per_interval: 100,
    };

    let service = FileWatchService::new(config);
    service.set_emitter(emitter).await;

    // 1. Initial watch on directory A
    service
        .watch_path(dir_a.path().to_str().unwrap(), None)
        .await
        .expect("watch dir_a should succeed");

    let watched = service.get_watched_paths().await;
    assert_eq!(watched.len(), 1);

    // 2. Incremental watch on directory B
    service
        .watch_path(dir_b.path().to_str().unwrap(), None)
        .await
        .expect("incremental watch dir_b should succeed");

    let watched = service.get_watched_paths().await;
    assert_eq!(watched.len(), 2);

    // 3. Unwatch directory A
    service
        .unwatch_path(dir_a.path().to_str().unwrap())
        .await
        .expect("unwatch dir_a should succeed");

    let watched = service.get_watched_paths().await;
    assert_eq!(watched.len(), 1);

    // Settle interval
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 4. Trigger filesystem changes in dir_b and unwatched dir_a
    let file_b = dir_b.path().join("target_file_b.txt");
    std::fs::write(&file_b, "hello world in b").expect("write target_file_b");

    let file_a = dir_a.path().join("ignored_file_a.txt");
    std::fs::write(&file_a, "hello world in a").expect("write ignored_file_a");

    // 5. Poll for emitted events
    let start = std::time::Instant::now();
    let mut found_b = false;
    let mut found_a = false;

    while start.elapsed() < std::time::Duration::from_secs(3) {
        {
            let events = emitted_events.lock().await;
            for (name, payload) in events.iter() {
                if name == "file-system-changed" {
                    if let Some(arr) = payload.as_array() {
                        for item in arr {
                            if let Some(path_str) = item.get("path").and_then(|p| p.as_str()) {
                                if path_str.contains("target_file_b.txt") {
                                    found_b = true;
                                }
                                if path_str.contains("ignored_file_a.txt") {
                                    found_a = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if found_b {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Final check for unwanted dir_a events
    {
        let events = emitted_events.lock().await;
        for (name, payload) in events.iter() {
            if name == "file-system-changed" {
                if let Some(arr) = payload.as_array() {
                    for item in arr {
                        if let Some(path_str) = item.get("path").and_then(|p| p.as_str()) {
                            if path_str.contains("ignored_file_a.txt") {
                                found_a = true;
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(found_b, "filesystem change in dir_b must be emitted");
    assert!(!found_a, "filesystem change in unwatched dir_a must not be emitted");

    // 6. Unwatch remaining directory B -> watcher becomes empty
    service
        .unwatch_path(dir_b.path().to_str().unwrap())
        .await
        .expect("unwatch dir_b should succeed");

    assert!(service.get_watched_paths().await.is_empty());
}

#[tokio::test]
async fn file_watch_unwatch_unknown_path_is_noop() {
    let dir = TestTempDir::new("file-watch-noop");
    let service = FileWatchService::new(FileWatcherConfig::default());

    service
        .watch_path(dir.path().to_str().unwrap(), None)
        .await
        .expect("watch dir should succeed");

    // Unwatching an unknown path should not error or remove existing path
    service
        .unwatch_path("__unknown_path_never_watched__")
        .await
        .expect("unwatching unknown path should be a no-op");

    let watched = service.get_watched_paths().await;
    assert_eq!(watched.len(), 1);

    service
        .unwatch_path(dir.path().to_str().unwrap())
        .await
        .expect("cleanup unwatch");
    assert!(service.get_watched_paths().await.is_empty());
}
