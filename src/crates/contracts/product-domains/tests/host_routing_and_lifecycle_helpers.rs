#![cfg(feature = "miniapp")]

//! Host Routing And Lifecycle Helpers tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn miniapp_host_routing_preserves_existing_primitive_and_allowlist_contract() {
    assert_eq!(split_host_method("fs.readFile"), Some(("fs", "readFile")));
    assert_eq!(split_host_method("shell"), None);

    assert!(is_host_primitive("fs.readFile"));
    assert!(is_host_primitive("shell.exec"));
    assert!(is_host_primitive("os.info"));
    assert!(is_host_primitive("net.fetch"));
    assert!(!is_host_primitive("storage.get"));
    assert!(!is_host_primitive("custom.method"));
    assert!(!is_host_primitive("shell"));

    assert_eq!(
        command_basename_for_allowlist(r"C:\Program Files\Git\cmd\git.exe"),
        "git"
    );
    assert_eq!(command_basename_for_allowlist("git.exe"), "git");
    assert_eq!(command_basename_for_allowlist("/usr/bin/git"), "git");
    assert_eq!(command_basename_for_allowlist("CARGO"), "cargo");

    assert_eq!(fs_method_access_mode("readFile"), FsAccessMode::Read);
    assert_eq!(fs_method_access_mode("writeFile"), FsAccessMode::Write);
    assert_eq!(fs_method_access_mode("access").policy_key(), None);
    let policy = serde_json::json!({
        "fs": {
            "read": ["/workspace", "/tmp/granted"],
            "write": ["/workspace/out"]
        }
    });
    assert_eq!(
        fs_policy_scopes(&policy, FsAccessMode::Read),
        vec!["/workspace".to_string(), "/tmp/granted".to_string()]
    );
    assert!(fs_resolved_path_allowed(
        Path::new("/workspace/src/main.rs"),
        [PathBuf::from("/workspace")]
    ));
    assert!(!fs_resolved_path_allowed(
        Path::new("/workspaced/src/main.rs"),
        [PathBuf::from("/workspace")]
    ));

    let argv = vec!["git".to_string(), "status".to_string()];
    assert_eq!(shell_exec_first_token(Some(&argv), "node ignored.js"), "git");
    assert_eq!(shell_exec_first_token(None, " cargo test "), "cargo");
    assert!(shell_exec_input_is_empty(Some(&[]), ""));
    assert!(!shell_exec_input_is_empty(Some(&argv), ""));
    assert_eq!(
        shell_exec_cwd(Some("/explicit"), Some(Path::new("/workspace")), Path::new("/appdata")),
        PathBuf::from("/explicit")
    );
    assert_eq!(
        shell_exec_cwd(None, Some(Path::new("/workspace")), Path::new("/appdata")),
        PathBuf::from("/workspace")
    );
    assert_eq!(shell_exec_timeout_ms(None), 30_000);
    assert_eq!(shell_exec_timeout_ms(Some(8_000)), 8_000);
    assert_eq!(
        shell_exec_default_env(),
        [("GIT_TERMINAL_PROMPT", "0"), ("LC_ALL", "C")]
    );

    assert!(command_basename_allowed(&[], "git"));
    assert!(command_basename_allowed(&["Git".to_string()], "git"));
    assert!(!command_basename_allowed(&["cargo".to_string()], "git"));

    assert!(host_allowed_by_allowlist(&[], "api.example.com"));
    assert!(host_allowed_by_allowlist(&["*".to_string()], "api.example.com"));
    assert!(host_allowed_by_allowlist(
        &["example.com".to_string()],
        "api.example.com"
    ));
    assert!(host_allowed_by_allowlist(
        &["api.example.com".to_string()],
        "api.example.com"
    ));
    assert!(!host_allowed_by_allowlist(
        &["example.com".to_string()],
        "badexample.com"
    ));
}

