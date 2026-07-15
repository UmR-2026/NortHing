//! MiniApp storage unit tests (split from storage.rs in R38c).
//!
//! These tests exercised `MiniAppStorage` end-to-end from the facade, then the
//! port adapter, app-IO, drafts, and import-bundle IO paths. After R37d's
//! partial split the fixture imports (`INDEX_HTML`, `STYLE_CSS`, `UI_JS`,
//! `WORKER_JS`) leaked to `storage_app_io.rs`; this sibling re-establishes the
//! imports locally so the test surface keeps compiling.

#[cfg(test)]
mod tests {
    use crate::miniapp::storage::{
        MiniApp, MiniAppCustomizationMetadata, MiniAppMeta, MiniAppSource, MiniAppStorage, MiniAppStorageErrorKind,
        NpmDep,
    };
    use northhing_product_domains::miniapp::customization::{
        MiniAppCustomizationOrigin, MiniAppCustomizationOriginKind,
    };
    use northhing_product_domains::miniapp::storage::{
        MiniAppStorageLayout, ESM_DEPS_JSON, INDEX_HTML, META_JSON, STYLE_CSS, UI_JS, WORKER_JS,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use northhing_test_support::TestTempDir;

    #[tokio::test]
    async fn storage_port_adapter_preserves_existing_file_lifecycle() {
        let root = TestTempDir::new("northhing-miniapp-storage-port");
        let miniapps_dir = root.path().join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);
        let port: &dyn northhing_product_domains::miniapp::ports::MiniAppStoragePort = &storage;
        let app = sample_app("demo_app");

        port.save(app.clone()).await.expect("invariant: port.save succeeds");

        let ids = port
            .list_app_ids()
            .await
            .expect("invariant: port.list_app_ids succeeds");
        assert_eq!(ids, vec!["demo_app".to_string()]);

        let meta = port
            .load_meta("demo_app".to_string())
            .await
            .expect("invariant: port.load_meta succeeds");
        assert_eq!(meta.name, "Demo");

        let source = port
            .load_source("demo_app".to_string())
            .await
            .expect("invariant: port.load_source succeeds");
        assert_eq!(source.ui_js, "console.log('ui');");

        let loaded = port
            .load("demo_app".to_string())
            .await
            .expect("invariant: port.load succeeds");
        assert_eq!(loaded.compiled_html, "<html></html>");

        port.save_app_storage("demo_app".to_string(), "answer".to_string(), serde_json::json!(42))
            .await
            .expect("invariant: port.load_app_storage succeeds");
        let app_storage = port
            .load_app_storage("demo_app".to_string())
            .await
            .expect("invariant: port.load_app_storage succeeds");
        assert_eq!(app_storage["answer"], 42);

        port.save_version("demo_app".to_string(), 1, app)
            .await
            .expect("invariant: port.list_versions succeeds");
        assert_eq!(
            port.list_versions("demo_app".to_string())
                .await
                .expect("invariant: port.list_versions succeeds"),
            vec![1]
        );
        assert_eq!(
            port.load_version("demo_app".to_string(), 1)
                .await
                .expect("invariant: test assertion holds")
                .id,
            "demo_app"
        );

        port.delete("demo_app".to_string())
            .await
            .expect("invariant: port.list_app_ids succeeds");
        assert!(port
            .list_app_ids()
            .await
            .expect("invariant: port.list_app_ids succeeds")
            .is_empty());
    }

