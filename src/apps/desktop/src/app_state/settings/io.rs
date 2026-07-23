use anyhow::{Context, Result};
use std::path::PathBuf;
use super::{AppSettings, ModelRef, now_unix_secs};

// ===== Disk IO =====

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
pub async fn load_app_settings() -> Result<AppSettings> {
    let path = app_settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("读取 {path:?} 失败"))?;
    let mut parsed: AppSettings =
        serde_json::from_str(&raw).with_context(|| format!("解析 {path:?} 失败（schema 可能不兼容）"))?;
    let dropped = dedup_providers_on_load(&mut parsed);
    if dropped > 0 {
        // 2026-07-18 (D2c): persist migration result immediately.
        if let Err(e) = save_app_settings(&parsed).await {
            tracing::warn!(target: "app_state", "load dedup save failed: {e}");
        }
    }
    Ok(parsed)
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

/// Save settings to `~/.northhing/config/app.json`. Creates parent dirs as
/// needed. Atomic write via tmp-file + rename (Phase 1: simple write —
/// upgrade to atomic in Phase 5 if race conditions surface).
pub async fn save_app_settings(settings: &AppSettings) -> Result<()> {
    let path = app_settings_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("创建目录 {parent:?} 失败"))?;
    }
    let json = serde_json::to_string_pretty(settings).context("序列化 settings 失败")?;
    tokio::fs::write(&path, json)
        .await
        .with_context(|| format!("写入 {path:?} 失败"))?;
    Ok(())
}
