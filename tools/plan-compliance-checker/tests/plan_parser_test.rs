use plan_compliance_checker::plan::parse_plan;
use std::path::PathBuf;

const SAMPLE: &str = r#"# My Plan

> preamble

## Phase 1

### Task 1.1: scaffold

**Files:**
- Create: `crates/foo/Cargo.toml`

- [ ] **Step 1: Write the file**

```toml
[package]
name = "foo"
```

- [ ] **Step 2: Verify**

Run: `cargo check -p foo`
Expected: PASS
"#;

#[test]
fn parse_extracts_task_with_id_and_title() {
    let plan = parse_plan(SAMPLE).expect("should parse");
    assert_eq!(plan.title, "My Plan");
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(plan.tasks[0].id, "1.1");
    assert_eq!(plan.tasks[0].title, "scaffold");
}

#[test]
fn parse_extracts_create_files() {
    let plan = parse_plan(SAMPLE).expect("should parse");
    assert_eq!(plan.tasks[0].files.create.len(), 1);
    assert_eq!(plan.tasks[0].files.create[0], PathBuf::from("crates/foo/Cargo.toml"));
}

#[test]
fn parse_extracts_steps_with_verify_command() {
    let plan = parse_plan(SAMPLE).expect("should parse");
    let task = &plan.tasks[0];
    assert_eq!(task.steps.len(), 2);
    assert_eq!(task.steps[1].verify_command.as_deref(), Some("cargo check -p foo"));
    assert_eq!(
        task.steps[1].expected_outcome,
        plan_compliance_checker::plan::ExpectedOutcome::Pass
    );
}

#[test]
fn parse_extracts_real_task_id_from_heading() {
    let plan = parse_plan(SAMPLE).expect("should parse");
    assert_eq!(plan.tasks[0].id, "1.1", "id should be parsed from heading text");
}
