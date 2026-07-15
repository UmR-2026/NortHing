#![cfg(feature = "miniapp")]

//! Runtime Facade And Customization tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn miniapp_runtime_facade_persists_port_backed_lifecycle_transitions() {
    let mut app = sample_miniapp_for_lifecycle(MiniAppSource {
        css: "body { color: black; }".to_string(),
        npm_dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
        ..MiniAppSource::default()
    });
    app.runtime = build_runtime_state(app.version, app.updated_at, &app.source, true, false);
    let storage = StoragePortStub::new(app);
    let facade = MiniAppRuntimeFacade::new(&storage);

    let installed = block_on(facade.mark_deps_installed("demo".to_string())).unwrap();
    assert!(!installed.runtime.deps_dirty);
    assert!(installed.runtime.worker_restart_required);

    let cleared = block_on(facade.clear_worker_restart_required("demo".to_string())).unwrap();
    assert!(!cleared.runtime.worker_restart_required);

    let recompiled =
        block_on(facade.persist_recompile_result("demo".to_string(), "<html>fresh</html>".to_string(), 2000)).unwrap();
    assert_eq!(recompiled.version, 3);
    assert_eq!(recompiled.compiled_html, "<html>fresh</html>");
    assert!(!recompiled.runtime.ui_recompile_required);

    let synced_source = MiniAppSource {
        css: "body { color: red; }".to_string(),
        npm_dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
        ..MiniAppSource::default()
    };
    let synced = block_on(facade.persist_sync_from_fs_result(
        "demo".to_string(),
        synced_source,
        "<html>synced</html>".to_string(),
        3000,
    ))
    .unwrap();
    assert_eq!(synced.version, 4);
    assert_eq!(synced.source.css, "body { color: red; }");
    assert!(synced.runtime.deps_dirty);
    assert!(synced.runtime.worker_restart_required);
    assert_eq!(storage.saved_version_numbers(), vec![3]);

    let rolled_back = block_on(facade.rollback("demo".to_string(), 3, 4000)).unwrap();
    assert_eq!(rolled_back.version, 5);
    assert_eq!(rolled_back.compiled_html, "<html>fresh</html>");
    assert!(rolled_back.runtime.worker_restart_required);
    assert_eq!(storage.saved_version_numbers(), vec![3, 4]);
}

#[test]
fn miniapp_runtime_facade_owns_manager_create_update_draft_and_apply_workflows() {
    let storage = StoragePortStub::new(sample_miniapp_for_lifecycle(MiniAppSource::default()));
    let facade = MiniAppRuntimeFacade::new(&storage);

    let created = block_on(facade.create_app(
        "created".to_string(),
        MiniAppCreateInput {
            name: "Created".to_string(),
            description: "Created app".to_string(),
            icon: "box".to_string(),
            category: "utility".to_string(),
            tags: vec!["created".to_string()],
            source: MiniAppSource {
                css: "body { color: black; }".to_string(),
                ..MiniAppSource::default()
            },
            permissions: MiniAppPermissions::default(),
            ai_context: None,
        },
        "<html>created</html>".to_string(),
        1000,
    ))
    .unwrap();
    assert_eq!(created.id, "created");
    assert_eq!(created.version, 1);
    assert_eq!(storage.current().compiled_html, "<html>created</html>");

    let updated = block_on(facade.persist_update_result_for_app(
        "created".to_string(),
        created.clone(),
        MiniAppUpdatePatch {
            source: Some(MiniAppSource {
                css: "body { color: red; }".to_string(),
                ..MiniAppSource::default()
            }),
            ..MiniAppUpdatePatch::default()
        },
        "<html>updated</html>".to_string(),
        2000,
    ))
    .unwrap();
    assert_eq!(updated.version, 2);
    assert_eq!(updated.source.css, "body { color: red; }");
    assert_eq!(storage.saved_version_numbers(), vec![1]);

    let draft = block_on(facade.persist_draft_for_app(
        "created".to_string(),
        "draft-1".to_string(),
        "/tmp/draft-1".to_string(),
        updated.clone(),
        "<html>draft</html>".to_string(),
        3000,
    ))
    .unwrap();
    assert_eq!(draft.app_id, "created");
    assert_eq!(draft.source_version, 2);
    assert_eq!(draft.draft_root, "/tmp/draft-1");
    assert_eq!(draft.app.compiled_html, "<html>draft</html>");

    let draft = block_on(facade.persist_draft_permission_update_result(
        draft,
        MiniAppPermissions {
            fs: Some(FsPermissions {
                read: Some(vec!["{workspace}".to_string()]),
                write: None,
            }),
            ..MiniAppPermissions::default()
        },
        "<html>permissioned</html>".to_string(),
        3500,
    ))
    .unwrap();
    assert_eq!(draft.updated_at, 3500);
    assert_eq!(draft.app.compiled_html, "<html>permissioned</html>");

    let applied = block_on(facade.apply_loaded_draft(
        updated,
        draft,
        "<html>applied</html>".to_string(),
        MiniAppCustomizationBaseline::UserCreated,
        4000,
    ))
    .unwrap();
    assert_eq!(applied.version, 3);
    assert_eq!(applied.compiled_html, "<html>applied</html>");
    assert_eq!(storage.saved_version_numbers(), vec![1, 2]);

    let metadata = storage
        .customization_metadata("created")
        .expect("customization metadata should be saved");
    assert_eq!(metadata.last_applied_draft_id.as_deref(), Some("draft-1"));
    assert_eq!(metadata.updated_at, 4000);

    block_on(facade.discard_draft("created".to_string(), "draft-1".to_string())).unwrap();
    assert_eq!(
        storage.deleted_drafts(),
        vec![("created".to_string(), "draft-1".to_string())]
    );
}