    #[tokio::test]
    async fn storage_adapter_uses_product_domain_layout_contract() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-layout-port-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir.clone());
        let app = sample_app("layout_app");
        let layout = MiniAppStorageLayout::new(&miniapps_dir, "layout_app");

        storage.save(&app).await.expect("invariant: storage.save succeeds");
        assert!(layout.storage_path().is_file());
        assert_eq!(
            fs::read_to_string(layout.storage_path()).expect("invariant: fs::read_to_string succeeds"),
            "{}".to_string()
        );
        storage
            .save_app_storage("layout_app", "answer", serde_json::json!(42))
            .await
            .expect("invariant: storage.save_version succeeds");
        storage
            .save_version("layout_app", 7, &app)
            .await
            .expect("invariant: storage.save_version succeeds");

        assert!(layout.app_dir().is_dir());
        assert!(layout.meta_path().is_file());
        assert!(layout.compiled_path().is_file());
        assert!(layout.package_json_path().is_file());
        assert!(layout.source_file_path(INDEX_HTML).is_file());
        assert!(layout.source_file_path(STYLE_CSS).is_file());
        assert!(layout.source_file_path(UI_JS).is_file());
        assert!(layout.source_file_path(WORKER_JS).is_file());
        assert!(layout.source_file_path(ESM_DEPS_JSON).is_file());
        assert!(layout.version_path(7).is_file());
    }

    #[tokio::test]
    async fn import_bundle_io_preserves_copy_and_fallback_contract() {
        let root = TestTempDir::new("northhing-miniapp-import-bundle-io");
        let miniapps_dir = root.path().join("miniapps");
        let import_root = root.path().join("import-source");
        let import_source_dir = import_root.join("source");
        fs::create_dir_all(&import_source_dir).expect("invariant: serde_json serialization succeeds");

        let template_app = sample_app("template-id");
        let meta_json = serde_json::to_string_pretty(&MiniAppMeta::from(&template_app))
            .expect("invariant: serde_json serialization succeeds");
        fs::write(import_root.join(META_JSON), &meta_json).expect("invariant: fs::write succeeds");
        fs::write(import_source_dir.join(INDEX_HTML), "<div id=\"app\"></div>").expect("invariant: fs::write succeeds");
        fs::write(import_source_dir.join(STYLE_CSS), "body { color: blue; }").expect("invariant: fs::write succeeds");
        fs::write(
            import_source_dir.join(UI_JS),
            "document.getElementById('app').textContent = 'imported';",
        )
        .expect("invariant: storage.read_import_meta_json succeeds");
        fs::write(import_source_dir.join(WORKER_JS), "").expect("invariant: storage.read_import_meta_json succeeds");

        let storage = MiniAppStorage::new(miniapps_dir.clone());
        let read_meta = storage
            .read_import_meta_json(&import_root)
            .await
            .expect("invariant: storage.read_import_meta_json succeeds");
        assert_eq!(read_meta, meta_json);

        storage
            .write_import_bundle(
                northhing_product_domains::miniapp::storage::MiniAppImportBundleWriteRequest {
                    source_path: import_root,
                    app_id: "imported-app".to_string(),
                    meta_json,
                    esm_dependencies_json: "[]".to_string(),
                    package_json: "{\"name\":\"miniapp-imported-app\"}".to_string(),
                    storage_json: "{}".to_string(),
                    compiled_html: "<html>placeholder</html>".to_string(),
                },
            )
            .await
            .expect("invariant: fs::read_to_string succeeds");

        let layout = MiniAppStorageLayout::new(&miniapps_dir, "imported-app");
        assert_eq!(
            fs::read_to_string(layout.source_file_path(STYLE_CSS)).expect("invariant: fs::read_to_string succeeds"),
            "body { color: blue; }"
        );
        assert_eq!(
            fs::read_to_string(layout.source_file_path(ESM_DEPS_JSON)).expect("invariant: fs::read_to_string succeeds"),
            "[]"
        );
        assert_eq!(
            fs::read_to_string(layout.package_json_path()).expect("invariant: fs::read_to_string succeeds"),
            "{\"name\":\"miniapp-imported-app\"}"
        );
        assert_eq!(
            fs::read_to_string(layout.storage_path()).expect("invariant: fs::read_to_string succeeds"),
            "{}"
        );
        assert_eq!(
            fs::read_to_string(layout.compiled_path()).expect("invariant: fs::read_to_string succeeds"),
            "<html>placeholder</html>"
        );
    }

    #[tokio::test]
    async fn saving_app_files_preserves_existing_storage_json() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-storage-preserve-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);
        let app = sample_app("storage_app");

        storage.save(&app).await.expect("invariant: storage.save succeeds");
        storage
            .save_app_storage("storage_app", "answer", serde_json::json!(42))
            .await
            .expect("invariant: storage.save succeeds");
        storage.save(&app).await.expect("invariant: storage.save succeeds");

        assert_eq!(
            storage
                .load_app_storage("storage_app")
                .await
                .expect("invariant: test assertion holds")
                .get("answer"),
            Some(&serde_json::json!(42))
        );
    }

    #[tokio::test]
    async fn draft_storage_is_hidden_and_isolated_from_active_storage() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-draft-storage-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);
        let app = sample_app("demo_app");

        storage.save(&app).await.expect("invariant: storage.save succeeds");
        storage
            .save_app_storage("demo_app", "answer", serde_json::json!(42))
            .await
            .expect("invariant: test assertion holds");
        storage
            .save_draft_storage("demo_app", "draft_one", "answer", serde_json::json!(7))
            .await
            .expect("invariant: test assertion holds");

        assert_eq!(
            storage
                .load_app_storage("demo_app")
                .await
                .expect("invariant: test assertion holds")
                .get("answer"),
            Some(&serde_json::json!(42))
        );
        assert_eq!(
            storage
                .load_draft_storage("demo_app", "draft_one")
                .await
                .expect("invariant: storage.list_app_ids succeeds")
                .get("answer"),
            Some(&serde_json::json!(7))
        );
        assert_eq!(
            storage
                .list_app_ids()
                .await
                .expect("invariant: storage.delete succeeds"),
            vec!["demo_app"]
        );

        let draft_dir = storage.app_drafts_dir("demo_app");
        assert!(draft_dir.exists());
        storage
            .delete("demo_app")
            .await
            .expect("invariant: storage.delete succeeds");
        assert!(!draft_dir.exists());
    }

    #[tokio::test]
    async fn mark_stale_drafts_moves_sandboxes_off_the_active_read_path() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-stale-drafts-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);
        let app = sample_app("demo_app");

        storage.save(&app).await.expect("invariant: storage.save succeeds");
        storage
            .save_draft_storage("demo_app", "stale_draft", "answer", serde_json::json!(7))
            .await
            .expect("invariant: storage.mark_stale_drafts_for_cleanup succeeds");

        assert!(storage.drafts_root().exists());
        let cleanup_targets = storage
            .mark_stale_drafts_for_cleanup()
            .await
            .expect("invariant: storage.mark_stale_drafts_for_cleanup succeeds");

        assert_eq!(cleanup_targets.len(), 1);
        assert!(cleanup_targets[0].exists());
        assert!(storage.cleanup_marker_path(&cleanup_targets[0]).exists());
        assert!(!storage.drafts_root().exists());
        assert!(storage.load("demo_app").await.is_ok());
        assert_eq!(
            storage
                .load_draft_storage("demo_app", "stale_draft")
                .await
                .expect("invariant: test assertion holds"),
            serde_json::json!({})
        );
    }

    #[tokio::test]
    async fn draft_reads_skip_marked_active_root() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-marked-draft-read-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);

        storage
            .save_draft_storage("demo_app", "stale_draft", "answer", serde_json::json!(7))
            .await
            .expect("invariant: test assertion holds");
        storage
            .write_cleanup_marker(&storage.drafts_root())
            .await
            .expect("invariant: test assertion holds");

        let error = storage.load_draft_storage("demo_app", "stale_draft").await.unwrap_err();
        assert_eq!(error.kind(), MiniAppStorageErrorKind::NotFound);
    }

    #[tokio::test]
    async fn cleanup_marked_drafts_removes_quarantined_sandboxes_later() {
        let root = std::env::temp_dir().join(format!(
            "northhing-miniapp-clean-marked-drafts-{}",
            uuid::Uuid::new_v4()
        ));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);

        storage
            .save_draft_storage("demo_app", "stale_draft", "answer", serde_json::json!(7))
            .await
            .expect("invariant: storage.mark_stale_drafts_for_cleanup succeeds");
        let cleanup_targets = storage
            .mark_stale_drafts_for_cleanup()
            .await
            .expect("invariant: storage.mark_stale_drafts_for_cleanup succeeds");
        let cleanup_root = cleanup_targets[0].clone();

        storage
            .cleanup_marked_drafts(cleanup_targets)
            .await
            .expect("invariant: test assertion holds");

        assert!(!cleanup_root.exists());
        assert!(!storage.drafts_root().exists());
    }

    #[tokio::test]
    async fn saving_new_draft_isolates_marked_active_root_first() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-marked-draft-write-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);

        storage
            .save_draft_storage("demo_app", "stale_draft", "answer", serde_json::json!(7))
            .await
            .expect("invariant: test assertion holds");
        storage
            .write_cleanup_marker(&storage.drafts_root())
            .await
            .expect("invariant: test assertion holds");

        storage
            .save_draft_storage("demo_app", "fresh_draft", "answer", serde_json::json!(9))
            .await
            .expect("invariant: test assertion holds");

        assert_eq!(
            storage
                .load_draft_storage("demo_app", "fresh_draft")
                .await
                .expect("invariant: test assertion holds")
                .get("answer"),
            Some(&serde_json::json!(9))
        );
        assert!(!storage.cleanup_marker_path(&storage.drafts_root()).exists());
    }

    #[tokio::test]
    async fn customization_metadata_roundtrips() {
        let root = std::env::temp_dir().join(format!("northhing-miniapp-customization-meta-{}", uuid::Uuid::new_v4()));
        let miniapps_dir = root.join("miniapps");
        let storage = MiniAppStorage::new(miniapps_dir);
        let app = sample_app("builtin-demo");
        storage.save(&app).await.expect("invariant: storage.save succeeds");

        let metadata = MiniAppCustomizationMetadata {
            origin: MiniAppCustomizationOrigin {
                kind: MiniAppCustomizationOriginKind::Builtin,
                builtin_id: Some("builtin-demo".to_string()),
                builtin_version: Some(3),
            },
            local_override: true,
            last_applied_draft_id: Some("draft_one".to_string()),
            available_builtin_update: None,
            declined_builtin_updates: Vec::new(),
            updated_at: 123,
        };

        storage
            .save_customization_metadata("builtin-demo", &metadata)
            .await
            .expect("invariant: test assertion holds");

        assert_eq!(
            storage
                .load_customization_metadata("builtin-demo")
                .await
                .expect("invariant: test assertion holds"),
            Some(metadata)
        );
    }

    fn sample_app(id: &str) -> MiniApp {
        MiniApp {
            id: id.to_string(),
            name: "Demo".to_string(),
            description: "Demo app".to_string(),
            icon: "sparkles".to_string(),
            category: "tools".to_string(),
            tags: vec!["demo".to_string()],
            version: 1,
            created_at: 1,
            updated_at: 2,
            source: MiniAppSource {
                html: "<div id=\"app\"></div>".to_string(),
                css: "body {}".to_string(),
                ui_js: "console.log('ui');".to_string(),
                esm_dependencies: Vec::new(),
                worker_js: "export default {};".to_string(),
                npm_dependencies: vec![NpmDep {
                    name: "lodash".to_string(),
                    version: "^4.17.21".to_string(),
                }],
            },
            compiled_html: "<html></html>".to_string(),
            permissions: Default::default(),
            ai_context: None,
            runtime: Default::default(),
            i18n: None,
        }
    }
}
