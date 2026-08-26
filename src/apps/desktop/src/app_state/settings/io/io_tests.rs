// 2026-07-31 (H-9): disk IO regression tests for the settings single-writer
// transaction + atomic write. Tests use the private `*_at(path)` variants so
// no test touches the real `~/.northhing/config/app.json` (path injection scheme).
//
// 2026-08-26 (P1c, P1-8): tests use a MockKeyring so they do not depend
// on the real OS keyring. The `_at` variants accept a `&dyn KeyringBackend`.

use super::*;
use crate::app_state::settings::keyring::{
    make_env_sentinel, MockKeyring, MCP_ENV_SENTINEL,
};
use crate::app_state::settings::{MCPServerConfig, MCPTransport};
use northhing_test_support::TestTempDir;
use std::collections::HashMap;

fn mcp_server_with_env(id: &str, name: &str, env: HashMap<String, String>) -> MCPServerConfig {
    MCPServerConfig {
        id: id.to_string(),
        name: name.to_string(),
        transport: MCPTransport::Stdio,
        enabled: true,
        command: Some("node".to_string()),
        args: vec!["server.js".to_string()],
        url: None,
        env,
        last_verified_at: None,
        last_verified_ok: None,
        last_tools: Vec::new(),
    }
}

// ===== H-9: concurrent transactions must not lose updates =====

/// 10 concurrent `update_app_settings_at` calls, each adding a different
/// workspace. Without the single-writer lock each transaction would load the
/// same stale snapshot and the last writer would silently wipe the other 9;
/// with the lock the final file must contain all 10.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_updates_preserve_all_writes() {
    let kr = std::sync::Arc::new(MockKeyring::new());
    let dir = TestTempDir::new("settings-io-concurrent");
    let path = dir.path().join("app.json");

    let mut tasks = Vec::new();
    for i in 0..10 {
        let path = path.clone();
        let kr_task = kr.clone();
        tasks.push(tokio::spawn(async move {
            update_app_settings_at(&path, kr_task.as_ref(), |s| {
                s.add_workspace(PathBuf::from(format!("/tmp/proj_{i}")));
                Ok(())
            })
            .await
        }));
    }
    for task in tasks {
        task.await
            .expect("concurrent update task panicked")
            .expect("concurrent update failed");
    }

    let final_settings = load_app_settings_at(&path, kr.as_ref()).await.expect("final load");
    assert_eq!(final_settings.workspaces.len(), 10, "no update may be lost");
    for i in 0..10 {
        let expected_path = PathBuf::from(format!("/tmp/proj_{i}"));
        assert!(
            final_settings.workspaces.iter().any(|w| w.path == expected_path),
            "workspace {i} must survive the concurrent writes"
        );
    }
}

// ===== H-9: f returning Err must not write =====

/// A failing closure aborts the transaction: the on-disk bytes must be
/// exactly the previous version (no partial write, no blanking).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_with_err_closure_does_not_write_file() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-err");
    let path = dir.path().join("app.json");

    let mut initial = AppSettings::default();
    initial.onboarding_completed = true;
    save_app_settings_at(&path, &initial).await.expect("seed write");
    let before = tokio::fs::read(&path).await.expect("read before");

    let result: anyhow::Result<()> = update_app_settings_at(&path, &kr, |_s| Err(anyhow::anyhow!("boom"))).await;
    assert!(result.is_err(), "closure error must propagate");

    let after = tokio::fs::read(&path).await.expect("read after");
    assert_eq!(before, after, "failed transaction must not touch the file");
}

// ===== H-9: atomic write =====

/// A leftover tmp file (simulated crash residue) must never affect the main
/// file: subsequent save/load cycles ignore it and the main file stays valid.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn leftover_tmp_file_does_not_break_main_file() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-crash");
    let path = dir.path().join("app.json");

    // Simulate a crash that left a tmp sibling behind.
    let tmp_residue = dir.path().join(format!(".app.json.{}.0.tmp", std::process::id()));
    tokio::fs::write(&tmp_residue, b"partial garbage")
        .await
        .expect("stray tmp");

    let mut settings = AppSettings::default();
    settings.add_workspace(PathBuf::from("/tmp/proj"));
    save_app_settings_at(&path, &settings)
        .await
        .expect("save with residue present");

    let loaded = load_app_settings_at(&path, &kr).await.expect("load with residue present");
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

// ===== fail-closed: unparseable file must propagate Err =====

/// A corrupt app.json must surface as `Err` (fail-closed) instead of being
/// silently replaced with defaults: a stale/truncated file should never make
/// the UI believe the user has no configuration.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_parse_failure_returns_err() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-parse");
    let path = dir.path().join("app.json");

    tokio::fs::write(&path, b"{ \"schema_version\": ")
        .await
        .expect("corrupt seed write");

    let result = load_app_settings_at(&path, &kr).await;
    assert!(result.is_err(), "corrupt JSON must propagate Err (fail-closed)");
}

