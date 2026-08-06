use super::keyring::{KeyringBackend, API_KEY_SENTINEL, PRODUCTION_KEYRING};
use super::{AppSettings, ModelRef};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ===== Disk IO =====

/// Process-wide single-writer lock for the settings file (H-9). All
/// load → mutate → save cycles run under this lock so two concurrent
/// settings actions can never read the same stale snapshot and clobber
/// each other's fields. The public [`load_app_settings`] holds it for its
/// whole run too, because a load may trigger migration writes (dedup +
/// keyring migration); the lock-free `*_at` variants exist so the update
/// path can compose them inside the lock (tokio's Mutex is not reentrant —
/// re-acquiring it would deadlock). Async because the critical section
/// awaits [`load_app_settings_at`] / [`save_app_settings_at`].
static SETTINGS_WRITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Resolve `~/.northhing/config/app.json`. Uses the same path convention as
/// ConfigManager (`self.path_manager.config_dir().join("app.json")`); for
/// Phase 1 we resolve it directly via `dirs` to keep this file independent of
/// `northhing-core`'s PathManager.
pub fn app_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法获取 home 目录")?;
    Ok(home.join(".northhing").join("config").join("app.json"))
}

/// Load settings from `~/.northhing/config/app.json`. Returns `AppSettings::default()`
/// when the file is missing or fails to parse — the welcome screen's `is_first_run()`
/// check decides whether to show onboarding UI.
///
/// 2026-07-18 (D2c): after deserialization, dedup providers by
/// (name, provider_type, base_url, api_key, model) — keep the first, drop the
/// rest; re-point `default_model` at the kept entry when its original id was
/// dropped. Persist the migration immediately when anything was dropped.
///
/// 2026-08-05 (FU-3): holds [`SETTINGS_WRITE_LOCK`] for the whole run — a
/// load is not read-only, it may persist dedup / keyring migration writes,
/// and those must serialize with `update_app_settings` transactions instead
/// of racing them (the pre-fix unlocked migration save could clobber a
/// concurrent update's write).
pub async fn load_app_settings() -> Result<AppSettings> {
    let path = app_settings_path()?;
    load_app_settings_locked(&path, &*PRODUCTION_KEYRING).await
}

/// [`load_app_settings_at`] under [`SETTINGS_WRITE_LOCK`] — the composition
/// the public load uses. Kept separate from the public wrapper so tests can
/// inject a path/keyring while still exercising the real lock.
async fn load_app_settings_locked(path: &Path, keyring: &dyn KeyringBackend) -> Result<AppSettings> {
    let _guard = SETTINGS_WRITE_LOCK.lock().await;
    load_app_settings_at(path, keyring).await
}

/// Lock-free inner load. Both call sites hold [`SETTINGS_WRITE_LOCK`] around
/// it (the public load via [`load_app_settings_locked`],
/// [`update_app_settings_at`] inside its transaction) because a load may
/// trigger migration writes; it must never lock itself — tokio's Mutex is
/// not reentrant, so a second acquisition inside the update transaction
/// would deadlock.
async fn load_app_settings_at(path: &Path, keyring: &dyn KeyringBackend) -> Result<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("读取 {path:?} 失败"))?;
    let mut parsed: AppSettings =
        serde_json::from_str(&raw).with_context(|| format!("解析 {path:?} 失败（schema 可能不兼容）"))?;
    let dropped = dedup_providers_on_load(&mut parsed);
    if dropped > 0 {
        // 2026-07-18 (D2c): persist migration result immediately.
        if let Err(e) = save_app_settings_at(path, &parsed).await {
            tracing::warn!(target: "app_state", "load dedup save failed: {e}");
        }
    }
    // 2026-08-04 (C3, P1-2): migrate plaintext API keys to OS keyring.
    // Any keyring failure aborts the entire load (fail-closed) — no
    // plaintext key is allowed to stay on disk when the keyring is
    // unavailable.
    let migrated = keyring_migrate_providers(keyring, &mut parsed)?;
    if migrated > 0 {
        save_app_settings_at(path, &parsed).await?;
    }
    Ok(parsed)
}

