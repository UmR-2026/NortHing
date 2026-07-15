use plan_compliance_checker::plan::{ExpectedOutcome, FilesSpec, Plan};

#[test]
fn empty_files_spec_is_default() {
    let f = FilesSpec::default();
    assert!(f.create.is_empty());
    assert!(f.modify.is_empty());
}

#[test]
fn expected_outcome_pass_serializes_to_pass() {
    let s = serde_json::to_string(&ExpectedOutcome::Pass).unwrap();
    assert_eq!(s, "\"Pass\"");
}

#[test]
fn plan_default_has_empty_tasks() {
    let p = Plan::default();
    assert!(p.tasks.is_empty());
}
