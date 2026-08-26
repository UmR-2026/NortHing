use super::keyring::{
    is_env_sentinel, load_env, make_env_sentinel, store_env, KeyringBackend, PRODUCTION_KEYRING,
};
use super::AppSettings;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ===== Disk IO =====

/// Process-wide single-writer lock for the settings file (H-9). All
/// load → mutate → save cycles run under this lock so two concurrent
/// settings actions can never read the same stale snapshot and clobber
/// each other's fields. Async because the critical section
/// awaits [`load_app_settings_at`] / [`save_app_settings_at`].
static SETTINGS_WRITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Resolve `~/.northhing/config/app.json`.
pub fn app_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法获取 home 目录")?;
    Ok(home.join(".northhing").join("config").join("app.json"))
}

/// Load settings from `~/.northhing/config/app.json`. Returns `AppSettings::default()`
/// when the file is missing or fails to parse — the welcome screen's `is_first_run()`
/// check decides whether to show onboarding UI.
pub async fn load_app_settings() -> Result<AppSettings> {
    let path = app_settings_path()?;
    load_app_settings_locked(&path, &*PRODUCTION_KEYRING).await
}

/// [`load_app_settings_at`] under [`SETTINGS_WRITE_LOCK`].
async fn load_app_settings_locked(path: &Path, keyring: &dyn KeyringBackend) -> Result<AppSettings> {
    let _guard = SETTINGS_WRITE_LOCK.lock().await;
    load_app_settings_at(path, keyring).await
}

/// Lock-free inner load.
async fn load_app_settings_at(path: &Path, keyring: &dyn KeyringBackend) -> Result<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("读取 {path:?} 失败"))?;
    let mut parsed: AppSettings =
        serde_json::from_str(&raw).with_context(|| format!("解析 {path:?} 失败（schema 可能不兼容）"))?;

    // 2026-08-26 (P1c, P1-8): migrate plaintext MCP envs to OS keyring,
    // and restore keyring-backed envs into in-memory MCP server configs.
    let migrated = keyring_migrate_mcp_servers(keyring, &mut parsed)?;
    if migrated > 0 {
        let mut disk_settings = parsed.clone();
        prepare_settings_for_save(keyring, &mut disk_settings)?;
        save_app_settings_at(path, &disk_settings).await?;
    }
    Ok(parsed)
}

/// Migrate plaintext MCP server environment variables to OS keyring on load,
/// and restore keyring sentinels to real env maps in memory.
///
/// Returns the number of plaintext envs migrated to keyring.
pub(super) fn keyring_migrate_mcp_servers(
    keyring: &dyn KeyringBackend,
    parsed: &mut AppSettings,
) -> Result<usize> {
    let mut count = 0usize;
    for server in &mut parsed.mcp_servers {
        if is_env_sentinel(&server.env) {
            server.env = load_env(keyring, &server.id)?;
        } else if !server.env.is_empty() {
            // Legacy plaintext env on disk -> store in keyring
            let plaintext = server.env.clone();
            store_env(keyring, &server.id, &plaintext)?;
            count += 1;
        }
    }
    if count > 0 {
        tracing::info!(
            target: "app_state",
            "keyring migration: moved {count} MCP server env(s) to OS keyring"
        );
    }
    Ok(count)
}

/// Prepare settings for on-disk serialization by storing plaintext MCP envs
/// into the OS keyring and replacing them with sentinels in `settings`.
pub(super) fn prepare_settings_for_save(
    keyring: &dyn KeyringBackend,
    settings: &mut AppSettings,
) -> Result<usize> {
    let mut count = 0usize;
    for server in &mut settings.mcp_servers {
        if server.env.is_empty() || is_env_sentinel(&server.env) {
            continue;
        }
        let plaintext_env = std::mem::take(&mut server.env);
        match store_env(keyring, &server.id, &plaintext_env) {
            Ok(_) => {
                server.env = make_env_sentinel();
                count += 1;
            }
            Err(e) => {
                server.env = plaintext_env;
                return Err(e).context(format!(
                    "keyring: failed to store MCP env for server '{}' ({})",
                    server.id, server.name
                ));
            }
        }
    }
    Ok(count)
}