#[test]
fn miniapp_runtime_facade_owns_import_bundle_recompile_and_runtime_state_workflow() {
    let storage = StoragePortStub::new(sample_miniapp_for_lifecycle(MiniAppSource::default()));
    let import_port = ImportPortStub::new(
        storage.clone(),
        serde_json::json!({
            "id": "legacy-id",
            "name": "Imported",
            "description": "Imported app",
            "icon": "box",
            "category": "utility",
            "tags": ["imported"],
            "version": 99,
            "created_at": 1,
            "updated_at": 2,
            "permissions": {}
        })
        .to_string(),
    );
    let compile_port = CompilePortStub::new();
    let facade = MiniAppRuntimeFacade::new(&storage);

    let imported = block_on(facade.import_from_path(
        &import_port,
        &compile_port,
        MiniAppImportFromPathRequest {
            source_path: PathBuf::from("fixtures/imported"),
            app_id: "imported-id".to_string(),
            theme: "dark".to_string(),
            workspace_root: Some(PathBuf::from("workspace/project")),
            imported_at: 5000,
            recompiled_at: 6000,
        },
    ))
    .unwrap();

    assert_eq!(imported.id, "imported-id");
    assert_eq!(imported.name, "Imported");
    assert_eq!(imported.compiled_html, "<html>imported-id:dark</html>");
    assert_eq!(
        imported.runtime.source_revision,
        build_source_revision(imported.version, imported.updated_at)
    );
    assert!(!imported.runtime.ui_recompile_required);
    assert_eq!(imported.updated_at, 6000);
    assert_eq!(storage.save_count(), 2);
    assert_eq!(compile_port.calls().len(), 1);
    assert!(compile_port.calls()[0].contains("imported-id|<html><body>imported</body></html>|dark|workspace/project"));

    let writes = import_port.writes();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].source_path, PathBuf::from("fixtures/imported"));
    assert_eq!(writes[0].app_id, "imported-id");
    let written_meta: serde_json::Value = serde_json::from_str(&writes[0].meta_json).unwrap();
    assert_eq!(written_meta["id"], "imported-id");
    assert_eq!(written_meta["updated_at"], 5000);
    assert_eq!(writes[0].compiled_html, PLACEHOLDER_COMPILED_HTML);
}

#[test]
fn miniapp_runtime_facade_skips_save_when_restart_flag_already_clear() {
    let mut app = sample_miniapp_for_lifecycle(MiniAppSource::default());
    app.runtime = build_runtime_state(app.version, app.updated_at, &app.source, false, false);
    let storage = StoragePortStub::new(app);
    let facade = MiniAppRuntimeFacade::new(&storage);

    let unchanged = block_on(facade.clear_worker_restart_required("demo".to_string())).unwrap();

    assert!(!unchanged.runtime.worker_restart_required);
    assert_eq!(storage.save_count(), 0);
    assert_eq!(storage.current().version, 3);
}

