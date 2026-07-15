#![cfg(feature = "miniapp")]

//! Compiler Export Storage And Runtime tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn miniapp_compiler_preserves_head_injection_contract() {
    let source = MiniAppSource {
        html: r#"<!DOCTYPE html><html><head><meta charset="utf-8"></head><body>x</body></html>"#.to_string(),
        ui_js: "console.log('ready')".to_string(),
        ..MiniAppSource::default()
    };

    let out = compile(
        &source,
        &MiniAppPermissions::default(),
        "app-id",
        "/tmp/app",
        "/tmp/workspace",
        "dark",
    )
    .unwrap();

    assert!(out.contains("<meta charset=\"utf-8\">"));
    assert!(out.contains("data-theme-type=\"dark\""));
    assert!(out.contains("<script type=\"module\">"));
    assert!(out.contains("console.log('ready')"));
}

#[test]
fn miniapp_export_and_runtime_dtos_remain_stable() {
    assert_eq!(RuntimeKind::Node, RuntimeKind::Node);

    let target = serde_json::to_string(&ExportTarget::Tauri).unwrap();
    assert_eq!(target, "\"Tauri\"");

    let check = ExportCheckResult {
        ready: false,
        runtime: None,
        missing: vec!["No JS runtime (install Bun or Node.js)".to_string()],
        warnings: Vec::new(),
    };
    let json = serde_json::to_value(&check).unwrap();
    assert_eq!(json["ready"], false);
    assert_eq!(json["missing"][0], "No JS runtime (install Bun or Node.js)");

    let install = InstallResult {
        success: true,
        stdout: "ok".to_string(),
        stderr: String::new(),
    };
    let json = serde_json::to_value(&install).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["stdout"], "ok");

    assert_eq!(export_runtime_label(&RuntimeKind::Bun), "bun");
    assert_eq!(export_runtime_label(&RuntimeKind::Node), "node");
    assert_eq!(MISSING_JS_RUNTIME_MESSAGE, "No JS runtime (install Bun or Node.js)");
    let missing_runtime = build_export_check_result(None);
    assert!(!missing_runtime.ready);
    assert_eq!(missing_runtime.runtime, None);
    assert_eq!(missing_runtime.missing, vec![MISSING_JS_RUNTIME_MESSAGE]);
    let detected_runtime = build_export_check_result(Some(&RuntimeKind::Node));
    assert!(detected_runtime.ready);
    assert_eq!(detected_runtime.runtime.as_deref(), Some("node"));
    assert!(detected_runtime.missing.is_empty());
}

#[test]
fn miniapp_storage_layout_preserves_file_shape_contract() {
    let root = PathBuf::from("/northhing/miniapps");
    let layout = MiniAppStorageLayout::new(&root, "app-1");

    assert_eq!(META_JSON, "meta.json");
    assert_eq!(SOURCE_DIR, "source");
    assert_eq!(INDEX_HTML, "index.html");
    assert_eq!(STYLE_CSS, "style.css");
    assert_eq!(UI_JS, "ui.js");
    assert_eq!(WORKER_JS, "worker.js");
    assert_eq!(PACKAGE_JSON, "package.json");
    assert_eq!(ESM_DEPS_JSON, "esm_dependencies.json");
    assert_eq!(COMPILED_HTML, "compiled.html");
    assert_eq!(STORAGE_JSON, "storage.json");
    assert_eq!(VERSIONS_DIR, "versions");
    assert_eq!(DRAFTS_DIR, ".drafts");
    assert_eq!(DRAFTS_CLEANUP_PREFIX, ".drafts.cleanup-");
    assert_eq!(DRAFTS_CLEANUP_MARKER, ".cleanup-pending");
    assert_eq!(DRAFT_JSON, "draft.json");
    assert_eq!(CUSTOMIZATION_JSON, ".customization.json");

    assert_eq!(layout.app_dir(), root.join("app-1"));
    assert_eq!(layout.meta_path(), root.join("app-1").join(META_JSON));
    assert_eq!(
        layout.source_file_path(INDEX_HTML),
        root.join("app-1").join(SOURCE_DIR).join(INDEX_HTML)
    );
    assert_eq!(
        layout.version_path(3),
        root.join("app-1").join(VERSIONS_DIR).join("v3.json")
    );
    assert_eq!(layout.versions_dir(), root.join("app-1").join(VERSIONS_DIR));
    assert_eq!(layout.customization_path(), root.join("app-1").join(CUSTOMIZATION_JSON));
    assert_eq!(MiniAppStorageLayout::drafts_root(&root), root.join(DRAFTS_DIR));
    assert_eq!(
        MiniAppStorageLayout::draft_dir(&root, "app-1", "draft-1"),
        root.join(DRAFTS_DIR).join("app-1").join("draft-1")
    );
    assert_eq!(
        MiniAppStorageLayout::draft_source_dir(&root, "app-1", "draft-1"),
        root.join(DRAFTS_DIR).join("app-1").join("draft-1").join(SOURCE_DIR)
    );
    assert_eq!(
        MiniAppStorageLayout::draft_manifest_path(&root, "app-1", "draft-1"),
        root.join(DRAFTS_DIR).join("app-1").join("draft-1").join(DRAFT_JSON)
    );
    assert_eq!(
        MiniAppStorageLayout::cleanup_drafts_root(&root, "cleanup-id"),
        root.join(".drafts.cleanup-cleanup-id")
    );
}

