// 2026-07-31 (H-9): disk IO regression tests for the settings single-writer
// transaction + atomic write. Tests use the private `*_at(path)` variants so
// no test touches the real `~/.northhing/config/app.json` (same path
// injection scheme as Task 5 `remote_connect/bot/persistence_tests.rs`).
//
// Note: this file lives under `settings/io/` because `io.rs` declares
// `mod io_tests;` as a child module (rustc resolves it to `io/io_tests.rs`).

use super::*;
use crate::app_state::settings::{ProviderConfig, ProviderType};
use northhing_test_support::TestTempDir;

fn provider_with_fields(id: &str, name: &str, base_url: &str, api_key: &str, model: &str) -> ProviderConfig {
    ProviderConfig {
        id: id.to_string(),
        name: name.to_string(),
        provider_type: if base_url.contains("anthropic") {
            ProviderType::Anthropic
        } else {
            ProviderType::Openai
        },
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        model: model.to_string(),
        enabled: true,
        created_at: 0,
        last_verified_at: None,
        last_verified_ok: None,
    }
}

// ===== H-9: concurrent transactions must not lose updates =====

/// 10 concurrent `update_app_settings_at` calls, each upserting a different
/// provider. Without the single-writer lock each transaction would load the
/// same stale snapshot and the last writer would silently wipe the other 9;
/// with the lock the final file must contain all 10.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_updates_preserve_all_writes() {
    let dir = TestTempDir::new("settings-io-concurrent");
    let path = dir.path().join("app.json");

    let mut tasks = Vec::new();
    for i in 0..10 {
        let path = path.clone();
        tasks.push(tokio::spawn(async move {
            update_app_settings_at(&path, |s| {
                let id = format!("p{i}");
                s.upsert_provider(provider_with_fields(&id, &format!("provider-{i}"), &format!("https://x{i}.com/v1"), &format!("sk-{i}"), &format!("model-{i}")));
                Ok(())
            })
            .await
        }));
    }
    for task in tasks {
        task.await.expect("concurrent update task panicked").expect("concurrent update failed");
    }

    let final_settings = load_app_settings_at(&path).await.expect("final load");
    assert_eq!(final_settings.providers.len(), 10, "no update may be lost");
    for i in 0..10 {
        let id = format!("p{i}");
        assert!(
            final_settings.providers.iter().any(|p| p.id == id),
            "provider {i} must survive the concurrent writes"
        );
    }
}

// ===== H-9: f returning Err must not write =====

/// A failing closure aborts the transaction: the on-disk bytes must be
/// exactly the previous version (no partial write, no blanking).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_with_err_closure_does_not_write_file() {
    let dir = TestTempDir::new("settings-io-err");
    let path = dir.path().join("app.json");

    let mut initial = AppSettings::default();
    initial.onboarding_completed = true;
    save_app_settings_at(&path, &initial).await.expect("seed write");
    let before = tokio::fs::read(&path).await.expect("read before");

    let result: anyhow::Result<()> = update_app_settings_at(&path, |_s| Err(anyhow::anyhow!("boom"))).await;
    assert!(result.is_err(), "closure error must propagate");

    let after = tokio::fs::read(&path).await.expect("read after");
    assert_eq!(before, after, "failed transaction must not touch the file");
}

// ===== H-9: atomic write =====

/// A leftover tmp file (simulated crash residue) must never affect the main
/// file: subsequent save/load cycles ignore it and the main file stays valid.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn leftover_tmp_file_does_not_break_main_file() {
    let dir = TestTempDir::new("settings-io-crash");
    let path = dir.path().join("app.json");

    // Simulate a crash that left a tmp sibling behind.
    let tmp_residue = dir.path().join(format!(".app.json.{}.0.tmp", std::process::id()));
    tokio::fs::write(&tmp_residue, b"partial garbage").await.expect("stray tmp");

    let mut settings = AppSettings::default();
    settings.add_workspace(PathBuf::from("/tmp/proj"));
    save_app_settings_at(&path, &settings).await.expect("save with residue present");

    let loaded = load_app_settings_at(&path).await.expect("load with residue present");
    assert_eq!(loaded.workspaces.len(), 1, "main file must be intact");

    let raw = tokio::fs::read_to_string(&path).await.expect("read main");
    serde_json::from_str::<AppSettings>(&raw).expect("main file must be valid JSON");
}