// ===== FU-3: the public load path runs under the write lock =====

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_loads_and_updates_preserve_all_writes() {
    let dir = TestTempDir::new("settings-io-fu3-race");
    let path = dir.path().join("app.json");

    let mut seeded = AppSettings::default();
    seeded.add_workspace(PathBuf::from("/tmp/seeded"));
    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    let mut tasks = Vec::new();
    for i in 0..8 {
        let load_path = path.clone();
        tasks.push(tokio::spawn(async move {
            load_app_settings_locked(&load_path, &*PRODUCTION_KEYRING)
                .await
                .expect("concurrent load");
        }));
        let update_path = path.clone();
        tasks.push(tokio::spawn(async move {
            update_app_settings_at(&update_path, &*PRODUCTION_KEYRING, |s| {
                s.add_workspace(PathBuf::from(format!("/tmp/u_{i}")));
                Ok(())
            })
            .await
            .expect("concurrent update");
        }));
    }
    let joined = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        for task in tasks {
            task.await.expect("concurrent task panicked");
        }
    })
    .await;
    assert!(joined.is_ok(), "load/update composition deadlocked");

    let final_settings = load_app_settings_at(&path, &*PRODUCTION_KEYRING)
        .await
        .expect("final load");
    assert_eq!(
        final_settings.workspaces.len(),
        1 + 8,
        "seeded workspace plus 8 updates must all survive"
    );
}

// ===== P1c, P1-8: MCP env keyring migration and integration tests =====

/// Plaintext MCP envs on disk are migrated to OS keyring on load.
/// In memory, the real env is restored; on disk, the env field becomes the sentinel.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_keyring_migration_plaintext_to_sentinel_on_load() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-mcp-env-migrate");
    let path = dir.path().join("app.json");

    let mut seeded = AppSettings::default();
    let mut env = HashMap::new();
    env.insert("API_KEY".to_string(), "sk-real-mcp-key-12345".to_string());
    env.insert("DEBUG".to_string(), "true".to_string());
    seeded.upsert_mcp(mcp_server_with_env("server-1", "My MCP Server", env.clone()));

    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    let loaded = load_app_settings_at(&path, &kr).await.expect("load with keyring");
    assert_eq!(loaded.mcp_servers.len(), 1);
    assert_eq!(
        loaded.mcp_servers[0].env.get("API_KEY").map(String::as_str),
        Some("sk-real-mcp-key-12345"),
        "in-memory env must have the restored plaintext value"
    );
    assert_eq!(
        loaded.mcp_servers[0].env.get("DEBUG").map(String::as_str),
        Some("true")
    );

    // Keyring must hold the serialized JSON.
    let raw_in_kr = kr.get("mcp-env:server-1").expect("keyring entry must exist");
    let parsed_kr: HashMap<String, String> =
        serde_json::from_str(&raw_in_kr).expect("keyring value must be valid JSON map");
    assert_eq!(parsed_kr, env);

    // On-disk file must contain the sentinel and NOT the plaintext secret.
    let on_disk = tokio::fs::read_to_string(&path).await.expect("read disk file");
    assert!(
        !on_disk.contains("sk-real-mcp-key-12345"),
        "plaintext secret must NOT remain in the on-disk file"
    );
    assert!(
        on_disk.contains(MCP_ENV_SENTINEL),
        "on-disk file must contain the MCP_ENV_SENTINEL"
    );
}

/// When on-disk file has sentinel, load restores the real env from keyring.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_keyring_sentinel_loaded_and_restored() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-mcp-env-restore");
    let path = dir.path().join("app.json");

    // Seed keyring with real secrets.
    let mut real_env = HashMap::new();
    real_env.insert("SECRET_TOKEN".to_string(), "super-secret-token".to_string());
    kr.store("mcp-env:srv-preloaded", &serde_json::to_string(&real_env).unwrap())
        .unwrap();

    // Seed disk file with sentinel.
    let mut seeded = AppSettings::default();
    seeded.upsert_mcp(mcp_server_with_env(
        "srv-preloaded",
        "Preloaded Server",
        make_env_sentinel(),
    ));
    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    let loaded = load_app_settings_at(&path, &kr).await.expect("load");
    assert_eq!(loaded.mcp_servers.len(), 1);
    assert_eq!(
        loaded.mcp_servers[0].env.get("SECRET_TOKEN").map(String::as_str),
        Some("super-secret-token"),
        "loaded env must be restored from keyring"
    );
    assert!(
        !loaded.mcp_servers[0].env.contains_key(MCP_ENV_SENTINEL),
        "in-memory env must not contain sentinel key"
    );
}