#[test]
fn miniapp_host_fs_call_plans_preserve_existing_path_and_permission_contract() {
    let read = plan_fs_host_call(
        "readFile",
        &serde_json::json!({ "path": "/workspace/read.txt", "encoding": "base64" }),
    )
    .expect("readFile should plan");
    assert_eq!(
        read,
        MiniAppFsHostCallPlan::ReadFile {
            path: PathBuf::from("/workspace/read.txt"),
            encoding_base64: true,
        }
    );
    assert_eq!(
        read.path_checks(),
        vec![MiniAppFsHostPathCheck {
            path: PathBuf::from("/workspace/read.txt"),
            mode: FsAccessMode::Read,
            denied_prefix: "Path",
        }]
    );

    let write = plan_fs_host_call(
        "writeFile",
        &serde_json::json!({ "p": "/workspace/out.txt", "data": "hello" }),
    )
    .expect("legacy p alias should plan");
    assert_eq!(
        write,
        MiniAppFsHostCallPlan::WriteFile {
            path: PathBuf::from("/workspace/out.txt"),
            data: "hello".to_string(),
        }
    );
    assert_eq!(
        write.path_checks(),
        vec![MiniAppFsHostPathCheck {
            path: PathBuf::from("/workspace/out.txt"),
            mode: FsAccessMode::Write,
            denied_prefix: "Path",
        }]
    );

    let copy = plan_fs_host_call(
        "copyFile",
        &serde_json::json!({ "src": "/workspace/src.txt", "dst": "/workspace/dst.txt" }),
    )
    .expect("copyFile should plan source and destination checks");
    assert_eq!(
        copy.path_checks(),
        vec![
            MiniAppFsHostPathCheck {
                path: PathBuf::from("/workspace/src.txt"),
                mode: FsAccessMode::Read,
                denied_prefix: "src",
            },
            MiniAppFsHostPathCheck {
                path: PathBuf::from("/workspace/dst.txt"),
                mode: FsAccessMode::Write,
                denied_prefix: "dst",
            }
        ]
    );

    let rename = plan_fs_host_call(
        "rename",
        &serde_json::json!({ "oldPath": "/workspace/old.txt", "newPath": "/workspace/new.txt" }),
    )
    .expect("rename should plan write checks for old and new paths");
    assert_eq!(
        rename.path_checks(),
        vec![
            MiniAppFsHostPathCheck {
                path: PathBuf::from("/workspace/old.txt"),
                mode: FsAccessMode::Write,
                denied_prefix: "oldPath",
            },
            MiniAppFsHostPathCheck {
                path: PathBuf::from("/workspace/new.txt"),
                mode: FsAccessMode::Write,
                denied_prefix: "newPath",
            }
        ]
    );

    let access = plan_fs_host_call("access", &serde_json::json!({ "path": "/workspace/read.txt" }))
        .expect("access should plan without permission checks");
    assert!(access.path_checks().is_empty());

    assert_eq!(
        plan_fs_legacy_path_check("copyFile", &serde_json::json!({ "path": "/workspace/legacy.txt" })),
        Some(MiniAppFsHostPathCheck {
            path: PathBuf::from("/workspace/legacy.txt"),
            mode: FsAccessMode::Write,
            denied_prefix: "Path",
        })
    );
    assert_eq!(
        plan_fs_legacy_path_check("unknownMethod", &serde_json::json!({ "p": "/workspace/legacy.txt" })),
        Some(MiniAppFsHostPathCheck {
            path: PathBuf::from("/workspace/legacy.txt"),
            mode: FsAccessMode::Read,
            denied_prefix: "Path",
        })
    );
    assert_eq!(
        plan_fs_legacy_path_check("access", &serde_json::json!({ "path": "/workspace/a.txt" })),
        None
    );
}

#[test]
fn miniapp_host_fs_call_plans_preserve_existing_error_contract() {
    let missing_path = plan_fs_host_call("readFile", &serde_json::json!({})).unwrap_err();
    assert_eq!(missing_path.kind(), MiniAppHostPlanErrorKind::Parse);
    assert_eq!(missing_path.message(), "missing path");

    let missing_src = plan_fs_host_call("copyFile", &serde_json::json!({ "dst": "/workspace/dst.txt" })).unwrap_err();
    assert_eq!(missing_src.kind(), MiniAppHostPlanErrorKind::Parse);
    assert_eq!(missing_src.message(), "missing param: src");

    let unknown = plan_fs_host_call("chmod", &serde_json::json!({ "path": "/workspace/a.txt" })).unwrap_err();
    assert_eq!(unknown.kind(), MiniAppHostPlanErrorKind::Validation);
    assert_eq!(unknown.message(), "unknown fs method: chmod");
}

