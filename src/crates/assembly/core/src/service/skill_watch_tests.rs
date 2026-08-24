use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestEmitter {
    emit_count: AtomicUsize,
}

#[async_trait::async_trait]
impl EventEmitter for TestEmitter {
    async fn emit(&self, event_name: &str, _payload: serde_json::Value) -> anyhow::Result<()> {
        if event_name == SKILLS_CHANGED_EVENT_NAME {
            self.emit_count.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_skill_watch_service_lifecycle_dispose() {
    let ws_service = Arc::new(WorkspaceService::new().await.expect("workspace service init"));
    let service = SkillWatchService::new(ws_service);
    let emitter = Arc::new(TestEmitter {
        emit_count: AtomicUsize::new(0),
    });

    service.set_event_emitter(emitter).await.expect("set emitter");
    assert!(!service.watched_paths().await.is_empty());

    service.dispose().await;
}

#[tokio::test]
async fn test_skill_watch_service_sync_rebuild() {
    let ws_service = Arc::new(WorkspaceService::new().await.expect("workspace service init"));
    let service = SkillWatchService::new(ws_service);
    let emitter = Arc::new(TestEmitter {
        emit_count: AtomicUsize::new(0),
    });

    service.set_event_emitter(emitter).await.expect("set emitter");
    let initial_paths = service.watched_paths().await;
    assert!(!initial_paths.is_empty());

    service.sync_watched_paths().await.expect("sync paths");
    let rebuilt_paths = service.watched_paths().await;
    assert_eq!(initial_paths, rebuilt_paths);

    service.dispose().await;
}

#[tokio::test]
async fn test_skill_watch_service_debounce_window() {
    let emitter = Arc::new(TestEmitter {
        emit_count: AtomicUsize::new(0),
    });
    let emitter_slot = Arc::new(Mutex::new(Some(emitter.clone() as Arc<dyn EventEmitter>)));
    let pending = Arc::new(Mutex::new(None));

    for _ in 0..5 {
        SkillWatchService::schedule_refresh(emitter_slot.clone(), pending.clone()).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    for _ in 0..20 {
        if emitter.emit_count.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert_eq!(
        emitter.emit_count.load(Ordering::SeqCst),
        1,
        "Multiple rapid triggers must result in exactly 1 debounced emission"
    );
}

#[tokio::test]
async fn test_skill_watch_service_concurrent_syncs_serialize() {
    // Boot path fires sync_watched_paths twice in quick succession (init spawn
    // + set_event_emitter). The sync_lock must serialize them: both complete
    // without error and the end state still reports a full watched-path set
    // that a subsequent sync reproduces identically.
    let ws_service = Arc::new(WorkspaceService::new().await.expect("workspace service init"));
    let service = SkillWatchService::new(ws_service);

    let (ra, rb) = tokio::join!(service.sync_watched_paths(), service.sync_watched_paths());
    ra.expect("concurrent sync a");
    rb.expect("concurrent sync b");

    let after_race = service.watched_paths().await;
    assert!(!after_race.is_empty(), "watched paths survive the sync race");

    service.sync_watched_paths().await.expect("post-race sync");
    assert_eq!(after_race, service.watched_paths().await);

    service.dispose().await;
}
