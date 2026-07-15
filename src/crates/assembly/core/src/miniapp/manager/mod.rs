//! MiniApp manager: facade over compile / policy / registry / lifecycle modules.
//!
//! Implementation is split across sibling files within this directory:
//! - [`mgr_runtime`]   compile, policy resolution, grant tracking, port trait impl
//! - [`mgr_registry`]  read-side lookups (list / get / versions / draft paths)
//! - [`mgr_lifecycle`] create / update / delete / draft / storage / rollback / import
//! - [`mgr_types`]     error mapping helpers (miniapp port errors <-> `NortHingError`)
//!
//! Public surface stays on this facade: struct, [`new`], [`path_manager`],
//! global initialiser/getter.

use std::sync::{Arc, OnceLock};

mod mgr_lifecycle;
mod mgr_registry;
mod mgr_runtime;
mod mgr_types;

static GLOBAL_MINIAPP_MANAGER: OnceLock<Arc<MiniAppManager>> = OnceLock::new();

/// Initialize the global MiniAppManager (called once at startup from Tauri app_state).
pub fn initialize_global_miniapp_manager(manager: Arc<MiniAppManager>) {
    let _ = GLOBAL_MINIAPP_MANAGER.set(manager);
}

/// Get the global MiniAppManager, returning None if not initialized.
pub fn try_get_global_miniapp_manager() -> Option<Arc<MiniAppManager>> {
    GLOBAL_MINIAPP_MANAGER.get().cloned()
}

/// MiniApp manager: create, read, update, delete, list, compile, rollback.
pub struct MiniAppManager {
    pub(super) storage: crate::miniapp::storage::MiniAppStorage,
    pub(super) path_manager: Arc<crate::infrastructure::PathManager>,
    /// User-granted paths per app (for resolve_policy).
    pub(super) granted_paths: tokio::sync::RwLock<std::collections::HashMap<String, Vec<std::path::PathBuf>>>,
}