/// Transactional settings update (H-9). Runs the whole load → `f` → atomic
/// save cycle under [`SETTINGS_WRITE_LOCK`], so concurrent settings actions
/// serialize instead of silently overwriting each other.
///
/// `f` is synchronous by design (no async closure): it mutates the loaded
/// [`AppSettings`] and returns the value the caller wants. Returning `Err`
/// aborts the transaction without touching the file.
pub async fn update_app_settings<T>(f: impl FnOnce(&mut AppSettings) -> Result<T>) -> Result<T> {
    let path = app_settings_path()?;
    update_app_settings_at(&path, &*PRODUCTION_KEYRING, f).await
}

async fn update_app_settings_at<T>(
    path: &Path,
    keyring: &dyn KeyringBackend,
    f: impl FnOnce(&mut AppSettings) -> Result<T>,
) -> Result<T> {
    let _guard = SETTINGS_WRITE_LOCK.lock().await;
    let mut settings = load_app_settings_at(path, keyring).await?;
    let result = f(&mut settings)?;

    let mut disk_settings = settings.clone();
    let migrated = prepare_settings_for_save(keyring, &mut disk_settings)?;
    if migrated > 0 {
        tracing::info!(
            target: "app_state",
            "keyring migration in update: moved {migrated} newly-added MCP server env(s)"
        );
    }
    save_app_settings_at(path, &disk_settings).await?;
    Ok(result)
}

/// Save settings to `path`. Creates parent dirs as needed. Atomic write:
/// serialize to a `.<name>.<pid>.<nonce>.tmp` sibling in the same directory,
/// flush, then rename over the target.
async fn save_app_settings_at(path: &Path, settings: &AppSettings) -> Result<()> {
    let parent = path.parent().context("app.json 路径缺少父目录")?;
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("创建目录 {parent:?} 失败"))?;
    let json = serde_json::to_string_pretty(settings).context("序列化 settings 失败")?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "app.json".to_string());
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = parent.join(format!(".{file_name}.{}.{nonce}.tmp", std::process::id()));

    if path.exists() {
        if let Err(error) = tokio::fs::copy(path, path.with_extension("bak")).await {
            tracing::warn!("Failed to back up app settings {}: {error}", path.display());
        }
    }

    {
        use tokio::io::AsyncWriteExt;
        let mut file = match tokio::fs::File::create(&tmp_path).await {
            Ok(file) => file,
            Err(source) => {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err(source).with_context(|| format!("写入 {tmp_path:?} 失败"));
            }
        };
        if let Err(source) = file.write_all(json.as_bytes()).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(source).with_context(|| format!("写入 {tmp_path:?} 失败"));
        }
        if let Err(source) = file.flush().await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(source).with_context(|| format!("写入 {tmp_path:?} 失败"));
        }
    }

    match tokio::fs::rename(&tmp_path, path).await {
        Ok(()) => Ok(()),
        Err(_first_error) => {
            if path.exists() {
                match tokio::fs::remove_file(path).await {
                    Ok(()) => {}
                    Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                    Err(source) => {
                        let _ = tokio::fs::remove_file(&tmp_path).await;
                        return Err(source).with_context(|| format!("写入 {path:?} 失败"));
                    }
                }
            }
            match tokio::fs::rename(&tmp_path, path).await {
                Ok(()) => Ok(()),
                Err(source) => {
                    let _ = tokio::fs::remove_file(&tmp_path).await;
                    Err(source).with_context(|| format!("写入 {path:?} 失败"))
                }
            }
        }
    }
}

#[cfg(test)]
mod io_tests;
