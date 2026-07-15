//! MiniApp storage app IO methods (split from storage.rs in R37d).
//!
//! Owns app CRUD, source/package file IO, version snapshots, app storage (KV),
//! and customization metadata IO. Drafts and import bundle IO live in
//! `storage_drafts.rs`; the import bundle methods (`read_import_meta_json`,
//! `write_import_bundle`) stay inline in `storage.rs` because the boundary
//! `required-rules` anchor them there.

use super::storage::{MiniApp, MiniAppMeta, MiniAppSource, MiniAppStorageError, MiniAppStorageResult, NpmDep};
use northhing_product_domains::miniapp::storage::{
    build_package_json, parse_npm_dependencies, COMPILED_HTML, ESM_DEPS_JSON, INDEX_HTML, META_JSON, PACKAGE_JSON,
    STORAGE_JSON, STYLE_CSS, UI_JS, WORKER_JS,
};
use std::path::PathBuf;

impl super::storage::MiniAppStorage {
    pub async fn ensure_app_dir(&self, app_id: &str) -> MiniAppStorageResult<()> {
        let dir = self.app_dir(app_id);
        let source = self.source_dir(app_id);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to create miniapp dir {}: {}", dir.display(), e)))?;
        tokio::fs::create_dir_all(&source)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to create source dir {}: {}", source.display(), e)))?;
        Ok(())
    }
    pub async fn list_app_ids(&self) -> MiniAppStorageResult<Vec<String>> {
        let root = self.miniapps_dir_handle();
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&root)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read miniapps dir: {}", e)))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read miniapps entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.') {
                        ids.push(name.to_string());
                    }
                }
            }
        }
        Ok(ids)
    }
    pub async fn load(&self, app_id: &str) -> MiniAppStorageResult<MiniApp> {
        let meta_path = self.meta_path(app_id);
        let meta_content = tokio::fs::read_to_string(&meta_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MiniAppStorageError::not_found(format!("MiniApp not found: {}", app_id))
            } else {
                MiniAppStorageError::io(format!("Failed to read meta: {}", e))
            }
        })?;
        let meta: MiniAppMeta = serde_json::from_str(&meta_content)
            .map_err(|e| MiniAppStorageError::parse(format!("Invalid meta.json: {}", e)))?;

        let source = self.load_source(app_id).await?;
        let compiled_html = self.load_compiled_html(app_id).await?;

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
    pub async fn load_meta(&self, app_id: &str) -> MiniAppStorageResult<MiniAppMeta> {
        let meta_path = self.meta_path(app_id);
        let content = tokio::fs::read_to_string(&meta_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MiniAppStorageError::not_found(format!("MiniApp not found: {}", app_id))
            } else {
                MiniAppStorageError::io(format!("Failed to read meta: {}", e))
            }
        })?;
        serde_json::from_str(&content).map_err(|e| MiniAppStorageError::parse(format!("Invalid meta.json: {}", e)))
    }
    async fn load_source(&self, app_id: &str) -> MiniAppStorageResult<MiniAppSource> {
        self.load_source_from_dirs(self.source_dir(app_id), self.app_dir(app_id))
            .await
    }
    pub(super) async fn load_source_from_dirs(
        &self,
        source_dir: PathBuf,
        package_dir: PathBuf,
    ) -> MiniAppStorageResult<MiniAppSource> {
        let sd = source_dir;
        let html = tokio::fs::read_to_string(sd.join(INDEX_HTML)).await.unwrap_or_default();
        let css = tokio::fs::read_to_string(sd.join(STYLE_CSS)).await.unwrap_or_default();
        let ui_js = tokio::fs::read_to_string(sd.join(UI_JS)).await.unwrap_or_default();
        let worker_js = tokio::fs::read_to_string(sd.join(WORKER_JS)).await.unwrap_or_default();

        let esm_dependencies = if sd.join(ESM_DEPS_JSON).exists() {
            let c = tokio::fs::read_to_string(sd.join(ESM_DEPS_JSON))
                .await
                .unwrap_or_default();
            serde_json::from_str(&c).unwrap_or_default()
        } else {
            Vec::new()
        };

        let npm_dependencies = self
            .load_npm_dependencies_from_package(package_dir.join(PACKAGE_JSON))
            .await?;

        Ok(MiniAppSource {
            html,
            css,
            ui_js,
            esm_dependencies,
            worker_js,
            npm_dependencies,
        })
    }
    pub async fn load_source_only(&self, app_id: &str) -> MiniAppStorageResult<MiniAppSource> {
        self.load_source(app_id).await
    }
    async fn load_npm_dependencies_from_package(&self, p: PathBuf) -> MiniAppStorageResult<Vec<NpmDep>> {
        if !p.exists() {
            return Ok(Vec::new());
        }
        let c = tokio::fs::read_to_string(&p)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read package.json: {}", e)))?;
        parse_npm_dependencies(&c).map_err(|e| MiniAppStorageError::parse(format!("Invalid package.json: {}", e)))
    }
    async fn load_compiled_html(&self, app_id: &str) -> MiniAppStorageResult<String> {
        let p = self.compiled_path(app_id);
        tokio::fs::read_to_string(&p).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MiniAppStorageError::not_found(format!("Compiled HTML not found: {}", app_id))
            } else {
                MiniAppStorageError::io(format!("Failed to read compiled.html: {}", e))
            }
        })
    }
    pub async fn save(&self, app: &MiniApp) -> MiniAppStorageResult<()> {
        self.save_app_files(&self.app_dir(&app.id), &self.source_dir(&app.id), app)
            .await
    }
    pub(super) async fn save_app_files(
        &self,
        app_dir: &std::path::Path,
        source_dir: &std::path::Path,
        app: &MiniApp,
    ) -> MiniAppStorageResult<()> {
        tokio::fs::create_dir_all(app_dir).await.map_err(|e| {
            MiniAppStorageError::io(format!("Failed to create miniapp dir {}: {}", app_dir.display(), e))
        })?;
        tokio::fs::create_dir_all(source_dir).await.map_err(|e| {
            MiniAppStorageError::io(format!("Failed to create source dir {}: {}", source_dir.display(), e))
        })?;
        let meta = MiniAppMeta::from(app);
        let meta_path = app_dir.join(META_JSON);
        let meta_json = serde_json::to_string_pretty(&meta).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(&meta_path, meta_json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write meta: {}", e)))?;

        let sd = source_dir;
        tokio::fs::write(sd.join(INDEX_HTML), &app.source.html)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write index.html: {}", e)))?;
        tokio::fs::write(sd.join(STYLE_CSS), &app.source.css)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write style.css: {}", e)))?;
        tokio::fs::write(sd.join(UI_JS), &app.source.ui_js)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write ui.js: {}", e)))?;
        tokio::fs::write(sd.join(WORKER_JS), &app.source.worker_js)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write worker.js: {}", e)))?;

        let esm_json =
            serde_json::to_string_pretty(&app.source.esm_dependencies).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(sd.join(ESM_DEPS_JSON), esm_json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write esm_dependencies.json: {}", e)))?;

        self.write_package_json_to_dir(app_dir, &app.id, &app.source.npm_dependencies)
            .await?;

        let storage_path = app_dir.join(STORAGE_JSON);
        if !storage_path.exists() {
            tokio::fs::write(&storage_path, "{}")
                .await
                .map_err(|e| MiniAppStorageError::io(format!("Failed to write storage.json: {}", e)))?;
        }

        tokio::fs::write(app_dir.join(COMPILED_HTML), &app.compiled_html)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write compiled.html: {}", e)))?;

        Ok(())
    }
    async fn write_package_json_to_dir(
        &self,
        app_dir: &std::path::Path,
        app_id: &str,
        deps: &[NpmDep],
    ) -> MiniAppStorageResult<()> {
        let pkg = build_package_json(app_id, deps);
        let json = serde_json::to_string_pretty(&pkg).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(app_dir.join(PACKAGE_JSON), json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write package.json: {}", e)))?;
        Ok(())
    }
    pub async fn save_version(&self, app_id: &str, version: u32, app: &MiniApp) -> MiniAppStorageResult<()> {
        let versions_dir = self.layout(app_id).versions_dir();
        tokio::fs::create_dir_all(&versions_dir)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to create versions dir: {}", e)))?;
        let path = self.version_path(app_id, version);
        let json = serde_json::to_string_pretty(app).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(&path, json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write version file: {}", e)))?;
        Ok(())
    }
    pub async fn load_app_storage(&self, app_id: &str) -> MiniAppStorageResult<serde_json::Value> {
        let p = self.storage_path(app_id);
        if !p.exists() {
            return Ok(serde_json::json!({}));
        }
        let c = tokio::fs::read_to_string(&p)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read storage: {}", e)))?;
        Ok(serde_json::from_str(&c).unwrap_or_else(|_| serde_json::json!({})))
    }
    pub async fn save_app_storage(
        &self,
        app_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> MiniAppStorageResult<()> {
        self.ensure_app_dir(app_id).await?;
        let mut current = self.load_app_storage(app_id).await?;
        let obj = current
            .as_object_mut()
            .ok_or_else(|| MiniAppStorageError::validation("App storage is not an object".to_string()))?;
        obj.insert(key.to_string(), value);
        let p = self.storage_path(app_id);
        let json = serde_json::to_string_pretty(&current).map_err(MiniAppStorageError::parse)?;
        tokio::fs::write(&p, json)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to write storage: {}", e)))?;
        Ok(())
    }
    pub async fn list_versions(&self, app_id: &str) -> MiniAppStorageResult<Vec<u32>> {
        let vdir = self.layout(app_id).versions_dir();
        if !vdir.exists() {
            return Ok(Vec::new());
        }
        let mut versions = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&vdir)
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read versions dir: {}", e)))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| MiniAppStorageError::io(format!("Failed to read versions entry: {}", e)))?
        {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('v') && name.ends_with(".json") {
                if let Ok(n) = name[1..name.len() - 5].parse::<u32>() {
                    versions.push(n);
                }
            }
        }
        versions.sort();
        Ok(versions)
    }
    pub async fn load_version(&self, app_id: &str, version: u32) -> MiniAppStorageResult<MiniApp> {
        let p = self.version_path(app_id, version);
        let c = tokio::fs::read_to_string(&p).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MiniAppStorageError::not_found(format!("Version v{} not found", version))
            } else {
                MiniAppStorageError::io(format!("Failed to read version: {}", e))
            }
        })?;
        serde_json::from_str(&c).map_err(|e| MiniAppStorageError::parse(format!("Invalid version file: {}", e)))
    }
}