#[test]
fn miniapp_host_shell_call_plans_preserve_existing_input_and_default_contract() {
    let argv_plan = plan_shell_host_call(
        "exec",
        &serde_json::json!({
            "args": ["git", "rev-parse", "--is-inside-work-tree"],
            "command": "ignored when args exists",
            "cwd": "/workspace",
            "timeout": 8000
        }),
        Some(Path::new("/fallback-workspace")),
        Path::new("/appdata"),
    )
    .expect("argv shell.exec should plan");
    assert_eq!(
        argv_plan,
        MiniAppShellHostCallPlan {
            argv: Some(vec![
                "git".to_string(),
                "rev-parse".to_string(),
                "--is-inside-work-tree".to_string(),
            ]),
            command: "ignored when args exists".to_string(),
            first_token: "git".to_string(),
            cwd: PathBuf::from("/workspace"),
            timeout_ms: 8000,
        }
    );

    let command_plan = plan_shell_host_call(
        "exec",
        &serde_json::json!({ "command": " cargo test " }),
        Some(Path::new("/workspace")),
        Path::new("/appdata"),
    )
    .expect("command shell.exec should plan");
    assert_eq!(command_plan.argv, None);
    assert_eq!(command_plan.command, "cargo test");
    assert_eq!(command_plan.first_token, "cargo");
    assert_eq!(command_plan.cwd, PathBuf::from("/workspace"));
    assert_eq!(command_plan.timeout_ms, 30_000);

    let appdata_plan = plan_shell_host_call(
        "exec",
        &serde_json::json!({ "command": "git status" }),
        None,
        Path::new("/appdata"),
    )
    .expect("missing cwd should fall back to app data dir");
    assert_eq!(appdata_plan.cwd, PathBuf::from("/appdata"));
}

#[test]
fn miniapp_host_shell_call_plans_preserve_existing_error_contract() {
    let empty = plan_shell_host_call(
        "exec",
        &serde_json::json!({ "command": "   " }),
        Some(Path::new("/workspace")),
        Path::new("/appdata"),
    )
    .unwrap_err();
    assert_eq!(empty.kind(), MiniAppHostPlanErrorKind::Parse);
    assert_eq!(empty.message(), "empty command");

    let unknown = plan_shell_host_call(
        "spawn",
        &serde_json::json!({ "command": "git status" }),
        Some(Path::new("/workspace")),
        Path::new("/appdata"),
    )
    .unwrap_err();
    assert_eq!(unknown.kind(), MiniAppHostPlanErrorKind::Validation);
    assert_eq!(unknown.message(), "unknown shell method: spawn");
}