/// After two writes, `<name>.bak` holds the previous version (v1), the main
/// file holds v2, and no tmp file residue remains.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_write_keeps_previous_version_in_bak() {
    let dir = TestTempDir::new("settings-io-bak");
    let path = dir.path().join("app.json");

    let mut v1 = AppSettings::default();
    v1.onboarding_completed = true;
    save_app_settings_at(&path, &v1).await.expect("write v1");

    let mut v2 = AppSettings::default();
    v2.add_workspace(PathBuf::from("/tmp/proj"));
    save_app_settings_at(&path, &v2).await.expect("write v2");

    let bak_raw = tokio::fs::read_to_string(path.with_extension("bak"))
        .await
        .expect("bak must exist after second write");
    let bak: AppSettings = serde_json::from_str(&bak_raw).expect("bak must be valid JSON");
    assert!(bak.onboarding_completed, "bak must be the previous version (v1)");
    assert!(bak.workspaces.is_empty(), "bak must not contain v2 data");

    let main: AppSettings =
        serde_json::from_str(&tokio::fs::read_to_string(&path).await.expect("read main")).expect("main JSON");
    assert_eq!(main.workspaces.len(), 1, "main must hold v2");

    // Atomic write must clean up after itself: no tmp sibling may remain.
    let mut entries = tokio::fs::read_dir(dir.path()).await.expect("list dir");
    while let Some(entry) = entries.next_entry().await.expect("next entry") {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.ends_with(".tmp"), "tmp file {name} must not remain after save");
    }
}

// ===== load-time dedup migration still works through the atomic writer =====

/// The D2c dedup-on-load migration persists its result immediately; after
/// H-9 that write goes through the atomic path (tmp + rename + bak) and the
/// on-disk file must end up deduplicated.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_dedup_migration_still_persists() {
    let dir = TestTempDir::new("settings-io-dedup");
    let path = dir.path().join("app.json");

    // Two identical providers (same name/type/base_url/api_key/model), only
    // the ids differ → dedup must drop the second and persist the result.
    let mut seeded = AppSettings::default();
    seeded.providers = vec![
        provider_with_fields("id-a", "foo", "https://x.com/v1", "sk", "gpt"),
        provider_with_fields("id-b", "foo", "https://x.com/v1", "sk", "gpt"),
    ];
    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    let loaded = load_app_settings_at(&path).await.expect("load");
    assert_eq!(loaded.providers.len(), 1, "in-memory dedup must drop the duplicate");

    let on_disk: AppSettings =
        serde_json::from_str(&tokio::fs::read_to_string(&path).await.expect("read main")).expect("main JSON");
    assert_eq!(on_disk.providers.len(), 1, "dedup migration must be persisted to disk");
    assert_eq!(on_disk.providers[0].id, "id-a", "first of group kept");
}

// ===== fail-closed: unparseable file must propagate Err =====

/// A corrupt app.json must surface as `Err` (fail-closed) instead of being
/// silently replaced with defaults: a stale/truncated file should never make
/// the UI believe the user has no configuration.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_parse_failure_returns_err() {
    let dir = TestTempDir::new("settings-io-parse");
    let path = dir.path().join("app.json");

    tokio::fs::write(&path, b"{ \"schema_version\": ").await.expect("corrupt seed write");

    let result = load_app_settings_at(&path).await;
    assert!(result.is_err(), "corrupt JSON must propagate Err (fail-closed)");
}
