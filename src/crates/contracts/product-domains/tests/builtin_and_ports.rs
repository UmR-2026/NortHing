#![cfg(feature = "miniapp")]

//! Builtin And Ports tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn miniapp_builtin_contract_preserves_seed_marker_and_hash_policy() {
    let app = BuiltinMiniAppBundle {
        id: "builtin-demo",
        version: 2,
        meta_json: r#"{"id":"builtin-demo"}"#,
        html: "<!doctype html><html></html>",
        css: "body { color: red; }",
        ui_js: r#"console.log("ui");"#,
        worker_js: r#"console.log("worker");"#,
        esm_dependencies_json: "[]",
    };
    let content_hash = builtin_content_hash(&app);

    assert_eq!(BUILTIN_INSTALL_MARKER, ".builtin-manifest.json");
    assert_eq!(LEGACY_BUILTIN_VERSION_MARKER, ".builtin-version");
    assert_eq!(
        content_hash,
        "sha256:5a2625011813ed9f39eea6875ab96047eb383ac005298ea86ce68e5ac4e79825"
    );

    assert!(should_seed_builtin_app(&app, &content_hash, None));
    assert!(!should_seed_builtin_app(
        &app,
        &content_hash,
        Some(&BuiltinInstallMarker {
            version: 2,
            hash: content_hash.clone(),
        }),
    ));
    assert!(should_seed_builtin_app(
        &app,
        &content_hash,
        Some(&BuiltinInstallMarker {
            version: 1,
            hash: content_hash.clone(),
        }),
    ));
    assert!(should_seed_builtin_app(
        &app,
        &content_hash,
        Some(&BuiltinInstallMarker {
            version: 3,
            hash: "sha256:old".to_string(),
        }),
    ));

    let package = build_builtin_package_json(app.id);
    assert_eq!(package["name"], "miniapp-builtin-demo");
    assert_eq!(package["private"], true);
    assert_eq!(package["dependencies"], serde_json::json!({}));

    let source_files = builtin_source_files(&app);
    assert_eq!(
        source_files,
        [
            (INDEX_HTML, app.html),
            (STYLE_CSS, app.css),
            (UI_JS, app.ui_js),
            (WORKER_JS, app.worker_js),
            (ESM_DEPS_JSON, app.esm_dependencies_json),
        ]
    );
    assert_eq!(
        BUILTIN_PLACEHOLDER_COMPILED_HTML,
        "<!DOCTYPE html><html><body>Loading...</body></html>"
    );
}

#[test]
fn miniapp_builtin_contract_owns_seed_plan_and_marker_wire_shape() {
    let app = BuiltinMiniAppBundle {
        id: "builtin-demo",
        version: 7,
        meta_json: r#"{"id":"builtin-demo"}"#,
        html: "<!doctype html><html></html>",
        css: "body { color: red; }",
        ui_js: r#"console.log("ui");"#,
        worker_js: r#"console.log("worker");"#,
        esm_dependencies_json: "[]",
    };
    let artifacts = northhing_product_domains::miniapp::builtin::build_builtin_seed_artifacts(&app);
    let marker = build_builtin_install_marker(&app, &artifacts.content_hash);

    assert_eq!(artifacts.marker, marker);
    assert_eq!(artifacts.legacy_version, "7");
    assert_eq!(legacy_builtin_version_marker_content(&app), "7");
    assert_eq!(resolve_builtin_seed_check(&app, Some(&marker)), BuiltinSeedCheck::Skip);

    let stale_marker = BuiltinInstallMarker {
        version: 7,
        hash: "sha256:stale".to_string(),
    };
    assert_eq!(
        resolve_builtin_seed_check(&app, Some(&stale_marker)),
        BuiltinSeedCheck::NeedsSeed(artifacts.clone())
    );
    assert_eq!(
        resolve_builtin_seed_check(&app, None),
        BuiltinSeedCheck::NeedsSeed(artifacts.clone())
    );
    assert_eq!(
        resolve_builtin_seed_action(artifacts.clone(), true),
        BuiltinSeedAction::PreserveLocalOverride(artifacts.clone())
    );
    assert_eq!(
        resolve_builtin_seed_action(artifacts.clone(), false),
        BuiltinSeedAction::SeedBundle(artifacts.clone())
    );

    let serialized = serialize_builtin_install_marker(&marker).unwrap();
    assert_eq!(
        serialized,
        format!("{{\n  \"version\": 7,\n  \"hash\": \"{}\"\n}}", artifacts.content_hash)
    );
    assert_eq!(parse_builtin_install_marker(&serialized).unwrap(), marker);
}

#[test]
fn miniapp_builtin_contract_owns_seed_meta_timestamp_policy() {
    let app = BuiltinMiniAppBundle {
        id: "builtin-demo",
        version: 7,
        meta_json: r#"{
            "id": "template-id",
            "name": "Built in",
            "description": "Demo",
            "icon": "box",
            "category": "tools",
            "version": 7,
            "created_at": 1,
            "updated_at": 2
        }"#,
        html: "<!doctype html><html></html>",
        css: "",
        ui_js: "",
        worker_js: "",
        esm_dependencies_json: "[]",
    };

    let fresh_meta = build_builtin_seed_meta(&app, None, 1000).unwrap();
    assert_eq!(fresh_meta.id, "builtin-demo");
    assert_eq!(fresh_meta.created_at, 1000);
    assert_eq!(fresh_meta.updated_at, 1000);

    let existing_meta = r#"{
        "id": "builtin-demo",
        "name": "Existing",
        "description": "Existing",
        "icon": "box",
        "category": "tools",
        "version": 6,
        "created_at": 123,
        "updated_at": 456
    }"#;
    assert_eq!(preserved_builtin_created_at(Some(existing_meta)), Some(123));
    assert_eq!(preserved_builtin_created_at(Some("{not json")), None);
    assert_eq!(preserved_builtin_created_at(None), None);

    let updated_meta = build_builtin_seed_meta(&app, preserved_builtin_created_at(Some(existing_meta)), 2000).unwrap();
    assert_eq!(updated_meta.id, "builtin-demo");
    assert_eq!(updated_meta.name, "Built in");
    assert_eq!(updated_meta.created_at, 123);
    assert_eq!(updated_meta.updated_at, 2000);
}

#[test]
fn miniapp_ports_keep_runtime_boundary_lightweight() {
    let decoded: MiniAppInstallDepsRequest = serde_json::from_value(serde_json::json!({
        "appId": "demo",
        "dependencies": [{"name": "lodash", "version": "^4.17.21"}]
    }))
    .unwrap();
    assert_eq!(decoded.app_id, "demo");
    assert_eq!(decoded.dependencies[0].name, "lodash");

    let request = MiniAppInstallDepsRequest {
        app_id: "demo".to_string(),
        dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["appId"], "demo");
    assert!(json.get("appDir").is_none());
    assert_eq!(json["dependencies"][0]["name"], "lodash");

    let error = MiniAppPortError::new(MiniAppPortErrorKind::RuntimeUnavailable, "missing node");
    let json = serde_json::to_value(error).unwrap();
    assert_eq!(json["kind"], "runtime_unavailable");
    assert_eq!(json["message"], "missing node");

    let port: &dyn MiniAppRuntimePort = &RuntimePortStub;
    let _future = port.detect_runtime();
}