#[test]
fn miniapp_lifecycle_helpers_preserve_runtime_revision_contract() {
    let source = MiniAppSource {
        npm_dependencies: vec![
            NpmDep {
                name: "zeta".to_string(),
                version: "2.0.0".to_string(),
            },
            NpmDep {
                name: "alpha".to_string(),
                version: "^1.0.0".to_string(),
            },
        ],
        ..MiniAppSource::default()
    };

    assert_eq!(build_source_revision(3, 1234), "src:3:1234");
    assert_eq!(build_deps_revision(&source), "alpha@^1.0.0|zeta@2.0.0");

    let runtime = build_runtime_state(3, 1234, &source, true, true);
    assert_eq!(runtime.source_revision, "src:3:1234");
    assert_eq!(runtime.deps_revision, "alpha@^1.0.0|zeta@2.0.0");
    assert!(runtime.deps_dirty);
    assert!(runtime.worker_restart_required);
    assert!(!runtime.ui_recompile_required);

    let mut app = sample_miniapp_for_lifecycle(source);
    assert!(ensure_runtime_state(&mut app));
    assert_eq!(app.runtime.source_revision, "src:3:1234");
    assert_eq!(app.runtime.deps_revision, "alpha@^1.0.0|zeta@2.0.0");
    assert!(!ensure_runtime_state(&mut app));

    assert_eq!(
        build_worker_revision(&app, r#"{"fs":{}}"#),
        r#"src:3:1234::alpha@^1.0.0|zeta@2.0.0::{"fs":{}}"#
    );
    assert_eq!(
        workspace_dir_string(Some(Path::new("/tmp/workspace"))),
        "/tmp/workspace"
    );
    assert_eq!(workspace_dir_string(None), "");
}

#[test]
fn miniapp_lifecycle_manager_state_helpers_preserve_core_transitions() {
    let source = MiniAppSource {
        npm_dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
        ..MiniAppSource::default()
    };
    let mut app = sample_miniapp_for_lifecycle(source.clone());

    mark_deps_installed_state(&mut app);
    assert_eq!(app.runtime.source_revision, "src:3:1234");
    assert_eq!(app.runtime.deps_revision, "lodash@^4.17.21");
    assert!(!app.runtime.deps_dirty);
    assert!(app.runtime.worker_restart_required);

    assert!(clear_worker_restart_required_state(&mut app));
    assert!(!app.runtime.worker_restart_required);
    assert!(!clear_worker_restart_required_state(&mut app));

    apply_recompile_result(&mut app, "<html>fresh</html>".to_string(), 2000);
    assert_eq!(app.compiled_html, "<html>fresh</html>");
    assert_eq!(app.updated_at, 2000);
    assert!(!app.runtime.ui_recompile_required);
    assert_eq!(app.runtime.source_revision, "src:3:1234");

    let current = sample_miniapp_for_lifecycle(MiniAppSource::default());
    let rollback_target = sample_miniapp_for_lifecycle(source.clone());
    let rolled_back = prepare_rollback_app(&current, rollback_target, 3000);
    assert_eq!(rolled_back.version, current.version + 1);
    assert_eq!(rolled_back.updated_at, 3000);
    assert!(rolled_back.runtime.deps_dirty);
    assert!(rolled_back.runtime.worker_restart_required);
    assert_eq!(rolled_back.runtime.deps_revision, "lodash@^4.17.21");

    let synced = apply_sync_from_fs_result(&current, source, "<html>synced</html>".to_string(), 4000);
    assert_eq!(synced.version, current.version + 1);
    assert_eq!(synced.updated_at, 4000);
    assert_eq!(synced.compiled_html, "<html>synced</html>");
    assert!(synced.runtime.deps_dirty);
    assert!(synced.runtime.worker_restart_required);

    let mut imported = synced.clone();
    imported.runtime.worker_restart_required = false;
    imported.runtime.deps_dirty = false;
    apply_import_runtime_state(&mut imported);
    assert!(imported.runtime.deps_dirty);
    assert!(imported.runtime.worker_restart_required);
    assert_eq!(imported.runtime.source_revision, "src:4:4000");
    assert_eq!(imported.runtime.deps_revision, "lodash@^4.17.21");
}

#[test]
fn miniapp_lifecycle_create_and_update_helpers_preserve_manager_contract() {
    let source = MiniAppSource {
        css: "body { color: black; }".to_string(),
        ..MiniAppSource::default()
    };
    let ai_context = MiniAppAiContext {
        original_prompt: "build a dashboard".to_string(),
        conversation_id: Some("conversation-1".to_string()),
        iteration_history: vec!["created".to_string()],
    };

    let created = build_created_app(
        "app-1".to_string(),
        MiniAppCreateInput {
            name: "Demo".to_string(),
            description: "Demo app".to_string(),
            icon: "sparkles".to_string(),
            category: "tools".to_string(),
            tags: vec!["demo".to_string()],
            source: source.clone(),
            permissions: MiniAppPermissions::default(),
            ai_context: Some(ai_context.clone()),
        },
        "<html>created</html>".to_string(),
        1000,
    );

    assert_eq!(created.id, "app-1");
    assert_eq!(created.version, 1);
    assert_eq!(created.created_at, 1000);
    assert_eq!(created.updated_at, 1000);
    assert_eq!(created.compiled_html, "<html>created</html>");
    assert_eq!(
        created.ai_context.as_ref().unwrap().conversation_id,
        ai_context.conversation_id
    );
    assert_eq!(created.runtime.source_revision, "src:1:1000");
    assert!(!created.runtime.deps_dirty);
    assert!(created.runtime.worker_restart_required);
    assert!(created.i18n.is_none());

    let updated_source = MiniAppSource {
        css: "body { color: red; }".to_string(),
        npm_dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
        ..source
    };
    let updated_permissions = MiniAppPermissions {
        fs: Some(FsPermissions {
            read: Some(vec!["{workspace}".to_string()]),
            write: None,
        }),
        ..MiniAppPermissions::default()
    };
    let patch = MiniAppUpdatePatch {
        name: Some("Updated".to_string()),
        source: Some(updated_source.clone()),
        permissions: Some(updated_permissions.clone()),
        ..MiniAppUpdatePatch::default()
    };
    assert_eq!(patch.source_for_compile(&created).css, updated_source.css);
    assert!(patch.permissions_for_compile(&created).fs.is_some());

    let updated = apply_update_patch(&created, patch, "<html>updated</html>".to_string(), 2000);

    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.description, created.description);
    assert_eq!(updated.tags, created.tags);
    assert_eq!(
        updated.ai_context.as_ref().unwrap().original_prompt,
        "build a dashboard"
    );
    assert_eq!(updated.version, 2);
    assert_eq!(updated.created_at, 1000);
    assert_eq!(updated.updated_at, 2000);
    assert_eq!(updated.compiled_html, "<html>updated</html>");
    assert_eq!(updated.source.css, "body { color: red; }");
    assert_eq!(
        updated.permissions.fs.as_ref().unwrap().read.as_ref().unwrap()[0],
        "{workspace}"
    );
    assert_eq!(updated.runtime.source_revision, "src:2:2000");
    assert_eq!(updated.runtime.deps_revision, "lodash@^4.17.21");
    assert!(updated.runtime.deps_dirty);
    assert!(updated.runtime.worker_restart_required);
    assert!(!updated.runtime.ui_recompile_required);

    let metadata_only = apply_update_patch(
        &updated,
        MiniAppUpdatePatch {
            tags: Some(vec!["metadata".to_string()]),
            ..MiniAppUpdatePatch::default()
        },
        "<html>metadata</html>".to_string(),
        3000,
    );

    assert_eq!(metadata_only.version, 3);
    assert_eq!(metadata_only.updated_at, 3000);
    assert_eq!(metadata_only.tags, vec!["metadata".to_string()]);
    assert_eq!(metadata_only.runtime.source_revision, "src:2:2000");
    assert_eq!(metadata_only.runtime.deps_revision, "lodash@^4.17.21");
    assert!(metadata_only.runtime.deps_dirty);
    assert!(metadata_only.runtime.worker_restart_required);
    assert!(!metadata_only.runtime.ui_recompile_required);
}

