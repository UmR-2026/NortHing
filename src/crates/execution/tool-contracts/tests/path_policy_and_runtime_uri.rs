//! Group 3: runtime_restrictions_snake_case_wire_shape, path_resolution,
//! tool_path_policy, tool_path_resolution, tool_path_absolute, runtime_uri,
//! runtime_artifact_reference tests.

mod common;
use common::*;
use serde_json::json;

#[test]
fn runtime_restrictions_keep_current_snake_case_wire_shape() {
    let value = json!({
        "allowed_tool_names": ["Read"],
        "denied_tool_names": ["Write"],
        "path_policy": {
            "write_roots": ["src"],
            "edit_roots": ["docs"],
            "delete_roots": ["target/generated"]
        }
    });

    let restrictions: ToolRuntimeRestrictions =
        serde_json::from_value(value.clone()).expect("deserialize restrictions");
    assert!(restrictions.is_tool_allowed("Read"));
    assert!(!restrictions.is_tool_allowed("Write"));
    assert_eq!(restrictions.path_policy.write_roots, vec!["src"]);
    assert_eq!(restrictions.path_policy.edit_roots, vec!["docs"]);
    assert_eq!(restrictions.path_policy.delete_roots, vec!["target/generated"]);

    let round_trip = serde_json::to_value(&restrictions).expect("serialize restrictions");
    assert_eq!(round_trip, value);
}

#[test]
fn path_resolution_contract_keeps_backend_and_runtime_helpers() {
    let remote = ToolPathResolution {
        requested_path: "src/lib.rs".to_string(),
        logical_path: "/workspace/src/lib.rs".to_string(),
        resolved_path: "/workspace/src/lib.rs".to_string(),
        backend: ToolPathBackend::RemoteWorkspace,
        runtime_scope: None,
        runtime_root: None,
    };
    assert!(remote.uses_remote_workspace_backend());
    assert!(!remote.is_runtime_artifact());

    let runtime_root = PathBuf::from("/runtime/workspace");
    let runtime = ToolPathResolution {
        requested_path: "northhing://runtime/workspace-1/logs/tool.txt".to_string(),
        logical_path: "northhing://runtime/workspace-1/logs/tool.txt".to_string(),
        resolved_path: runtime_root.join("logs").join("tool.txt").display().to_string(),
        backend: ToolPathBackend::Local,
        runtime_scope: Some("workspace-1".to_string()),
        runtime_root: Some(runtime_root.clone()),
    };

    assert!(!runtime.uses_remote_workspace_backend());
    assert!(runtime.is_runtime_artifact());
    assert_eq!(
        runtime.logical_child_path(&runtime_root.join("logs").join("tool.txt")),
        Some("northhing://runtime/workspace-1/logs/tool.txt".to_string())
    );
    assert_eq!(runtime.logical_child_path(&PathBuf::from("/outside/tool.txt")), None);
}

#[test]
fn tool_path_policy_owner_matches_resolved_roots_by_backend() {
    let target = ToolPathResolution {
        requested_path: "src/lib.rs".to_string(),
        logical_path: "/workspace/src/lib.rs".to_string(),
        resolved_path: "/workspace/src/lib.rs".to_string(),
        backend: ToolPathBackend::RemoteWorkspace,
        runtime_scope: None,
        runtime_root: None,
    };
    let local_root = ToolPathResolution {
        requested_path: "src".to_string(),
        logical_path: "/workspace/src".to_string(),
        resolved_path: "/workspace/src".to_string(),
        backend: ToolPathBackend::Local,
        runtime_scope: None,
        runtime_root: None,
    };
    let remote_root = ToolPathResolution {
        requested_path: "src".to_string(),
        logical_path: "/workspace/src".to_string(),
        resolved_path: "/workspace/src".to_string(),
        backend: ToolPathBackend::RemoteWorkspace,
        runtime_scope: None,
        runtime_root: None,
    };

    let allowed = is_tool_path_allowed_by_resolved_roots(
        &target,
        &[local_root, remote_root],
        |resolution, root| -> Result<bool, ()> {
            Ok(is_remote_posix_path_within_root(
                &resolution.resolved_path,
                &root.resolved_path,
            ))
        },
    )
    .expect("containment callback should succeed");

    assert!(allowed);
}