#[test]
fn miniapp_runtime_search_plan_preserves_common_install_locations() {
    let home = PathBuf::from("/home/northhing");
    let candidates = candidate_dirs(Some(&home));

    assert_eq!(candidates[0], PathBuf::from("/opt/homebrew/bin"));
    assert!(candidates.contains(&home.join(".bun").join("bin")));
    assert!(candidates.contains(&home.join(".asdf").join("shims")));

    let roots = version_manager_roots(Some(&home));
    assert_eq!(roots[0], home.join(".nvm").join("versions").join("node"));
    assert!(roots.contains(&home.join(".fnm").join("node-versions")));

    assert_eq!(runtime_lookup_order(), &["bun", "node"]);
    let _detect_runtime: fn() -> Option<DetectedRuntime> = detect_runtime;
    assert_eq!(
        candidate_executable_path(Path::new("/usr/local/bin"), "node"),
        PathBuf::from("/usr/local/bin").join("node")
    );
    assert_eq!(
        versioned_executable_candidate(Path::new("/home/northhing/.nvm/versions/node/v20"), "node"),
        PathBuf::from("/home/northhing/.nvm/versions/node/v20")
            .join("bin")
            .join("node")
    );
}

#[test]
fn miniapp_worker_install_command_preserves_runtime_choice() {
    let bun = install_command_for_runtime(&RuntimeKind::Bun, true);
    assert_eq!(bun.program, "bun");
    assert_eq!(bun.args, &["install", "--production"]);

    let node_with_pnpm = install_command_for_runtime(&RuntimeKind::Node, true);
    assert_eq!(node_with_pnpm.program, "pnpm");
    assert_eq!(node_with_pnpm.args, &["install", "--prod"]);

    let node_without_pnpm = install_command_for_runtime(&RuntimeKind::Node, false);
    assert_eq!(node_without_pnpm.program, "npm");
    assert_eq!(node_without_pnpm.args, &["install", "--production"]);

    assert_eq!(
        plan_install_deps(false, &RuntimeKind::Node, true),
        InstallDepsPlan::SkipMissingPackageJson
    );
    assert!(matches!(
        plan_install_deps(true, &RuntimeKind::Node, true),
        InstallDepsPlan::Run(command) if command.program == "pnpm"
    ));
    assert!(!worker_pool_at_capacity(4));
    assert!(worker_pool_at_capacity(5));
    assert!(worker_is_idle(10_000, 10_000 - worker_idle_timeout_ms() - 1));
    assert_eq!(
        select_lru_worker([("newer", 20), ("older", 10)]),
        Some("older".to_string())
    );
}

#[test]
fn miniapp_storage_package_json_contract_remains_stable() {
    let deps = parse_npm_dependencies(
        r#"{
            "name": "miniapp-demo",
            "dependencies": {
                "left-pad": "^1.3.0",
                "local-only": { "workspace": true }
            }
        }"#,
    )
    .unwrap();

    assert!(deps.contains(&NpmDep {
        name: "left-pad".to_string(),
        version: "^1.3.0".to_string(),
    }));
    assert!(deps.contains(&NpmDep {
        name: "local-only".to_string(),
        version: "*".to_string(),
    }));

    let package = build_package_json(
        "demo",
        &[NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
    );

    assert_eq!(package["name"], "miniapp-demo");
    assert_eq!(package["private"], true);
    assert_eq!(package["dependencies"]["lodash"], "^4.17.21");
}

