use std::process::Command;

#[test]
fn binary_prints_help_with_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_plan-compliance-checker"))
        .arg("--help")
        .output()
        .expect("failed to run binary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--task"));
    assert!(stdout.contains("--start-sha"));
}

#[test]
fn binary_rejects_missing_plan_path() {
    let out = Command::new(env!("CARGO_BIN_EXE_plan-compliance-checker"))
        .output()
        .expect("failed to run binary");
    assert!(!out.status.success());
}

#[test]
fn command_runner_returns_exit_code() {
    use plan_compliance_checker::command_runner::run_command;
    use std::path::Path;

    let cwd = Path::new(".");
    // successful command
    let (code, stdout) = run_command(cwd, "echo", &["hello"]).expect("should run");
    assert_eq!(code, 0);
    assert!(stdout.contains("hello"));

    // failing command
    let (code, _) = run_command(cwd, "false", &[]).expect("should run false");
    assert_ne!(code, 0);
}
