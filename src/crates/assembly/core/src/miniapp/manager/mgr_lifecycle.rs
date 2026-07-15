//! Lifecycle / write-side operations on [`MiniAppManager`]:
//! create, update, delete, draft create/sync/permission/apply/discard,
//! app and draft KV storage, customization metadata, builtin-update flow,
//! mark-deps-installed / clear-worker-restart, rollback, recompile, sync-from-fs,
//! and import-from-path.
//!
//! Everything in here goes through `runtime_facade()` for persistence; errors
//! are normalised through [`super::mgr_types::map_miniapp_port_error`].

use super::mgr_types::map_miniapp_port_error;
use super::MiniAppManager;
use crate::miniapp::types::{MiniApp, MiniAppAiContext, MiniAppPermissions, MiniAppSource};
use crate::util::errors::NortHingResult;
use chrono::Utc;
use northhing_product_domains::miniapp::customization::MiniAppCustomizationMetadata;
use northhing_product_domains::miniapp::draft::MiniAppDraft;
use northhing_product_domains::miniapp::lifecycle::{MiniAppCreateInput, MiniAppUpdatePatch};
use northhing_product_domains::miniapp::ports::MiniAppImportFromPathRequest;
use std::path::{Path, PathBuf};
use uuid::Uuid;

impl MiniAppManager {
    /// Create a new MiniApp (generates id, sets created_at/updated_at, compiles).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        name: String,
        description: String,
        icon: String,
        category: String,
        tags: Vec<String>,
        source: MiniAppSource,
        permissions: MiniAppPermissions,
        ai_context: Option<MiniAppAiContext>,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniApp> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();

        let compiled_html = self.compile_source(&id, &source, &permissions, "dark", workspace_root)?;

