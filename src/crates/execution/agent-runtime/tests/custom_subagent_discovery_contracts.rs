use northhing_agent_runtime::custom_subagent::{
    custom_subagent_possible_dirs, custom_subagent_save_markdown_file, load_custom_subagent_definitions,
    CustomSubagentDefinition, CustomSubagentDiscoveryRoots, CustomSubagentKind,
};
use northhing_test_support::TestTempDir;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn custom_subagent_discovery_preserves_directory_priority_and_deduplication() {
    let workspace = TestTempDir::new("northhing-runtime-subagent-workspace");
    let northhing_user = TestTempDir::new("northhing-runtime-subagent-user");
    let home = TestTempDir::new("northhing-runtime-subagent-home");

    let project_northhing = workspace.path().join(".northhing").join("agents");
    let project_claude = workspace.path().join(".claude").join("agents");
    let user_northhing = northhing_user.path().join("agents");
    let home_claude = home.path().join(".claude").join("agents");
    fs::create_dir_all(&project_northhing).expect("project northhing agents dir should be created");
    fs::create_dir_all(&project_claude).expect("project claude agents dir should be created");
    fs::create_dir_all(&user_northhing).expect("user northhing agents dir should be created");
    fs::create_dir_all(&home_claude).expect("home claude agents dir should be created");

    write_agent(
        &project_northhing.join("shared.md"),
        "Shared",
        "Project northhing agent",
        CustomSubagentKind::Project,
    );
    write_agent(
        &project_claude.join("shared.md"),
        "Shared",
        "Project Claude duplicate",
        CustomSubagentKind::Project,
    );
    write_agent(
        &user_northhing.join("user-only.md"),
        "UserOnly",
        "northhing user agent",
        CustomSubagentKind::User,
    );
    write_agent(
        &home_claude.join("home-only.md"),
        "HomeOnly",
        "Claude user agent",
        CustomSubagentKind::User,
    );
    fs::write(project_northhing.join("ignored.txt"), "ignored").expect("ignored text file should be written");
    fs::create_dir_all(project_northhing.join("nested")).expect("nested dir should be created");
    write_agent(
        &project_northhing.join("nested").join("nested.md"),
        "Nested",
        "Nested project agent",
        CustomSubagentKind::Project,
    );

    let roots = CustomSubagentDiscoveryRoots {
        workspace_root: workspace.path().to_path_buf(),
        northhing_user_agents_dir: Some(user_northhing.clone()),
        home_dir: Some(home.path().to_path_buf()),
    };

    let dirs = custom_subagent_possible_dirs(&roots);
    assert_eq!(
        dirs.iter().map(|entry| entry.path.as_path()).collect::<Vec<_>>(),
        vec![
            project_northhing.as_path(),
            project_claude.as_path(),
            user_northhing.as_path(),
            home_claude.as_path(),
        ]
    );
    assert_eq!(
        dirs.iter().map(|entry| entry.kind).collect::<Vec<_>>(),
        vec![
            CustomSubagentKind::Project,
            CustomSubagentKind::Project,
            CustomSubagentKind::User,
            CustomSubagentKind::User,
        ]
    );

    let report = load_custom_subagent_definitions(&roots);
    assert!(report.errors.is_empty());
    assert_eq!(
        report
            .definitions
            .iter()
            .map(|loaded| loaded.definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Shared", "UserOnly", "HomeOnly"]
    );
    assert_eq!(report.definitions[0].definition.description, "Project northhing agent");
    assert_eq!(report.definitions[0].path, project_northhing.join("shared.md"));
}

#[test]
fn custom_subagent_discovery_reports_parse_errors_without_dropping_valid_files() {
    let workspace = TestTempDir::new("northhing-runtime-subagent-invalid");
    let project_northhing = workspace.path().join(".northhing").join("agents");
    fs::create_dir_all(&project_northhing).expect("project agents dir should be created");
    let broken_path = project_northhing.join("broken.md");
    fs::write(&broken_path, "No front matter").expect("broken markdown file should be written");
    write_agent(
        &project_northhing.join("valid.md"),
        "Valid",
        "Valid project agent",
        CustomSubagentKind::Project,
    );

    let roots = CustomSubagentDiscoveryRoots {
        workspace_root: workspace.path().to_path_buf(),
        northhing_user_agents_dir: None,
        home_dir: None,
    };

    let report = load_custom_subagent_definitions(&roots);
    assert_eq!(report.definitions.len(), 1);
    assert_eq!(report.definitions[0].definition.name, "Valid");
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].path, broken_path);
    assert_eq!(
        report.errors[0].error,
        "Failed to parse markdown file: Failed to capture content"
    );
}
fn write_agent(path: &Path, name: &str, description: &str, kind: CustomSubagentKind) {
    let definition = CustomSubagentDefinition::from_front_matter_fields(
        Some(name),
        Some(description),
        None,
        None,
        None,
        None,
        format!("{name} prompt."),
        kind,
    )
    .expect("custom subagent definition should be valid");
    custom_subagent_save_markdown_file(path, &definition).expect("custom subagent markdown should save");
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos()
        .to_string()
}
