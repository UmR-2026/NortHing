//! MiniApp storage draft methods (split from storage.rs in R37d).
//!
//! Owns draft CRUD, draft storage (KV), and the stale-draft quarantine
//! machinery (mark-for-cleanup / cleanup / safe-path checks).

use super::storage::{MiniApp, MiniAppMeta, MiniAppStorageError, MiniAppStorageResult};
use northhing_product_domains::miniapp::storage::{
    COMPILED_HTML, DRAFTS_CLEANUP_PREFIX, DRAFTS_DIR, DRAFT_JSON, META_JSON, STORAGE_JSON,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

impl super::storage::MiniAppStorage {
    pub async fn save_draft(
        &self,
        app_id: &str,
        draft_id: &str,
        app: &MiniApp,
        manifest: &serde_json::Value,
    ) -> MiniAppStorageResult<()> {
        self.ensure_active_drafts_root_writable().await?;
        let draft_dir = self.draft_dir(app_id, draft_id);
        let source_dir = self.draft_source_dir(app_id, draft_id);
        self.save_app_files(&draft_dir, &source_dir, app).await?;
        let manifest_json = serde_json::to_string_pretty(manifest).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(draft_dir.join(DRAFT_JSON), manifest_json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write draft.json: {}", e)))?;
        let storage_path = draft_dir.join(STORAGE_JSON);
        if !storage_path.exists() {
            tokio::fs::write(storage_path, "{}")
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to write draft storage: {}", e)))?;
        }
        Ok(())
    }
    pub async fn load_draft_app(&self, app_id: &str, draft_id: &str) -> MiniAppStorageResult<MiniApp> {
        self.ensure_active_drafts_root_readable(app_id, draft_id)?;
        let draft_dir = self.draft_dir(app_id, draft_id);
        let meta_content = tokio::fs::read_to_string(draft_dir.join(META_JSON))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Self::draft_not_found(app_id, draft_id)
                } else {
                    MiniAppStorageError::io(format!("Failed to read draft meta: {}", e))
                }
            })?;
        let meta: MiniAppMeta = serde_json::from_str(&meta_content)
            .map_err(|e| MiniAppStorageError::parse(format!("Invalid draft meta.json: {}", e)))?;
        let source = self
            .load_source_from_dirs(self.draft_source_dir(app_id, draft_id), draft_dir.clone())
            .await?;
        let compiled_html = tokio::fs::read_to_string(draft_dir.join(COMPILED_HTML))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MiniAppStorageError::not_found(format!(
                        "MiniApp draft compiled HTML not found: {}/{}",
                        app_id, draft_id
                    ))
                } else {
                    MiniAppStorageError::io(format!("Failed to read draft compiled.html: {}", e))
                }
            })?;
        Ok(MiniApp {
            id: meta.id,
            name: meta.name,
            description: meta.description,
            icon: meta.icon,
            category: meta.category,
            tags: meta.tags,
            version: meta.version,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            source,
            compiled_html,
            permissions: meta.permissions,
            ai_context: meta.ai_context,
            runtime: meta.runtime,
            i18n: meta.i18n,
        })
    }
    pub async fn load_draft_manifest(&self, app_id: &str, draft_id: &str) -> MiniAppStorageResult<serde_json::Value> {
        self.ensure_active_drafts_root_readable(app_id, draft_id)?;
        let path = self.draft_dir(app_id, draft_id).join(DRAFT_JSON);
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Self::draft_not_found(app_id, draft_id)
            } else {
                MiniAppStorageError::io(format!("Failed to read draft.json: {}", e))
            }
        })?;
        serde_json::from_str(&content).map_err(|e| MiniAppStorageError::parse(format!("Invalid draft.json: {}", e)))
    }
    pub async fn delete_draft(&self, app_id: &str, draft_id: &str) -> MiniAppStorageResult<()> {
        let dir = self.draft_dir(app_id, draft_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir)
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to delete miniapp draft: {}", e)))?;
        }
        Ok(())
    }
    pub async fn mark_stale_drafts_for_cleanup(&self) -> MiniAppStorageResult<Vec<PathBuf>> {
        let mut targets = self.collect_marked_drafts_roots().await?;
        if let Some(target) = self.isolate_active_drafts_root().await? {
            targets.push(target);
        }
        targets.sort();
        targets.dedup();
        Ok(targets)
    }
    pub async fn cleanup_marked_drafts(&self, targets: Vec<PathBuf>) -> MiniAppStorageResult<()> {
        for target in targets {
            if !self.is_cleanup_safe_drafts_root(&target) {
                continue;
            }
            if !self.cleanup_marker_path(&target).exists() {
                continue;
            }
            if target.exists() {
                tokio::fs::remove_dir_all(&target).await.map_err(|e| {
                    MiniAppStorageError::io(format!(
                        "Failed to clean marked miniapp drafts {}: {}",
                        target.display(),
                        e
                    ))
                })?;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Ok(())
    }
    async fn ensure_active_drafts_root_writable(&self) -> MiniAppStorageResult<()> {
        if self.cleanup_marker_path(&self.drafts_root()).exists() {
            let _ = self.isolate_active_drafts_root().await?;
        }
        Ok(())
    }
    async fn collect_marked_drafts_roots(&self) -> MiniAppStorageResult<Vec<PathBuf>> {
        let root = self.miniapps_dir_handle();
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut targets = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&root)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read miniapps dir: {}", e)))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read miniapps entry: {}", e)))?
        {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.starts_with(DRAFTS_CLEANUP_PREFIX) && path.is_dir() && self.cleanup_marker_path(&path).exists() {
                targets.push(path);
            }
        }
        Ok(targets)
    }
    async fn isolate_active_drafts_root(&self) -> MiniAppStorageResult<Option<PathBuf>> {
        let active = self.drafts_root();
        if !active.exists() {
            return Ok(None);
        }
        self.write_cleanup_marker(&active).await?;
        let target = self.cleanup_drafts_root();
        tokio::fs::rename(&active, &target).await.map_err(|e| {
            MiniAppStorageError::io(format!(
                "Failed to mark miniapp drafts for cleanup {} -> {}: {}",
                active.display(),
                target.display(),
                e
            ))
        })?;
        Ok(Some(target))
    }
    pub(super) async fn write_cleanup_marker(&self, drafts_root: &Path) -> MiniAppStorageResult<()> {
        tokio::fs::create_dir_all(drafts_root).await.map_err(|e| {
            MiniAppStorageError::io(format!(
                "Failed to create miniapp drafts dir {}: {}",
                drafts_root.display(),
                e
            ))
        })?;
        tokio::fs::write(self.cleanup_marker_path(drafts_root), "pending miniapp draft cleanup\n")
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to mark miniapp drafts: {}", e)))?;
        Ok(())
    }
    fn is_cleanup_safe_drafts_root(&self, path: &Path) -> bool {
        let root = self.miniapps_dir_handle();
        if !path.starts_with(root) {
            return false;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            return false;
        };
        name == DRAFTS_DIR || name.starts_with(DRAFTS_CLEANUP_PREFIX)
    }
    pub async fn load_draft_storage(&self, app_id: &str, draft_id: &str) -> MiniAppStorageResult<serde_json::Value> {
        self.ensure_active_drafts_root_readable(app_id, draft_id)?;
        let p = self.draft_dir(app_id, draft_id).join(STORAGE_JSON);
        if !p.exists() {
            return Ok(serde_json::json!({}));
        }
        let c = tokio::fs::read_to_string(&p)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read draft storage: {}", e)))?;
        Ok(serde_json::from_str(&c).unwrap_or_else(|_| serde_json::json!({})))
    }
    pub async fn save_draft_storage(
        &self,
        app_id: &str,
        draft_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> MiniAppStorageResult<()> {
        self.ensure_active_drafts_root_writable().await?;
        let dir = self.draft_dir(app_id, draft_id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to create draft dir: {}", e)))?;
        let mut current = self.load_draft_storage(app_id, draft_id).await?;
        let obj = current
            .as_object_mut()
            .ok_or_else(|| MiniAppStorageError::validation("Draft storage is not an object".to_string()))?;
        obj.insert(key.to_string(), value);
        let json = serde_json::to_string_pretty(&current).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(dir.join(STORAGE_JSON), json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write draft storage: {}", e)))?;
        Ok(())
    }
}