#[test]
fn miniapp_runtime_facade_preserves_storage_errors_without_state_writes() {
    let app = sample_miniapp_for_lifecycle(MiniAppSource::default());
    let storage = StoragePortStub::new(app);
    let facade = MiniAppRuntimeFacade::new(&storage);

    let missing_app = block_on(facade.mark_deps_installed("missing".to_string())).unwrap_err();
    assert_eq!(missing_app.kind, MiniAppPortErrorKind::NotFound);
    assert_eq!(storage.save_count(), 0);
    assert!(storage.saved_version_numbers().is_empty());

    let missing_version = block_on(facade.rollback("demo".to_string(), 99, 4000)).unwrap_err();
    assert_eq!(missing_version.kind, MiniAppPortErrorKind::NotFound);
    assert_eq!(storage.save_count(), 0);
    assert!(storage.saved_version_numbers().is_empty());
}

#[test]
fn miniapp_draft_contract_preserves_manifest_and_response_shape() {
    let app = sample_miniapp_for_lifecycle(MiniAppSource::default());
    let manifest = build_draft_manifest("app-1", "draft-1", 7, 1234);

    assert_eq!(manifest.app_id, "app-1");
    assert_eq!(manifest.draft_id, "draft-1");
    assert_eq!(manifest.source_version, 7);
    assert_eq!(manifest.status, MINIAPP_DRAFT_STATUS_DRAFT);
    assert_eq!(manifest.created_at, 1234);
    assert_eq!(manifest.updated_at, 1234);

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["appId"], "app-1");
    assert_eq!(json["draftId"], "draft-1");
    assert_eq!(json["sourceVersion"], 7);

    let response = build_draft_response("/tmp/draft", app, manifest.clone());
    assert_eq!(response.app_id, "app-1");
    assert_eq!(response.draft_root, "/tmp/draft");
    assert_eq!(response.app.id, "demo");

    let mut applied = manifest;
    applied.mark_applied(2345);
    assert_eq!(applied.status, MINIAPP_DRAFT_STATUS_APPLIED);
    assert_eq!(applied.updated_at, 2345);
}

#[test]
fn miniapp_customization_apply_helper_preserves_builtin_override_policy() {
    let metadata = apply_draft_customization_metadata(
        None,
        MiniAppCustomizationBaseline::Builtin {
            builtin_id: "builtin-pr-review".to_string(),
            builtin_version: 4,
        },
        "draft-1",
        1234,
    );

    assert_eq!(metadata.origin.kind, MiniAppCustomizationOriginKind::Builtin);
    assert_eq!(metadata.origin.builtin_id.as_deref(), Some("builtin-pr-review"));
    assert_eq!(metadata.origin.builtin_version, Some(4));
    assert!(metadata.local_override);
    assert_eq!(metadata.last_applied_draft_id.as_deref(), Some("draft-1"));
    assert!(metadata.available_builtin_update.is_none());
    assert_eq!(metadata.updated_at, 1234);

    let updated = apply_draft_customization_metadata(
        Some(metadata),
        MiniAppCustomizationBaseline::Builtin {
            builtin_id: "builtin-pr-review".to_string(),
            builtin_version: 5,
        },
        "draft-2",
        2345,
    );

    assert_eq!(updated.origin.builtin_version, Some(5));
    assert!(updated.local_override);
    assert_eq!(updated.last_applied_draft_id.as_deref(), Some("draft-2"));
    assert!(updated.available_builtin_update.is_none());

    let user_created = MiniAppCustomizationMetadata {
        origin: MiniAppCustomizationOrigin {
            kind: MiniAppCustomizationOriginKind::UserCreated,
            builtin_id: None,
            builtin_version: None,
        },
        local_override: false,
        last_applied_draft_id: None,
        available_builtin_update: None,
        declined_builtin_updates: Vec::new(),
        updated_at: 10,
    };
    let user_created_update = apply_draft_customization_metadata(
        Some(user_created),
        MiniAppCustomizationBaseline::Builtin {
            builtin_id: "builtin-pr-review".to_string(),
            builtin_version: 6,
        },
        "draft-3",
        3456,
    );

    assert_eq!(
        user_created_update.origin.kind,
        MiniAppCustomizationOriginKind::UserCreated
    );
    assert!(!user_created_update.local_override);
    assert_eq!(user_created_update.last_applied_draft_id.as_deref(), Some("draft-3"));
    assert_eq!(user_created_update.updated_at, 3456);
}

