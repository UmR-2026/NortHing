use plan_compliance_checker::plan::Plan;
use plan_compliance_checker::task::{TaskResult, check_plan};
use std::path::PathBuf;

#[test]
fn check_returns_pending_when_no_commits() {
    let cwd = std::env::current_dir().unwrap();
    let plan = Plan::default(); // empty plan
    let results = check_plan(&plan, &cwd).unwrap();
    assert!(results.is_empty());
}

#[test]
fn check_reports_missing_create_path() {
    use plan_compliance_checker::plan::{FilesSpec, Task};
    let cwd = std::env::current_dir().unwrap();
    let plan = Plan {
        path: PathBuf::from("test.md"),
        title: "t".into(),
        tasks: vec![Task {
            id: "1.1".into(),
            title: "scaffold".into(),
            files: FilesSpec {
                create: vec![PathBuf::from("nonexistent-file-xyz.rs")],
                modify: vec![],
            },
            steps: vec![],
        }],
        plan_start_sha: "HEAD".into(),
    };
    let results = check_plan(&plan, &cwd).unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], TaskResult::Pending { .. }));
}