#[test]
fn miniapp_storage_import_fallback_contract_remains_stable() {
    let root = PathBuf::from("/miniapps/incoming");
    let layout = MiniAppImportLayout::new(&root);

    assert_eq!(layout.meta_path(), root.join(META_JSON));
    assert_eq!(layout.source_dir(), root.join(SOURCE_DIR));
    assert_eq!(
        layout.source_file_path(INDEX_HTML),
        root.join(SOURCE_DIR).join(INDEX_HTML)
    );
    assert_eq!(
        layout.required_source_file_paths(),
        vec![
            (INDEX_HTML, root.join(SOURCE_DIR).join(INDEX_HTML)),
            (STYLE_CSS, root.join(SOURCE_DIR).join(STYLE_CSS)),
            (UI_JS, root.join(SOURCE_DIR).join(UI_JS)),
            (WORKER_JS, root.join(SOURCE_DIR).join(WORKER_JS)),
        ]
    );
    assert_eq!(
        layout.esm_dependencies_path(),
        root.join(SOURCE_DIR).join(ESM_DEPS_JSON)
    );
    assert_eq!(layout.package_json_path(), root.join(PACKAGE_JSON));
    assert_eq!(layout.storage_json_path(), root.join(STORAGE_JSON));

    assert_eq!(REQUIRED_SOURCE_FILES, &[INDEX_HTML, STYLE_CSS, UI_JS, WORKER_JS]);
    assert_eq!(EMPTY_ESM_DEPENDENCIES_JSON, "[]");
    assert_eq!(EMPTY_STORAGE_JSON, "{}");
    assert_eq!(
        PLACEHOLDER_COMPILED_HTML,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>Loading...</body></html>"
    );

    let package = build_package_json("imported-app", &[]);
    assert_eq!(package["name"], "miniapp-imported-app");
    assert_eq!(package["private"], true);
    assert_eq!(package["dependencies"], serde_json::json!({}));

    let fallbacks = build_import_fallbacks("imported-app");
    assert_eq!(fallbacks.esm_dependencies_json, "[]");
    assert_eq!(fallbacks.storage_json, "{}");
    assert_eq!(fallbacks.compiled_html, PLACEHOLDER_COMPILED_HTML);
    assert_eq!(fallbacks.package_json, package);
}

#[test]
fn miniapp_import_bundle_plan_rehomes_meta_and_preserves_fallback_wire_shape() {
    let source_meta_json = serde_json::json!({
        "id": "template-id",
        "name": "Imported",
        "description": "Imported app",
        "icon": "box",
        "category": "utility",
        "tags": ["demo"],
        "version": 7,
        "created_at": 11,
        "updated_at": 12,
        "permissions": {},
        "runtime": {}
    })
    .to_string();

    let plan = build_import_bundle_plan("new-app", &source_meta_json, 1234).unwrap();

    assert_eq!(plan.esm_dependencies_json, "[]");
    assert_eq!(plan.storage_json, "{}");
    assert_eq!(plan.compiled_html, PLACEHOLDER_COMPILED_HTML);
    let meta: serde_json::Value = serde_json::from_str(&plan.meta_json).unwrap();
    assert_eq!(meta["id"], "new-app");
    assert_eq!(meta["name"], "Imported");
    assert_eq!(meta["version"], 7);
    assert_eq!(meta["created_at"], 1234);
    assert_eq!(meta["updated_at"], 1234);

    let package: serde_json::Value = serde_json::from_str(&plan.package_json).unwrap();
    assert_eq!(package["name"], "miniapp-new-app");
    assert_eq!(package["private"], true);
    assert_eq!(package["dependencies"], serde_json::json!({}));
}

#[test]
fn miniapp_import_bundle_plan_preserves_invalid_meta_error_classification() {
    let error = build_import_bundle_plan("new-app", "{", 1234).unwrap_err();

    assert!(matches!(error, MiniAppImportBundlePlanError::InvalidMeta(_)));
    assert!(error.to_string().starts_with("Invalid meta.json:"));
}