#[test]
fn tool_path_policy_owner_ignores_mismatched_backend_roots() {
    let target = ToolPathResolution {
        requested_path: "src/lib.rs".to_string(),
        logical_path: "/workspace/src/lib.rs".to_string(),
        resolved_path: "/workspace/src/lib.rs".to_string(),
        backend: ToolPathBackend::RemoteWorkspace,
        runtime_scope: None,
        runtime_root: None,
    };
    let local_root = ToolPathResolution {
        requested_path: "src".to_string(),
        logical_path: "/workspace/src".to_string(),
        resolved_path: "/workspace/src".to_string(),
        backend: ToolPathBackend::Local,
        runtime_scope: None,
        runtime_root: None,
    };

    let allowed = is_tool_path_allowed_by_resolved_roots(&target, &[local_root], |_, _| -> Result<bool, ()> {
        panic!("mismatched backend roots must not call the containment callback");
    })
    .expect("backend mismatch should not invoke containment");

    assert!(!allowed);
}

#[test]
fn tool_path_policy_owner_preserves_denial_message() {
    let message = build_tool_path_policy_denial_message(
        "/workspace/blocked/file.txt",
        ToolPathOperation::Write,
        &["/workspace/allowed".to_string()],
    );

    assert_eq!(
        message,
        "Path '/workspace/blocked/file.txt' is not allowed for write. Allowed roots: /workspace/allowed"
    );
}

#[test]
fn tool_path_resolution_owner_preserves_runtime_uri_scope_and_backend() {
    let runtime_root = PathBuf::from("/runtime/workspace");

    let resolution = resolve_tool_path_with_context(
        "northhing://runtime/workspace-123/plans/demo.plan.md",
        Some("/home/project"),
        true,
        Some("workspace-123"),
        Some(runtime_root.clone()),
    )
    .expect("runtime URI should resolve through the provider-neutral owner");

    assert_eq!(
        resolution.requested_path,
        "northhing://runtime/workspace-123/plans/demo.plan.md"
    );
    assert_eq!(
        resolution.logical_path,
        "northhing://runtime/workspace-123/plans/demo.plan.md"
    );
    assert_eq!(
        PathBuf::from(&resolution.resolved_path),
        runtime_root.join("plans").join("demo.plan.md")
    );
    assert_eq!(resolution.backend, ToolPathBackend::Local);
    assert_eq!(resolution.runtime_scope.as_deref(), Some("workspace-123"));
    assert_eq!(resolution.runtime_root.as_deref(), Some(runtime_root.as_path()));
}

#[test]
fn tool_path_resolution_owner_rejects_mismatched_runtime_scope() {
    let err = resolve_tool_path_with_context(
        "northhing://runtime/workspace-456/plans/demo.plan.md",
        Some("/home/project"),
        true,
        Some("workspace-123"),
        Some(PathBuf::from("/runtime/workspace")),
    )
    .expect_err("runtime artifact scopes must match the active workspace");

    assert_eq!(
        err.to_string(),
        "Runtime URI scope 'workspace-456' does not match the current workspace"
    );
}

#[test]
fn tool_path_resolution_owner_selects_workspace_backend_semantics() {
    let local = resolve_tool_path_with_context("src/lib.rs", Some("/repo/project"), false, None, None)
        .expect("local path should resolve through host semantics");
    assert_eq!(local.backend, ToolPathBackend::Local);
    assert_eq!(
        PathBuf::from(local.resolved_path),
        PathBuf::from("/repo/project").join("src").join("lib.rs")
    );

    let remote = resolve_tool_path_with_context(r"src\lib.rs", Some("/home/project"), true, None, None)
        .expect("remote path should resolve through POSIX workspace semantics");
    assert_eq!(remote.backend, ToolPathBackend::RemoteWorkspace);
    assert_eq!(remote.resolved_path, "/home/project/src/lib.rs");
    assert_eq!(remote.logical_path, "/home/project/src/lib.rs");
}

