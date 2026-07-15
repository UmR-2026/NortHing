use plan_compliance_checker::path_resolver::{detect_path_mismatch, find_workspace_root};
use std::path::PathBuf;

#[test]
fn finds_workspace_root_from_subdir() {
    let cwd = std::env::current_dir().unwrap();
    let root = find_workspace_root(&cwd).expect("should find workspace root");
    assert!(root.join("Cargo.toml").exists(), "root should contain Cargo.toml");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        manifest.contains("[workspace]"),
        "Cargo.toml should be a workspace manifest"
    );
}

#[test]
fn detects_mismatch_when_path_missing() {
    let cwd = std::env::current_dir().unwrap();
    let root = find_workspace_root(&cwd).unwrap();
    // Pick a workspace member path that does NOT exist at the plan's path
    // but exists at the suggestion path (the second component IS the workspace root).
    let plan_path = PathBuf::from("crates/services/services-core/Cargo.toml");
    let m = detect_path_mismatch(&plan_path, &root);
    assert!(!m.exists_relative, "the path should not exist as-written");
    assert!(
        m.suggestion.is_some(),
        "should suggest the real path: src/crates/services/services-core/Cargo.toml"
    );
}