#[test]
fn miniapp_lifecycle_draft_helpers_preserve_manager_contract() {
    let mut active = sample_miniapp_for_lifecycle(MiniAppSource {
        css: "body { color: black; }".to_string(),
        ..MiniAppSource::default()
    });
    active.runtime = build_runtime_state(active.version, active.updated_at, &active.source, false, false);

    let prepared = prepare_draft_app(active.clone(), "<html>draft</html>".to_string(), 2000);

    assert_eq!(prepared.version, active.version);
    assert_eq!(prepared.source.css, "body { color: black; }");
    assert_eq!(prepared.updated_at, 2000);
    assert_eq!(prepared.compiled_html, "<html>draft</html>");
    assert_eq!(prepared.runtime.source_revision, "src:3:1234");
    assert!(!prepared.runtime.worker_restart_required);

    let mut draft_from_fs = prepared.clone();
    draft_from_fs.source = MiniAppSource {
        css: "body { background: white; }".to_string(),
        npm_dependencies: vec![NpmDep {
            name: "lodash".to_string(),
            version: "^4.17.21".to_string(),
        }],
        ..MiniAppSource::default()
    };
    let synced = apply_draft_source_sync_result(draft_from_fs, "<html>synced</html>".to_string(), 3000);

    assert_eq!(synced.version, active.version);
    assert_eq!(synced.updated_at, 3000);
    assert_eq!(synced.source.css, "body { background: white; }");
    assert_eq!(synced.runtime.source_revision, "src:3:3000");
    assert_eq!(synced.runtime.deps_revision, "lodash@^4.17.21");
    assert!(synced.runtime.deps_dirty);
    assert!(synced.runtime.worker_restart_required);

    let updated_permissions = MiniAppPermissions {
        fs: Some(FsPermissions {
            read: None,
            write: Some(vec!["{workspace}".to_string()]),
        }),
        ..MiniAppPermissions::default()
    };
    let permissioned = apply_draft_permission_update_result(
        synced.clone(),
        updated_permissions,
        "<html>permissioned</html>".to_string(),
        4000,
    );

    assert_eq!(permissioned.version, active.version);
    assert_eq!(permissioned.updated_at, 4000);
    assert!(permissioned.permissions.fs.as_ref().unwrap().write.is_some());
    assert_eq!(permissioned.runtime.source_revision, "src:3:4000");
    assert!(permissioned.runtime.worker_restart_required);

    let mut draft_to_apply = permissioned;
    draft_to_apply.name = "Draft name".to_string();
    draft_to_apply.description = "Draft description".to_string();
    draft_to_apply.i18n = Some(MiniAppI18n::default());

    let applied = apply_draft_to_active(&active, draft_to_apply, "<html>applied</html>".to_string(), 5000);

    assert_eq!(applied.id, active.id);
    assert_eq!(applied.created_at, active.created_at);
    assert_eq!(applied.version, active.version + 1);
    assert_eq!(applied.updated_at, 5000);
    assert_eq!(applied.name, "Draft name");
    assert_eq!(applied.description, "Draft description");
    assert_eq!(applied.compiled_html, "<html>applied</html>");
    assert!(applied.i18n.is_some());
    assert_eq!(applied.runtime.source_revision, "src:4:5000");
    assert!(applied.runtime.deps_dirty);
    assert!(applied.runtime.worker_restart_required);
}