#[test]
fn miniapp_customization_builtin_update_policy_preserves_decline_contract() {
    let mut metadata = apply_draft_customization_metadata(
        None,
        MiniAppCustomizationBaseline::Builtin {
            builtin_id: "builtin-pr-review".to_string(),
            builtin_version: 4,
        },
        "draft-1",
        1234,
    );

    let available = mark_builtin_update_available_metadata(metadata, 5, "hash-v5", 2000, false);
    assert!(available.should_surface_update);
    assert!(available.metadata_changed);
    metadata = available.metadata;
    assert_eq!(
        metadata.available_builtin_update.as_ref().unwrap().source_hash,
        "hash-v5"
    );

    metadata = decline_builtin_update_metadata(
        metadata,
        5,
        "hash-v5",
        2100,
        Some(MiniAppCustomizationLocalSnapshot {
            version: 7,
            updated_at: 2200,
        }),
    );

    assert!(metadata.available_builtin_update.is_none());
    assert_eq!(metadata.updated_at, 2100);
    assert_eq!(metadata.declined_builtin_updates.len(), 1);
    assert_eq!(
        metadata.declined_builtin_updates[0].last_applied_draft_id.as_deref(),
        Some("draft-1")
    );
    assert!(declined_builtin_update_needs_local_snapshot(&metadata, "hash-v5"));
    assert!(is_current_declined_builtin_update(
        &metadata,
        "hash-v5",
        Some(MiniAppCustomizationLocalSnapshot {
            version: 7,
            updated_at: 2200,
        }),
    ));
    assert!(!is_current_declined_builtin_update(
        &metadata,
        "hash-v5",
        Some(MiniAppCustomizationLocalSnapshot {
            version: 8,
            updated_at: 2200,
        }),
    ));

    let suppressed = mark_builtin_update_available_metadata(metadata.clone(), 5, "hash-v5", 2300, true);
    assert!(!suppressed.should_surface_update);
    assert!(!suppressed.metadata_changed);
    assert!(suppressed.metadata.available_builtin_update.is_none());

    let fallback = is_current_declined_builtin_update(&metadata, "hash-v5", None);
    assert!(fallback);
}

#[test]
fn miniapp_customization_decline_policy_updates_existing_and_trims_old_records() {
    let mut metadata = apply_draft_customization_metadata(
        None,
        MiniAppCustomizationBaseline::Builtin {
            builtin_id: "builtin-pr-review".to_string(),
            builtin_version: 4,
        },
        "draft-1",
        1000,
    );

    metadata = decline_builtin_update_metadata(metadata, 5, "hash-v5", 2000, None);
    metadata = decline_builtin_update_metadata(metadata, 5, "hash-v5", 2500, None);
    assert_eq!(metadata.declined_builtin_updates.len(), 1);
    assert_eq!(metadata.declined_builtin_updates[0].declined_at, 2500);

    for idx in 0..=MAX_DECLINED_BUILTIN_UPDATES {
        metadata = decline_builtin_update_metadata(
            metadata,
            6 + idx as u32,
            &format!("hash-{}", idx),
            3000 + idx as i64,
            None,
        );
    }

    assert_eq!(metadata.declined_builtin_updates.len(), MAX_DECLINED_BUILTIN_UPDATES);
    assert!(!metadata
        .declined_builtin_updates
        .iter()
        .any(|record| record.source_hash == "hash-v5"));
}

fn sample_miniapp_for_lifecycle(source: MiniAppSource) -> MiniApp {
    MiniApp {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        description: "Demo app".to_string(),
        icon: "sparkles".to_string(),
        category: "tools".to_string(),
        tags: Vec::new(),
        version: 3,
        created_at: 1,
        updated_at: 1234,
        source,
        compiled_html: "<html></html>".to_string(),
        permissions: MiniAppPermissions::default(),
        ai_context: None,
        runtime: MiniAppRuntimeState::default(),
        i18n: None,
    }
}
