use plan_compliance_checker::git_inspector::commits_since;

#[test]
fn lists_commits_since_given_sha() {
    let cwd = std::env::current_dir().unwrap();
    let commits = commits_since(&cwd, "HEAD~3").expect("should list commits");
    assert!(commits.len() >= 3, "should list at least 3 commits back from HEAD~3");
}