/// Updating settings with a new MCP server env stores the env in keyring and writes sentinel to disk.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_update_app_settings_stores_new_env_in_keyring() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-mcp-env-update");
    let path = dir.path().join("app.json");

    update_app_settings_at(&path, &kr, |s| {
        let mut env = HashMap::new();
        env.insert("DATABASE_URL".to_string(), "postgres://user:pass@localhost/db".to_string());
        s.upsert_mcp(mcp_server_with_env("pg-mcp", "Postgres MCP", env));
        Ok(())
    })
    .await
    .expect("update");

    // Keyring contains the env JSON.
    let kr_val = kr.get("mcp-env:pg-mcp").expect("keyring store");
    assert!(kr_val.contains("postgres://user:pass@localhost/db"));

    // Disk file contains sentinel, not plaintext.
    let on_disk = tokio::fs::read_to_string(&path).await.expect("read disk");
    assert!(!on_disk.contains("postgres://user:pass@localhost/db"));
    assert!(on_disk.contains(MCP_ENV_SENTINEL));

    // Subsequent load restores the env correctly.
    let loaded = load_app_settings_at(&path, &kr).await.expect("load");
    assert_eq!(
        loaded.mcp_servers[0].env.get("DATABASE_URL").map(String::as_str),
        Some("postgres://user:pass@localhost/db")
    );
}

/// Missing keyring entry on load results in empty env map (fail-open) and does not panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_fail_open_missing_entry_returns_empty_map() {
    let kr = MockKeyring::new(); // empty keyring
    let dir = TestTempDir::new("settings-io-mcp-env-fail-open");
    let path = dir.path().join("app.json");

    let mut seeded = AppSettings::default();
    seeded.upsert_mcp(mcp_server_with_env("srv-missing", "Missing", make_env_sentinel()));
    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    let loaded = load_app_settings_at(&path, &kr).await.expect("load must succeed (fail-open)");
    assert_eq!(loaded.mcp_servers.len(), 1);
    assert!(
        loaded.mcp_servers[0].env.is_empty(),
        "missing keyring entry must yield empty env map"
    );
}

/// Loading an already-sentinel file is idempotent and does not overwrite keyring with sentinel map.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring() {
    let kr = MockKeyring::new();
    let dir = TestTempDir::new("settings-io-mcp-env-idempotent");
    let path = dir.path().join("app.json");

    let mut real_env = HashMap::new();
    real_env.insert("KEY".to_string(), "ORIGINAL".to_string());
    kr.store("mcp-env:srv-idem", &serde_json::to_string(&real_env).unwrap())
        .unwrap();

    let mut seeded = AppSettings::default();
    seeded.upsert_mcp(mcp_server_with_env("srv-idem", "Idem", make_env_sentinel()));
    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");

    // First load
    let loaded1 = load_app_settings_at(&path, &kr).await.expect("first load");
    assert_eq!(loaded1.mcp_servers[0].env, real_env);

    // Second load
    let loaded2 = load_app_settings_at(&path, &kr).await.expect("second load");
    assert_eq!(loaded2.mcp_servers[0].env, real_env);

    // Keyring value remains original
    let kr_raw = kr.get("mcp-env:srv-idem").unwrap();
    let parsed: HashMap<String, String> = serde_json::from_str(&kr_raw).unwrap();
    assert_eq!(parsed, real_env);
}

/// When keyring store fails during load-time migration (fail-closed), the load returns Err
/// and the disk file remains unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mcp_env_fail_closed_on_store_error_does_not_corrupt_disk() {
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug)]
    struct FailingKeyring(AtomicBool);

    impl KeyringBackend for FailingKeyring {
        fn store(&self, _account: &str, _secret: &str) -> anyhow::Result<()> {
            self.0.store(true, Ordering::SeqCst);
            Err(anyhow::anyhow!("keyring store failed: mock failure"))
        }
        fn get(&self, _account: &str) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("keyring get failed"))
        }
        fn delete(&self, _account: &str) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("keyring delete failed"))
        }
    }

    let failing_kr = FailingKeyring(AtomicBool::new(false));
    let dir = TestTempDir::new("settings-io-mcp-fail-closed");
    let path = dir.path().join("app.json");

    let mut seeded = AppSettings::default();
    let mut env = HashMap::new();
    env.insert("SECRET".to_string(), "plain_secret_value".to_string());
    seeded.upsert_mcp(mcp_server_with_env("srv-fail", "Fail Server", env));

    tokio::fs::write(&path, serde_json::to_string_pretty(&seeded).expect("seed serialize"))
        .await
        .expect("seed write");
    let before_bytes = tokio::fs::read(&path).await.expect("read before");

    let result = load_app_settings_at(&path, &failing_kr).await;
    assert!(result.is_err(), "migration failure must return Err (fail-closed)");
    assert!(failing_kr.0.load(Ordering::SeqCst), "store must have been attempted");

    let after_bytes = tokio::fs::read(&path).await.expect("read after");
    assert_eq!(before_bytes, after_bytes, "disk file must remain unchanged on fail-closed");
}