#[test]
fn tool_path_absolute_contract_keeps_remote_posix_and_runtime_uri_semantics() {
    assert!(tool_path_is_effectively_absolute(
        "northhing://runtime/current/logs/tool.txt",
        false
    ));
    assert!(tool_path_is_effectively_absolute(r"\home\workspace\src\lib.rs", true));
    assert!(!tool_path_is_effectively_absolute("src/lib.rs", true));
    assert_eq!(
        tool_path_is_effectively_absolute("src/lib.rs", false),
        PathBuf::from("src/lib.rs").is_absolute()
    );
}

#[test]
fn runtime_uri_contract_is_provider_neutral_and_normalized() {
    let uri = build_northhing_runtime_uri("workspace-123", r"plans\demo.plan.md").expect("runtime URI should build");

    assert_eq!(uri, "northhing://runtime/workspace-123/plans/demo.plan.md");
    assert!(is_northhing_runtime_uri(&uri));

    let parsed = parse_northhing_runtime_uri(&uri).expect("runtime URI should parse");
    assert_eq!(parsed.workspace_scope, "workspace-123");
    assert_eq!(parsed.relative_path, "plans/demo.plan.md");
    assert_eq!(
        normalize_runtime_relative_path("/sessions/turn-1/result.json").expect("relative path should normalize"),
        "sessions/turn-1/result.json"
    );
}

#[test]
fn runtime_uri_contract_rejects_escape_and_invalid_scope() {
    let escape = build_northhing_runtime_uri("workspace-123", "../secret.txt")
        .expect_err("runtime URI should reject parent directory escape");
    assert_eq!(escape.to_string(), "Runtime artifact path cannot escape its root");

    let empty_scope = build_northhing_runtime_uri("  ", "logs/tool.txt").expect_err("scope should be required");
    assert_eq!(empty_scope.to_string(), "Runtime URI workspace scope cannot be empty");

    let unsupported = parse_northhing_runtime_uri("/tmp/result.txt").expect_err("non-runtime URI should fail");
    assert_eq!(unsupported.to_string(), "Unsupported runtime URI: /tmp/result.txt");
}

#[test]
fn runtime_artifact_reference_owner_preserves_remote_uri_shape() {
    let reference = build_tool_runtime_artifact_reference(r"plans\demo.plan.md", None, Some("workspace-123"), true)
        .expect("remote artifact reference should build as runtime URI");

    assert_eq!(reference, "northhing://runtime/workspace-123/plans/demo.plan.md");
}

#[test]
fn runtime_artifact_reference_owner_preserves_local_path_shape() {
    let runtime_root = PathBuf::from("/runtime/workspace");

    let reference = build_tool_runtime_artifact_reference(
        r"sessions\session-1\tool-results\result.json",
        Some(runtime_root.as_path()),
        None,
        false,
    )
    .expect("local artifact reference should build as host path");

    assert_eq!(
        PathBuf::from(reference),
        runtime_root
            .join("sessions")
            .join("session-1")
            .join("tool-results")
            .join("result.json")
    );
}

#[test]
fn runtime_artifact_reference_owner_preserves_session_prefix_and_rejects_escape() {
    let session_reference = build_tool_session_runtime_artifact_reference(
        "session-1",
        "tool-results/result.json",
        None,
        Some("workspace-123"),
        true,
    )
    .expect("session artifact reference should build");

    assert_eq!(
        session_reference,
        "northhing://runtime/workspace-123/sessions/session-1/tool-results/result.json"
    );

    let runtime_root = PathBuf::from("/runtime/workspace");
    let escape = build_tool_runtime_artifact_reference("../secret.txt", Some(runtime_root.as_path()), None, false)
        .expect_err("artifact references must not escape the runtime root");

    assert_eq!(escape.to_string(), "Runtime artifact path cannot escape its root");
}
