//! Tests for transactional bot persistence (H-6 audit fix): single-writer
//! `update_bot_persistence`, fail-closed loads, atomic writes with `.bak`
//! backup, and the legacy-file migration path.
//!
//! Path isolation: the tests exercise the path-parameterized internals
//! (`*_at` variants) against `TestTempDir` paths instead of touching
//! `dirs::home_dir()`, so they are parallel-safe and never touch the real
//! user profile.

use super::command_router::BotChatState;
use super::{
    load_bot_persistence_at, try_load_bot_persistence_at, update_bot_persistence_at, BotConfig, BotPersistenceData,
    BotPersistenceError, SavedBotConnection,
};
use northhing_test_support::TestTempDir;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn test_bot(chat_id: &str) -> SavedBotConnection {
    SavedBotConnection {
        bot_type: chat_id.to_string(),
        chat_id: chat_id.to_string(),
        config: BotConfig::Telegram {
            bot_token: format!("token-{chat_id}"),
        },
        chat_state: BotChatState::new(chat_id.to_string()),
        connected_at: 0,
    }
}

fn persistence_paths(dir: &TestTempDir) -> (PathBuf, PathBuf) {
    (
        dir.path().join("remote_connect_persistence.json"),
        dir.path().join("bot_connections.json"),
    )
}

#[test]
fn concurrent_updates_do_not_lose_entries() {
    let dir = TestTempDir::new("bot-persistence-concurrent");
    let (main, legacy) = persistence_paths(&dir);

    std::thread::scope(|scope| {
        for i in 0..10 {
            let main = main.clone();
            let legacy = legacy.clone();
            scope.spawn(move || {
                update_bot_persistence_at(&main, &legacy, |data| {
                    data.connections.push(test_bot(&format!("bot-{i}")));
                })
                .unwrap();
            });
        }
    });

    let data = try_load_bot_persistence_at(&main, &legacy).unwrap();
    assert_eq!(data.connections.len(), 10, "all 10 concurrent updates must survive");
    for i in 0..10 {
        assert!(
            data.connections.iter().any(|c| c.bot_type == format!("bot-{i}")),
            "connection bot-{i} lost by a concurrent save"
        );
    }
}

#[test]
fn update_fails_closed_on_corrupted_main_file_without_running_f() {
    let dir = TestTempDir::new("bot-persistence-corrupt");
    let (main, legacy) = persistence_paths(&dir);
    let corrupted = b"{ not valid json !!!";
    std::fs::write(&main, corrupted).unwrap();

    let f_ran = AtomicBool::new(false);
    let result = update_bot_persistence_at(&main, &legacy, |data| {
        f_ran.store(true, Ordering::SeqCst);
        data.connections.push(test_bot("bot-x"));
    });

    assert!(matches!(result, Err(BotPersistenceError::Corrupted(_))));
    assert!(
        !f_ran.load(Ordering::SeqCst),
        "f must not run when the main file is corrupted"
    );
    assert_eq!(
        std::fs::read(&main).unwrap(),
        corrupted,
        "corrupted file bytes must be left untouched"
    );
}

#[test]
fn load_returns_default_with_warn_on_corrupted_file() {
    let dir = TestTempDir::new("bot-persistence-load-warn");
    let (main, legacy) = persistence_paths(&dir);
    std::fs::write(&main, b"{ not valid json !!!").unwrap();

    let warn_seen = Arc::new(AtomicBool::new(false));
    let subscriber = CapturingSubscriber {
        warn_seen: warn_seen.clone(),
    };
    tracing::subscriber::with_default(subscriber, || {
        let data = load_bot_persistence_at(&main, &legacy);
        assert_eq!(data.connections.len(), 0, "corrupted load must fall back to default");
        assert!(!data.verbose_mode);
    });

    assert!(warn_seen.load(Ordering::SeqCst), "corrupted load must emit a warn");
}

#[test]
fn second_write_keeps_previous_version_in_bak() {
    let dir = TestTempDir::new("bot-persistence-bak");
    let (main, legacy) = persistence_paths(&dir);

    update_bot_persistence_at(&main, &legacy, |data| data.connections.push(test_bot("first"))).unwrap();
    update_bot_persistence_at(&main, &legacy, |data| data.connections.push(test_bot("second"))).unwrap();

    let bak_path = main.with_extension("bak");
    assert!(bak_path.exists(), ".bak must exist after a second write");
    let previous: BotPersistenceData = serde_json::from_str(&std::fs::read_to_string(&bak_path).unwrap()).unwrap();
    assert!(previous.connections.iter().any(|c| c.bot_type == "first"));
    assert!(
        !previous.connections.iter().any(|c| c.bot_type == "second"),
        ".bak must hold the previous version, not the latest"
    );
}

#[test]
fn missing_main_file_falls_back_to_legacy_file() {
    let dir = TestTempDir::new("bot-persistence-legacy");
    let (main, legacy) = persistence_paths(&dir);
    let mut legacy_data = BotPersistenceData::default();
    legacy_data.connections.push(test_bot("legacy-bot"));
    std::fs::write(&legacy, serde_json::to_string_pretty(&legacy_data).unwrap()).unwrap();

    let data = try_load_bot_persistence_at(&main, &legacy).unwrap();
    assert_eq!(data.connections.len(), 1);
    assert_eq!(data.connections[0].bot_type, "legacy-bot");
}

#[test]
fn corrupted_legacy_file_is_fail_closed() {
    let dir = TestTempDir::new("bot-persistence-legacy-corrupt");
    let (main, legacy) = persistence_paths(&dir);
    std::fs::write(&legacy, b"{ not valid json !!!").unwrap();

    let result = try_load_bot_persistence_at(&main, &legacy);
    assert!(matches!(result, Err(BotPersistenceError::Parse { .. })));
}

#[test]
fn missing_both_files_is_empty_state() {
    let dir = TestTempDir::new("bot-persistence-empty");
    let (main, legacy) = persistence_paths(&dir);

    let data = try_load_bot_persistence_at(&main, &legacy).unwrap();
    assert_eq!(data.connections.len(), 0);
    assert!(!data.verbose_mode);
}

/// Minimal `tracing` subscriber that records whether a WARN event was
/// emitted during the wrapped call. Thread-local via
/// `tracing::subscriber::with_default`, so parallel tests are unaffected.
struct CapturingSubscriber {
    warn_seen: Arc<AtomicBool>,
}

impl tracing::subscriber::Subscriber for CapturingSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        if event.metadata().level() == &tracing::Level::WARN {
            self.warn_seen.store(true, Ordering::SeqCst);
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}
