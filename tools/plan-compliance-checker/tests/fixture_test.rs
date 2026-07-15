use plan_compliance_checker::plan::parse_plan;

fn fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}.md");
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture: {path}"))
}

#[test]
fn good_plan_parses_with_no_warnings() {
    let plan = parse_plan(&fixture("good-plan")).expect("should parse");
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(plan.tasks[0].id, "1.1");
    assert_eq!(plan.tasks[0].files.create.len(), 1);
}

#[test]
fn mismatch_plan_parses_and_path_does_not_exist() {
    use plan_compliance_checker::path_resolver::{detect_path_mismatch, find_workspace_root};
    let plan = parse_plan(&fixture("path-mismatch-plan")).expect("should parse");
    let cwd = std::env::current_dir().unwrap();
    let root = find_workspace_root(&cwd).unwrap();
    let m = detect_path_mismatch(&plan.tasks[0].files.create[0], &root);
    assert!(!m.exists_relative);
}

#[test]
fn missing_plan_path_is_unrecoverable() {
    use plan_compliance_checker::path_resolver::{detect_path_mismatch, find_workspace_root};
    let plan = parse_plan(&fixture("missing-file-plan")).expect("should parse");
    let cwd = std::env::current_dir().unwrap();
    let root = find_workspace_root(&cwd).unwrap();
    let m = detect_path_mismatch(&plan.tasks[0].files.create[0], &root);
    assert!(!m.exists_relative);
    assert!(m.suggestion.is_none(), "no suggestion when path is truly absent");
}

#[test]
fn bad_commit_plan_parses_cleanly() {
    let plan = parse_plan(&fixture("bad-commit-plan")).expect("should parse");
    assert_eq!(plan.tasks[0].id, "1.1");
}