/// Migrate plaintext API keys from `ProviderConfig.api_key` to the OS keyring.
///
/// For each provider with a non-empty, non-sentinel `api_key`:
/// 1. Store the real key in the keyring under `(KEYRING_SERVICE, provider.id)`.
/// 2. Replace the in-memory field with [`API_KEY_SENTINEL`].
///
/// ## Fail-closed
///
/// If any keyring `store` operation fails, the entire migration is aborted
/// and an `Err` is returned. The in-memory `parsed` is left unchanged so
/// the caller can decide whether to retry or propagate the error upward.
///
/// ## Returns
///
/// The number of providers migrated (0 means none were plaintext).
pub(super) fn keyring_migrate_providers(keyring: &dyn KeyringBackend, parsed: &mut AppSettings) -> Result<usize> {
    let mut count = 0usize;
    for provider in &mut parsed.providers {
        if provider.api_key.is_empty() || provider.api_key == API_KEY_SENTINEL {
            continue;
        }
        // Store the plaintext key in the OS keyring.
        let plaintext = std::mem::take(&mut provider.api_key);
        // If keyring store fails, put the key back and abort (fail-closed).
        match keyring.store(&provider.id, &plaintext) {
            Ok(()) => {
                provider.api_key = API_KEY_SENTINEL.to_string();
                count += 1;
            }
            Err(e) => {
                // Restore the plaintext key so the in-memory state is unchanged.
                provider.api_key = plaintext;
                return Err(e).context(format!(
                    "keyring: failed to migrate API key for provider '{}' ({}). \
                     The OS keychain may be unavailable — please configure a \
                     Secret Service provider on Linux, or check your keychain \
                     access on macOS/Windows",
                    provider.id, provider.name
                ));
            }
        }
    }
    if count > 0 {
        tracing::info!(
            target: "app_state",
            "keyring migration: moved {count} provider API key(s) to OS keyring"
        );
    }
    Ok(count)
}

/// Transactional settings update (H-9). Runs the whole load → `f` → atomic
/// save cycle under [`SETTINGS_WRITE_LOCK`], so concurrent settings actions
/// serialize instead of silently overwriting each other (the pre-fix
/// load-modify-write-back race).
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
    // 2026-08-04 (C3, P1-2): migrate any new plaintext keys added by `f`
    // before saving, so no plaintext key reaches disk even on the first save.
    let migrated = keyring_migrate_providers(keyring, &mut settings)?;
    if migrated > 0 {
        tracing::info!(
            target: "app_state",
            "keyring migration in update: moved {migrated} newly-added provider API key(s)"
        );
    }
    save_app_settings_at(path, &settings).await?;
    Ok(result)
}

/// 2026-07-18 (D2c): in-place provider dedup + default-model re-point.
/// Keeps the first of each (name, provider_type, base_url, api_key, model) group.
/// Returns the number of dropped duplicates (caller decides whether to save).
pub(super) fn dedup_providers_on_load(s: &mut AppSettings) -> usize {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, String, String, String, String)> = HashSet::new();
    let mut kept_ids: Vec<String> = Vec::new();
    let mut dropped_count = 0usize;
    s.providers.retain(|p| {
        let key = (
            p.name.clone(),
            serde_json::to_string(&p.provider_type).unwrap_or_default(),
            p.base_url.clone(),
            p.api_key.clone(),
            p.model.clone(),
        );
        if seen.insert(key) {
            kept_ids.push(p.id.clone());
            true
        } else {
            dropped_count += 1;
            false
        }
    });
    if dropped_count > 0 {
        let kept_set: HashSet<&str> = kept_ids.iter().map(|x| x.as_str()).collect();
        if let Some(dm) = &s.default_model {
            if !kept_set.contains(dm.provider_id.as_str()) {
                // default_model pointed at a dropped entry → re-point at the
                // first kept provider so the reference stays valid.
                if let Some(first) = s.providers.first() {
                    s.default_model = Some(ModelRef {
                        provider_id: first.id.clone(),
                        model: first.model.clone(),
                    });
                } else {
                    s.default_model = None;
                }
            }
        }
        tracing::info!(
            target: "app_state",
            "load dedup: dropped {dropped_count} duplicate provider(s)"
        );
    }
    dropped_count
}

/// Save settings to `path`. Creates parent dirs as needed. Atomic write:
/// serialize to a `.<name>.<pid>.<nonce>.tmp` sibling in the same directory,
/// flush, then rename over the target (same-directory rename is atomic, so a
/// reader never observes a truncated file). The previous content is copied to
/// `<name>.bak` first; a failed backup is warn-only and never blocks the
/// write.
///
/// 2026-07-31 (H-9): replaced the previous plain `tokio::fs::write` to the
/// target, which could leave a truncated JSON file on crash. Kept as the
/// low-level API so the load-time dedup migration and other callers can
/// still write directly.
///
/// 2026-08-05 (FU-4): the public `save_app_settings` wrapper became dead
/// code once H-9 funneled all writes through `update_app_settings` and was
/// deleted; this worker remains the only settings writer.
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

    // Write + flush the tmp file before rename so the published file is never
    // a partial write (the pre-fix `tokio::fs::write` to the target could
    // leave truncated JSON on crash). The handle drops at the end of this
    // block, releasing the file before the rename below.
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
            // Windows: external scanners/indexers may briefly hold a
            // non-shareable handle on the target, making rename fail with
            // PermissionDenied. Retry once after removing the target — same
            // fallback as `json_store.rs` `replace_file_from_temp`.
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