impl MiniAppManager {
    pub fn new(path_manager: Arc<crate::infrastructure::PathManager>) -> Self {
        let storage = crate::miniapp::storage::MiniAppStorage::new(path_manager.clone());
        Self {
            storage,
            path_manager,
            granted_paths: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get the path manager (for external callers that need paths like miniapp_dir).
    pub fn path_manager(&self) -> &Arc<crate::infrastructure::PathManager> {
        &self.path_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miniapp::types::{FsPermissions, MiniAppMeta, MiniAppPermissions, MiniAppSource, NpmDep};
    use northhing_product_domains::miniapp::storage::{
        COMPILED_HTML, DRAFT_JSON, ESM_DEPS_JSON, INDEX_HTML, META_JSON, PACKAGE_JSON, SOURCE_DIR, STORAGE_JSON,
        STYLE_CSS, UI_JS, WORKER_JS,
    };

    fn test_manager() -> MiniAppManager {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-manager-draft-{}", uuid::Uuid::new_v4()));
        let path_manager = Arc::new(crate::infrastructure::PathManager::with_user_root_for_tests(root));
        MiniAppManager::new(path_manager)
    }

    fn sample_source(css: &str) -> MiniAppSource {
        MiniAppSource {
            html: "<!DOCTYPE html><html><head></head><body><div id=\"app\"></div></body></html>".to_string(),
            css: css.to_string(),
            ui_js: "document.getElementById('app').textContent = 'demo';".to_string(),
            esm_dependencies: Vec::new(),
            worker_js: String::new(),
            npm_dependencies: Vec::new(),
        }
    }

    async fn create_sample_app(manager: &MiniAppManager) -> crate::miniapp::types::MiniApp {
        manager
            .create(
                "Demo".to_string(),
                "Demo app".to_string(),
                "box".to_string(),
                "utility".to_string(),
                vec!["demo".to_string()],
                sample_source("body { color: black; }"),
                MiniAppPermissions::default(),
                None,
                None,
            )
            .await
            .expect("invariant: manager.create in create_sample_app helper succeeds")
    }

    #[test]
    fn miniapp_port_error_mapping_preserves_manager_error_shape() {
        let not_found =
            super::mgr_types::map_miniapp_port_error(northhing_product_domains::miniapp::ports::MiniAppPortError::new(
                northhing_product_domains::miniapp::ports::MiniAppPortErrorKind::NotFound,
                "Not found: MiniApp not found: missing",
            ));
        assert_eq!(not_found.to_string(), "Not found: MiniApp not found: missing");

        let deserialization =
            super::mgr_types::map_miniapp_port_error(northhing_product_domains::miniapp::ports::MiniAppPortError::new(
                northhing_product_domains::miniapp::ports::MiniAppPortErrorKind::Deserialization,
                "Deserialization error: Invalid draft manifest",
            ));
        assert_eq!(
            deserialization.to_string(),
            "Deserialization error: Invalid draft manifest"
        );

        let permission_denied =
            super::mgr_types::map_miniapp_port_error(northhing_product_domains::miniapp::ports::MiniAppPortError::new(
                northhing_product_domains::miniapp::ports::MiniAppPortErrorKind::PermissionDenied,
                "IO error: access denied",
            ));
        match permission_denied {
            crate::util::errors::NortHingError::Io(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
                assert_eq!(error.to_string(), "access denied");
            }
            other => panic!("expected permission denied IO error, got {other:?}"),
        }
    }

    async fn write_import_source(root: &std::path::Path) {
        let source_dir = root.join(SOURCE_DIR);
        tokio::fs::create_dir_all(&source_dir)
            .await
            .expect("invariant: tokio::fs::create_dir_all succeeds");
        let meta = MiniAppMeta {
            id: "template-id".to_string(),
            name: "Imported".to_string(),
            description: "Imported app".to_string(),
            icon: "box".to_string(),
            category: "utility".to_string(),
            tags: vec!["imported".to_string()],
            version: 7,
            created_at: 11,
            updated_at: 12,
            permissions: MiniAppPermissions::default(),
            ai_context: None,
            runtime: Default::default(),
            i18n: None,
        };
        tokio::fs::write(
            root.join(META_JSON),
            serde_json::to_string_pretty(&meta).expect("invariant: serde_json serialization succeeds"),
        )
        .await
        .expect("invariant: tokio::fs::write succeeds");
        tokio::fs::write(
            source_dir.join(INDEX_HTML),
            "<!DOCTYPE html><html><head></head><body><div id=\"app\"></div></body></html>",
        )
        .await
        .expect("invariant: tokio::fs::write succeeds");
        tokio::fs::write(source_dir.join(STYLE_CSS), "body { color: blue; }")
            .await
            .expect("invariant: tokio::fs::write succeeds");
        tokio::fs::write(
            source_dir.join(UI_JS),
            "document.getElementById('app').textContent = 'imported';",
        )
        .await
        .expect("invariant: tokio::fs::write succeeds");
        tokio::fs::write(source_dir.join(WORKER_JS), "")
            .await
            .expect("invariant: tokio::fs::write worker.js succeeds");
    }

    #[tokio::test]
    async fn runtime_preflight_preserves_recompile_sync_rollback_and_deps_state() {
        let manager = test_manager();
        let mut app = create_sample_app(&manager).await;
        app.source.npm_dependencies = vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }];
        manager
            .storage
            .save(&app)
            .await
            .expect("invariant: storage.save succeeds");

        let installed = manager
            .mark_deps_installed(&app.id)
            .await
            .expect("invariant: manager.mark_deps_installed succeeds");
        assert!(!installed.runtime.deps_dirty);
        assert!(installed.runtime.worker_restart_required);
        let cleared = manager
            .clear_worker_restart_required(&app.id)
            .await
            .expect("invariant: manager.clear_worker_restart_required succeeds");
        assert!(!cleared.runtime.worker_restart_required);

        let style_path = manager
            .path_manager()
            .miniapp_dir(&app.id)
            .join(SOURCE_DIR)
            .join(STYLE_CSS);
        tokio::fs::write(&style_path, "body { color: red; }")
            .await
            .expect("invariant: manager.sync_from_fs succeeds");
        let synced = manager
            .sync_from_fs(&app.id, "dark", None)
            .await
            .expect("invariant: manager.sync_from_fs succeeds");
        assert_eq!(synced.version, app.version + 1);
        assert_eq!(synced.source.css, "body { color: red; }");
        assert!(synced.runtime.deps_dirty);
        assert!(synced.runtime.worker_restart_required);
        assert_eq!(
            manager
                .list_versions(&app.id)
                .await
                .expect("invariant: manager.recompile succeeds"),
            vec![1]
        );

        let recompiled = manager
            .recompile(&app.id, "dark", None)
            .await
            .expect("invariant: manager.recompile succeeds");
        assert_eq!(recompiled.version, synced.version);
        assert_eq!(recompiled.source.css, synced.source.css);
        assert!(recompiled.compiled_html.contains("body { color: red; }"));
        assert!(!recompiled.runtime.ui_recompile_required);

        let rolled_back = manager
            .rollback(&app.id, app.version)
            .await
            .expect("invariant: manager.rollback succeeds");
        assert_eq!(rolled_back.version, recompiled.version + 1);
        // sync_from_fs snapshots the source already loaded from disk; keep this
        // boundary explicit before moving manager/runtime ownership.
        assert_eq!(rolled_back.source.css, "body { color: red; }");
        assert!(rolled_back.runtime.deps_dirty);
        assert!(rolled_back.runtime.worker_restart_required);
        assert_eq!(
            manager
                .list_versions(&app.id)
                .await
                .expect("invariant: manager.list_versions succeeds"),
            vec![1, 2]
        );
    }

    #[tokio::test]
    async fn import_from_path_preserves_fallback_files_recompile_and_runtime_state() {
        let manager = test_manager();
        let import_root =
            std::env::temp_dir().join(format!("northhing-miniapp-import-source-{}", uuid::Uuid::new_v4()));
        write_import_source(&import_root).await;

        let imported = manager
            .import_from_path(import_root.clone(), None)
            .await
            .expect("invariant: manager.import_from_path succeeds");
        let app_dir = manager.path_manager().miniapp_dir(&imported.id);
        let source_dir = app_dir.join(SOURCE_DIR);

        assert_ne!(imported.id, "template-id");
        assert_eq!(imported.name, "Imported");
        assert_eq!(imported.version, 7);
        assert_eq!(imported.source.css, "body { color: blue; }");
        assert!(imported.compiled_html.contains("textContent = 'imported'"));
        assert!(!imported.runtime.deps_dirty);
        assert!(imported.runtime.worker_restart_required);
        assert!(!imported.runtime.ui_recompile_required);

        assert_eq!(
            tokio::fs::read_to_string(source_dir.join(ESM_DEPS_JSON))
                .await
                .expect("invariant: tokio::fs::read_to_string succeeds"),
            "[]"
        );
        assert_eq!(
            tokio::fs::read_to_string(app_dir.join(STORAGE_JSON))
                .await
                .expect("invariant: serde_json deserialization succeeds"),
            "{}"
        );
        let package_json: serde_json::Value = serde_json::from_str(
            &tokio::fs::read_to_string(app_dir.join(PACKAGE_JSON))
                .await
                .expect("invariant: tokio::fs::read_to_string package.json succeeds"),
        )
        .expect("invariant: serde_json deserialization succeeds");
        assert_eq!(package_json["name"], format!("miniapp-{}", imported.id));
        assert_eq!(package_json["dependencies"], serde_json::json!({}));
        assert!(tokio::fs::read_to_string(app_dir.join(COMPILED_HTML))
            .await
            .expect("invariant: tokio::fs::read_to_string compiled.html succeeds")
            .contains("textContent = 'imported'"));

        let _ = tokio::fs::remove_dir_all(import_root).await;
    }

    #[tokio::test]
    async fn import_from_path_preserves_invalid_meta_error_shape() {
        let manager = test_manager();
        let import_root = std::env::temp_dir().join(format!(
            "northhing-miniapp-invalid-import-source-{}",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::create_dir_all(&import_root)
            .await
            .expect("invariant: tokio::fs::create_dir_all succeeds");
        let source_dir = import_root.join(SOURCE_DIR);
        tokio::fs::create_dir_all(&source_dir)
            .await
            .expect("invariant: tokio::fs::create_dir_all succeeds");
        for file_name in [INDEX_HTML, STYLE_CSS, UI_JS, WORKER_JS] {
            tokio::fs::write(source_dir.join(file_name), "")
                .await
                .expect("invariant: tokio::fs::write succeeds");
        }
        tokio::fs::write(import_root.join(META_JSON), "{")
            .await
            .expect("invariant: manager.import_from_path succeeds");

        let error = manager.import_from_path(import_root.clone(), None).await;

        match error {
            Err(crate::util::errors::NortHingError::Deserialization(message)) => {
                assert!(message.starts_with("Invalid meta.json:"));
            }
            other => panic!("expected invalid meta deserialization error, got {other:?}"),
        }
        let _ = tokio::fs::remove_dir_all(import_root).await;
    }

    #[tokio::test]
    async fn draft_lifecycle_keeps_active_storage_and_source_isolated_until_apply() {
        let manager = test_manager();
        let app = create_sample_app(&manager).await;
        manager
            .set_storage(&app.id, "score", serde_json::json!(3))
            .await
            .expect("invariant: manager.create_draft succeeds");

        let draft = manager
            .create_draft(&app.id, "dark", None)
            .await
            .expect("invariant: manager.create_draft succeeds");
        assert_eq!(draft.source_version, app.version);
        assert_eq!(draft.app.source.css, "body { color: black; }");

        let draft_css = manager
            .storage
            .draft_dir(&app.id, &draft.draft_id)
            .join("source")
            .join("style.css");
        tokio::fs::write(&draft_css, "body { background: white; }")
            .await
            .expect("invariant: tokio::fs::write draft style.css succeeds");

        let draft = manager
            .sync_draft_from_fs(&app.id, &draft.draft_id, "dark", None)
            .await
            .expect("invariant: manager.get succeeds");
        assert_eq!(draft.app.source.css, "body { background: white; }");

        let active_before_apply = manager
            .get(&app.id)
            .await
            .expect("invariant: manager.get_storage succeeds");
        assert_eq!(active_before_apply.source.css, "body { color: black; }");
        assert_eq!(
            manager
                .get_storage(&app.id, "score")
                .await
                .expect("invariant: manager.get_storage succeeds"),
            serde_json::json!(3)
        );

        let applied = manager
            .apply_draft(&app.id, &draft.draft_id, "dark", None)
            .await
            .expect("invariant: manager.list_versions succeeds");

        assert_eq!(applied.version, app.version + 1);
        assert_eq!(applied.source.css, "body { background: white; }");
        assert_eq!(
            manager
                .list_versions(&app.id)
                .await
                .expect("invariant: manager.get_storage succeeds"),
            vec![1]
        );
        assert_eq!(
            manager
                .get_storage(&app.id, "score")
                .await
                .expect("invariant: manager.get_storage succeeds"),
            serde_json::json!(3)
        );
    }

    #[tokio::test]
    async fn apply_draft_does_not_require_manifest_metadata() {
        let manager = test_manager();
        let app = create_sample_app(&manager).await;
        let draft = manager
            .create_draft(&app.id, "dark", None)
            .await
            .expect("invariant: manager.create_draft succeeds");
        let draft_dir = manager.storage.draft_dir(&app.id, &draft.draft_id);
        tokio::fs::remove_file(draft_dir.join(DRAFT_JSON))
            .await
            .expect("invariant: tokio::fs::remove_file draft.json succeeds");

        let applied = manager
            .apply_draft(&app.id, &draft.draft_id, "dark", None)
            .await
            .expect("invariant: manager.list_versions succeeds");

        assert_eq!(applied.version, app.version + 1);
        assert_eq!(applied.source.css, app.source.css);
        assert_eq!(
            manager
                .list_versions(&app.id)
                .await
                .expect("invariant: manager.list_versions succeeds"),
            vec![1]
        );
    }

    #[tokio::test]
    async fn draft_permission_diff_flags_high_risk_changes_before_apply() {
        let manager = test_manager();
        let app = create_sample_app(&manager).await;
        let draft = manager
            .create_draft(&app.id, "dark", None)
            .await
            .expect("invariant: manager.create_draft succeeds");

        let draft_permissions = MiniAppPermissions {
            fs: Some(FsPermissions {
                read: None,
                write: Some(vec!["{workspace}".to_string()]),
            }),
            ..Default::default()
        };
        manager
            .set_draft_permissions(&app.id, &draft.draft_id, draft_permissions, "dark", None)
            .await
            .expect("invariant: manager.set_draft_permissions succeeds");

        let diff = manager
            .permission_diff_for_draft(&app.id, &draft.draft_id)
            .await
            .expect("invariant: manager.get succeeds");

        assert!(diff.high_risk);
        assert_eq!(diff.added, vec!["fs.write:{workspace}".to_string()]);
        assert!(manager
            .get(&app.id)
            .await
            .expect("invariant: manager.get succeeds")
            .permissions
            .fs
            .is_none());
    }
}