        self.runtime_facade()
            .create_app(
                id,
                MiniAppCreateInput {
                    name,
                    description,
                    icon,
                    category,
                    tags,
                    source,
                    permissions,
                    ai_context,
                },
                compiled_html,
                now,
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Update existing MiniApp (increment version, recompile, save).
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        app_id: &str,
        name: Option<String>,
        description: Option<String>,
        icon: Option<String>,
        category: Option<String>,
        tags: Option<Vec<String>>,
        source: Option<MiniAppSource>,
        permissions: Option<MiniAppPermissions>,
        ai_context: Option<MiniAppAiContext>,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniApp> {
        let previous_app = self.storage.load(app_id).await?;
        let patch = MiniAppUpdatePatch {
            name,
            description,
            icon,
            category,
            tags,
            source,
            permissions,
            ai_context,
        };
        let now = Utc::now().timestamp_millis();
        let compiled_html = self.compile_source(
            app_id,
            patch.source_for_compile(&previous_app),
            patch.permissions_for_compile(&previous_app),
            "dark",
            workspace_root,
        )?;
        self.runtime_facade()
            .persist_update_result_for_app(app_id.to_string(), previous_app, patch, compiled_html, now)
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Delete MiniApp and its directory.
    pub async fn delete(&self, app_id: &str) -> NortHingResult<()> {
        self.granted_paths.write().await.remove(app_id);
        self.storage.delete(app_id).await
    }

    /// Get app storage (KV) value.
    pub async fn get_storage(&self, app_id: &str, key: &str) -> NortHingResult<serde_json::Value> {
        let storage = self.storage.load_app_storage(app_id).await?;
        Ok(storage.get(key).cloned().unwrap_or(serde_json::Value::Null))
    }

    /// Set app storage (KV) value.
    pub async fn set_storage(&self, app_id: &str, key: &str, value: serde_json::Value) -> NortHingResult<()> {
        self.storage.save_app_storage(app_id, key, value).await
    }

    pub async fn get_draft_storage(
        &self,
        app_id: &str,
        draft_id: &str,
        key: &str,
    ) -> NortHingResult<serde_json::Value> {
        let storage = self.storage.load_draft_storage(app_id, draft_id).await?;
        Ok(storage.get(key).cloned().unwrap_or(serde_json::Value::Null))
    }

    pub async fn set_draft_storage(
        &self,
        app_id: &str,
        draft_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> NortHingResult<()> {
        self.storage.save_draft_storage(app_id, draft_id, key, value).await
    }

    pub async fn create_draft(
        &self,
        app_id: &str,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniAppDraft> {
        let app = self.get(app_id).await?;
        let now = Utc::now().timestamp_millis();
        let draft_id = Uuid::new_v4().to_string();
        let compiled_html = self.compile_source_with_app_data_dir(
            app_id,
            &self.storage.draft_dir(app_id, &draft_id),
            &app.source,
            &app.permissions,
            theme,
            workspace_root,
        )?;
        let draft_root = self.storage.draft_dir(app_id, &draft_id).to_string_lossy().to_string();
        self.runtime_facade()
            .persist_draft_for_app(app_id.to_string(), draft_id, draft_root, app, compiled_html, now)
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn get_draft(&self, app_id: &str, draft_id: &str) -> NortHingResult<MiniAppDraft> {
        self.runtime_facade()
            .get_draft(
                app_id.to_string(),
                draft_id.to_string(),
                self.draft_root_string(app_id, draft_id),
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn sync_draft_from_fs(
        &self,
        app_id: &str,
        draft_id: &str,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniAppDraft> {
        let draft = self.get_draft(app_id, draft_id).await?;
        let now = Utc::now().timestamp_millis();
        let compiled_html = self.compile_source_with_app_data_dir(
            app_id,
            &self.storage.draft_dir(app_id, draft_id),
            &draft.app.source,
            &draft.app.permissions,
            theme,
            workspace_root,
        )?;
        self.runtime_facade()
            .persist_draft_source_sync_result(draft, compiled_html, now)
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn set_draft_permissions(
        &self,
        app_id: &str,
        draft_id: &str,
        permissions: MiniAppPermissions,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniAppDraft> {
        let draft = self.get_draft(app_id, draft_id).await?;
        let now = Utc::now().timestamp_millis();
        let compiled_html = self.compile_source_with_app_data_dir(
            app_id,
            &self.storage.draft_dir(app_id, draft_id),
            &draft.app.source,
            &permissions,
            theme,
            workspace_root,
        )?;
        self.runtime_facade()
            .persist_draft_permission_update_result(draft, permissions, compiled_html, now)
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn permission_diff_for_draft(
        &self,
        app_id: &str,
        draft_id: &str,
    ) -> NortHingResult<northhing_product_domains::miniapp::customization::MiniAppPermissionDiff> {
        self.runtime_facade()
            .permission_diff_for_draft(app_id.to_string(), draft_id.to_string())
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn apply_draft(
        &self,
        app_id: &str,
        draft_id: &str,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniApp> {
        let current = self.get(app_id).await?;
        let draft_app = self.storage.load_draft_app(app_id, draft_id).await?;
        let now = Utc::now().timestamp_millis();
        let compiled_html =
            self.compile_source(app_id, &draft_app.source, &draft_app.permissions, theme, workspace_root)?;
        self.runtime_facade()
            .apply_draft_app(
                current,
                draft_id.to_string(),
                draft_app,
                compiled_html,
                self.customization_baseline(app_id),
                now,
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn discard_draft(&self, app_id: &str, draft_id: &str) -> NortHingResult<()> {
        self.runtime_facade()
            .discard_draft(app_id.to_string(), draft_id.to_string())
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn mark_stale_drafts_for_cleanup(&self) -> NortHingResult<Vec<PathBuf>> {
        self.storage.mark_stale_drafts_for_cleanup().await
    }

    pub async fn cleanup_marked_drafts(&self, targets: Vec<PathBuf>) -> NortHingResult<()> {
        self.storage.cleanup_marked_drafts(targets).await
    }

    pub async fn load_customization_metadata(
        &self,
        app_id: &str,
    ) -> NortHingResult<Option<MiniAppCustomizationMetadata>> {
        self.storage.load_customization_metadata(app_id).await
    }

    pub async fn save_customization_metadata(
        &self,
        app_id: &str,
        metadata: &MiniAppCustomizationMetadata,
    ) -> NortHingResult<()> {
        self.storage.save_customization_metadata(app_id, metadata).await
    }

    pub async fn mark_builtin_update_available(
        &self,
        app_id: &str,
        builtin_version: u32,
        source_hash: &str,
        detected_at: i64,
    ) -> NortHingResult<bool> {
        self.runtime_facade()
            .mark_builtin_update_available(
                app_id.to_string(),
                builtin_version,
                source_hash.to_string(),
                detected_at,
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn decline_builtin_update(
        &self,
        app_id: &str,
        builtin_version: u32,
        source_hash: &str,
        declined_at: i64,
    ) -> NortHingResult<Option<MiniAppCustomizationMetadata>> {
        self.runtime_facade()
            .decline_builtin_update(
                app_id.to_string(),
                builtin_version,
                source_hash.to_string(),
                declined_at,
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn mark_deps_installed(&self, app_id: &str) -> NortHingResult<MiniApp> {
        self.runtime_facade()
            .mark_deps_installed(app_id.to_string())
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn clear_worker_restart_required(&self, app_id: &str) -> NortHingResult<MiniApp> {
        self.runtime_facade()
            .clear_worker_restart_required(app_id.to_string())
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Rollback app to a previous version (loads version snapshot, saves as current).
    pub async fn rollback(&self, app_id: &str, version: u32) -> NortHingResult<MiniApp> {
        let now = Utc::now().timestamp_millis();
        self.runtime_facade()
            .rollback(app_id.to_string(), version, now)
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Recompile app (e.g. after workspace or theme change). Updates compiled_html and saves.
    pub async fn recompile(&self, app_id: &str, theme: &str, workspace_root: Option<&Path>) -> NortHingResult<MiniApp> {
        let app = self.storage.load(app_id).await?;
        let compiled_html = self.compile_source(app_id, &app.source, &app.permissions, theme, workspace_root)?;
        self.runtime_facade()
            .persist_recompile_result_for_app(app, compiled_html, Utc::now().timestamp_millis())
            .await
            .map_err(map_miniapp_port_error)
    }

    pub async fn sync_from_fs(
        &self,
        app_id: &str,
        theme: &str,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniApp> {
        let previous_app = self.storage.load(app_id).await?;
        let source = self.storage.load_source_only(app_id).await?;
        let compiled_html = self.compile_source(app_id, &source, &previous_app.permissions, theme, workspace_root)?;
        self.runtime_facade()
            .persist_sync_from_fs_result_for_app(
                app_id.to_string(),
                previous_app,
                source,
                compiled_html,
                Utc::now().timestamp_millis(),
            )
            .await
            .map_err(map_miniapp_port_error)
    }

    /// Import a MiniApp from a directory (e.g. miniapps/git-graph). Copies meta, source, package.json, storage into a new app id and recompiles.
    pub async fn import_from_path(
        &self,
        source_path: PathBuf,
        workspace_root: Option<&Path>,
    ) -> NortHingResult<MiniApp> {
        let id = Uuid::new_v4().to_string();
        let imported_at = Utc::now().timestamp_millis();
        self.runtime_facade()
            .import_from_path(
                &self.storage,
                self,
                MiniAppImportFromPathRequest {
                    source_path,
                    app_id: id,
                    theme: "dark".to_string(),
                    workspace_root: workspace_root.map(Path::to_path_buf),
                    imported_at,
                    recompiled_at: Utc::now().timestamp_millis(),
                },
            )
            .await
            .map_err(map_miniapp_port_error)
    }
}
