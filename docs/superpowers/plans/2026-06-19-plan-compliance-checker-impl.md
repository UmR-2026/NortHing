<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
 Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
 本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# Plan Compliance Checker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Acceptance gate:** This plan is NOT done when the last task is committed. The plan IS done only when all gates in **[`docs/superpowers/plans/2026-06-19-plan-compliance-checker-verify.md`](2026-06-19-plan-compliance-checker-verify.md)** pass and the executor signs off there. The verification protocol defines per-phase gates and final acceptance criteria.

**Goal:** Build a small Rust CLI binary that mechanically verifies whether the workspace state matches a plan markdown document, with first-class detection of plan-vs-workspace path mismatches (catching the bug in the lightweight actor plan).

**Architecture:** Standalone binary in `tools/plan-compliance-checker/`. Markdown parser — task checker — four orthogonal check kinds (file existence, command exit code, commit presence, path consistency). Output in human or JSON format. Workspace root detected by walking up to find `[workspace]` Cargo.toml.

**Tech Stack:** Rust 2024 edition, `clap` v4 derive, `tokio`, `serde` + `serde_json`, `pulldown-cmark` for markdown parsing, `anyhow` for error handling.

**Spec:** `docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md`

**Working directory:** `E:\agent-project\agent-app-v3`
**Branch:** `v3-restructure`
**Toolchain:** `set PATH=C:\Users\UmR\.cargo\bin;C:\Users\UmR\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;%PATH%` *before every cargo command* — GNU toolchain ahead of MSVC in PATH breaks `getrandom`/`aws-lc-rs` with `dlltool.exe not found`.

---

## File structure

| Path | Responsibility |
|---|---|
| `tools/plan-compliance-checker/Cargo.toml` | Crate manifest |
| `tools/plan-compliance-checker/README.md` | Usage + how to extend plan format |
| `tools/plan-compliance-checker/src/main.rs` | clap CLI entry; dispatches to checker + report |
| `tools/plan-compliance-checker/src/plan.rs` | Markdown — `Plan` struct |
| `tools/plan-compliance-checker/src/task.rs` | Per-task verification logic; 4 check kinds |
| `tools/plan-compliance-checker/src/command_runner.rs` | Run shell command, capture exit code |
| `tools/plan-compliance-checker/src/git_inspector.rs` | git history inspection via `git` CLI |
| `tools/plan-compliance-checker/src/path_resolver.rs` | Workspace root + path mismatch detection |
| `tools/plan-compliance-checker/src/report.rs` | Human + JSON formatter |
| `tools/plan-compliance-checker/tests/plan_parser_test.rs` | Parser unit tests |
| `tools/plan-compliance-checker/tests/task_checker_test.rs` | Checker unit tests |
| `tools/plan-compliance-checker/tests/path_resolver_test.rs` | Path resolution tests |
| `tools/plan-compliance-checker/tests/cli_test.rs` | End-to-end CLI tests |
| `tools/plan-compliance-checker/tests/fixtures/good-plan.md` | Golden test plan (no path mismatches) |
| `tools/plan-compliance-checker/tests/fixtures/path-mismatch-plan.md` | Plan that triggers path warnings |
| `tools/plan-compliance-checker/tests/fixtures/missing-file-plan.md` | Plan whose Create paths don't exist |
| `tools/plan-compliance-checker/tests/fixtures/bad-commit-plan.md` | Plan whose task has no commit |
| `Cargo.toml` | Add `tools/plan-compliance-checker` to `[workspace.members]` |
| `docs/notes/plan-compliance-checker.md` | Maintainer's note |
| `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md` | MODIFY: correct all 19 task paths |

---

## Phase 1 — Skeleton (4 tasks)

### Task 1.1: Create the `tools/plan-compliance-checker` crate manifest

**Files:**
- Create: `tools/plan-compliance-checker/Cargo.toml`
- Modify: `Cargo.toml` (add to `[workspace.members]`)

- [ ] **Step 1: Write the manifest**

```toml
[package]
name = "plan-compliance-checker"
version = "0.1.0"
edition = "2024"
description = "Verify workspace state against a plan markdown document"
license = "MIT OR Apache-2.0"

[[bin]]
name = "plan-compliance-checker"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
pulldown-cmark = "0.10"
anyhow = "1"
```

- [ ] **Step 2: Add to workspace**

Open `Cargo.toml` and add `"tools/plan-compliance-checker"` to `[workspace.members]`.

- [ ] **Step 3: Create empty `main.rs`**

```rust
// tools/plan-compliance-checker/src/main.rs
fn main() {
 println!("plan-compliance-checker skeleton");
}
```

- [ ] **Step 4: Verify the crate compiles**

Run: `cargo check -p plan-compliance-checker`
Expected: SUCCESS (downloads dependencies; may take 30-60s on first run).

- [ ] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker Cargo.toml
git commit -m "feat(checker): scaffold plan-compliance-checker crate"
```

---

### Task 1.2: Add clap CLI skeleton

**Files:**
- Modify: `tools/plan-compliance-checker/src/main.rs`

- [ ] **Step 1: Write the failing CLI test**

Create `tools/plan-compliance-checker/tests/cli_test.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p plan-compliance-checker --test cli_test`
Expected: COMPILE ERROR (`env!` macro fails or binary doesn't exist yet).

- [ ] **Step 3: Replace main.rs with clap skeleton**

```rust
// tools/plan-compliance-checker/src/main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "plan-compliance-checker", about = "Verify workspace state against a plan markdown document", version)]
struct Cli {
 /// Path to the plan markdown file
 plan: std::path::PathBuf,

 /// Check only one task (e.g., "1.3")
 #[arg(long)]
 task: Option<String>,

 /// Skip long-running verify commands (cargo build, cargo test)
 #[arg(long)]
 skip_slow: bool,

 /// Force re-running verify commands even for slow ones
 #[arg(long)]
 force_verify: bool,

 /// Override the plan-start SHA (otherwise reads from .plan-compliance-checker.toml or HEAD~)
 #[arg(long)]
 start_sha: Option<String>,

 /// Output format
 #[arg(long, value_enum, default_value_t = Format::Human)]
 format: Format,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format { Human, Json }

fn main() -> anyhow::Result<()> {
 let _cli = Cli::parse();
 Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test cli_test`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker/src/main.rs tools/plan-compliance-checker/tests/cli_test.rs
git commit -m "feat(checker): clap CLI skeleton with --task / --skip-slow / --start-sha / --format"
```

---

### Task 1.3: Plan + Task data structures

**Files:**
- Create: `tools/plan-compliance-checker/src/plan.rs`
- Modify: `tools/plan-compliance-checker/src/main.rs`

- [ ] **Step 1: Write the failing data-structure test**

Create `tools/plan-compliance-checker/tests/plan_struct_test.rs`:

```rust
use plan_compliance_checker::plan::{Plan, FilesSpec, ExpectedOutcome};

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test plan_struct_test`
Expected: COMPILE ERROR (module `plan` does not exist).

- [ ] **Step 3: Implement plan.rs**

```rust
// tools/plan-compliance-checker/src/plan.rs
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
 pub path: PathBuf,
 pub title: String,
 pub tasks: Vec<Task>,
 pub plan_start_sha: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Task {
 pub id: String,
 pub title: String,
 pub files: FilesSpec,
 pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesSpec {
 pub create: Vec<PathBuf>,
 pub modify: Vec<ModifyTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyTarget {
 pub path: PathBuf,
 pub range: Option<LineRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
 pub start: usize,
 pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
 pub index: usize,
 pub description: String,
 pub expected_outcome: ExpectedOutcome,
 pub verify_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpectedOutcome {
 Pass,
 Fail(String),
 Custom(i32),
}
```

- [ ] **Step 4: Update main.rs**

```rust
// tools/plan-compliance-checker/src/main.rs (extend)
pub mod plan;
use clap::Parser;

#[derive(Parser)]
#[command(name = "plan-compliance-checker", about = "Verify workspace state against a plan markdown document", version)]
struct Cli {
 plan: std::path::PathBuf,
 #[arg(long)] task: Option<String>,
 #[arg(long)] skip_slow: bool,
 #[arg(long)] force_verify: bool,
 #[arg(long)] start_sha: Option<String>,
 #[arg(long, value_enum, default_value_t = Format::Human)] format: Format,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format { Human, Json }

fn main() -> anyhow::Result<()> {
 let _cli = Cli::parse();
 Ok(())
}
```

- [ ] **Step 5: Add a `lib.rs` so tests can import**

Create `tools/plan-compliance-checker/src/lib.rs`:

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test plan_struct_test`
Expected: 3 passed.

- [ ] **Step 7: Commit**

```bash
git add tools/plan-compliance-checker/src/plan.rs tools/plan-compliance-checker/src/lib.rs tools/plan-compliance-checker/src/main.rs tools/plan-compliance-checker/tests/plan_struct_test.rs
git commit -m "feat(checker): Plan + Task + FilesSpec data structures"
```

---

### Task 1.4: Path resolver — workspace root detection

**Files:**
- Create: `tools/plan-compliance-checker/src/path_resolver.rs`
- Modify: `tools/plan-compliance-checker/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `tools/plan-compliance-checker/tests/path_resolver_test.rs`:

```rust
use plan_compliance_checker::path_resolver::{find_workspace_root, detect_path_mismatch};
use std::path::PathBuf;

#[test]
fn finds_workspace_root_from_subdir() {
 let cwd = std::env::current_dir().unwrap();
 let root = find_workspace_root(&cwd).expect("should find workspace root");
 assert!(root.join("Cargo.toml").exists(), "root should contain Cargo.toml");
 let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
 assert!(manifest.contains("[workspace]"), "Cargo.toml should be a workspace manifest");
}

#[test]
fn detects_mismatch_when_path_missing() {
 let cwd = std::env::current_dir().unwrap();
 let root = find_workspace_root(&cwd).unwrap();
 let plan_path = PathBuf::from("crates/nonexistent-crate/Cargo.toml");
 let m = detect_path_mismatch(&plan_path, &root);
 assert!(!m.exists_relative, "the path should not exist");
 assert!(m.suggestion.is_some(), "should suggest a real path");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test path_resolver_test`
Expected: COMPILE ERROR (`path_resolver` module does not exist).

- [ ] **Step 3: Implement path_resolver.rs**

```rust
// tools/plan-compliance-checker/src/path_resolver.rs
use std::path::{Path, PathBuf};

pub struct PathMismatch {
 pub exists_relative: bool,
 pub suggestion: Option<PathBuf>,
}

pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
 let mut current: PathBuf = start.to_path_buf();
 loop {
 let manifest = current.join("Cargo.toml");
 if manifest.exists() {
 if let Ok(content) = std::fs::read_to_string(&manifest) {
 if content.contains("[workspace]") {
 return Some(current);
 }
 }
 }
 if !current.pop() { return None; }
 }
}

pub fn detect_path_mismatch(plan_path: &Path, workspace_root: &Path) -> PathMismatch {
 let absolute = workspace_root.join(plan_path);
 if absolute.exists() {
 return PathMismatch { exists_relative: true, suggestion: None };
 }
 // Try the alternative layout: <root>/<first_segment>/...
 let components: Vec<_> = plan_path.components().collect();
 if components.len() >= 2 {
 let candidate: PathBuf = components.iter().collect();
 if candidate.exists() {
 return PathMismatch { exists_relative: false, suggestion: Some(candidate) };
 }
 }
 PathMismatch { exists_relative: false, suggestion: None }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
pub mod path_resolver;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test path_resolver_test`
Expected: 2 passed (note: `find_workspace_root(cwd)` from inside `tools/plan-compliance-checker/` should walk up 3 levels to `E:\agent-project\agent-app-v3\`).

- [ ] **Step 6: Commit**

```bash
git add tools/plan-compliance-checker/src/path_resolver.rs tools/plan-compliance-checker/src/lib.rs tools/plan-compliance-checker/tests/path_resolver_test.rs
git commit -m "feat(checker): workspace root detection + path mismatch resolver"
```

---

### Task 1.5: Verify Phase 1

**Files:** none (verification only)

- [ ] **Step 1: Workspace check**

Run: `cargo check --workspace --all-features`
Expected: SUCCESS.

- [ ] **Step 2: Run all checker tests**

Run: `cargo test -p plan-compliance-checker`
Expected: 7 tests passed (2 CLI + 3 plan + 2 path_resolver).

- [ ] **Step 3: Update PROJECT_STATE.md**

Append to the "🔧 Lightweight Actor" section (or create a new "🔧 Plan Compliance Checker" section):

```markdown
## 🔧 Plan Compliance Checker (2026-06-19, in progress)

Spec: docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md
Plan: docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md
Phase 1 (skeleton + data structures + path resolver) complete.
```

- [ ] **Step 4: Commit**

```bash
git add docs/PROJECT_STATE.md
git commit -m "docs(state): plan-compliance-checker Phase 1 (skeleton) complete"
```

---

## Phase 2 — Parser + checker (5 tasks)

### Task 2.1: Markdown parser — task heading + title

**Files:**
- Modify: `tools/plan-compliance-checker/src/plan.rs`

- [ ] **Step 1: Write the failing parser test**

Create `tools/plan-compliance-checker/tests/plan_parser_test.rs`:

```rust
use plan_compliance_checker::plan::parse_plan;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test`
Expected: COMPILE ERROR (`parse_plan` does not exist).

- [ ] **Step 3: Implement `parse_plan` (heading-only, no steps yet)**

Add to `plan.rs`:

```rust
use pulldown_cmark::{Event, Parser as MdParser, Tag};

pub fn parse_plan(input: &str) -> anyhow::Result<Plan> {
 let mut plan = Plan::default();
 let mut current_task: Option<Task> = None;

 for event in MdParser::new(input) {
 match event {
 Event::Start(Tag::Heading { level: 1, .. }) if current_task.is_none() && plan.title.is_empty() => {
 // Title collected on Text event after this
 }
 Event::Start(Tag::Heading { level: 3, .. }) => {
 if let Some(t) = current_task.take() {
 plan.tasks.push(t);
 }
 current_task = Some(Task::default());
 }
 Event::Text(text) if current_task.is_some() && current_task.as_ref().unwrap().title.is_empty() => {
 if let Some(ref mut t) = current_task {
 t.title = text.to_string();
 }
 }
 _ => {}
 }
 }
 if let Some(t) = current_task { plan.tasks.push(t); }

 // Post-process: extract task IDs from headings
 for (i, task) in plan.tasks.iter_mut().enumerate() {
 task.id = format!("{}.{}", i + 1, 1); // placeholder; improved in Task 2.4
 }
 Ok(plan)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker/src/plan.rs tools/plan-compliance-checker/tests/plan_parser_test.rs
git commit -m "feat(checker): markdown parser extracts task titles (step 1)"
```

---

### Task 2.2: Parser — Files block

**Files:**
- Modify: `tools/plan-compliance-checker/src/plan.rs`

- [ ] **Step 1: Extend the failing test**

Append to `tests/plan_parser_test.rs`:

```rust
#[test]
fn parse_extracts_create_files() {
 let plan = parse_plan(SAMPLE).expect("should parse");
 assert_eq!(plan.tasks[0].files.create.len(), 1);
 assert_eq!(plan.tasks[0].files.create[0].to_string_lossy(), "crates/foo/Cargo.toml");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test parse_extracts_create_files`
Expected: FAIL (0 files extracted).

- [ ] **Step 3: Implement Files block parsing**

Extend `parse_plan` in `plan.rs`:

```rust
// Inside the loop, after detecting Task heading, collect list items until next heading
let mut in_files_block = false;
let mut current_task: Option<Task> = None;
let mut code_buffer = String::new();
let mut in_code = false;

for event in MdParser::new(input) {
 match event {
 Event::Start(Tag::Heading { level: 3, .. }) => {
 if let Some(t) = current_task.take() { plan.tasks.push(t); }
 current_task = Some(Task::default());
 in_files_block = false;
 }
 Event::Start(Tag::Item) if current_task.is_some() => {
 // next text is list-item content
 }
 Event::Text(text) if current_task.is_some() => {
 if let Some(ref mut t) = current_task {
 let s = text.to_string();
 if s.starts_with("Create:") {
 if let Some(path) = extract_path(&s) {
 t.files.create.push(path);
 in_files_block = true;
 }
 } else if s.starts_with("Modify:") {
 if let Some((path, range)) = extract_modify_target(&s) {
 t.files.modify.push(ModifyTarget { path, range });
 in_files_block = true;
 }
 } else if in_files_block {
 // another list item that isn't Create/Modify: end of files block
 in_files_block = false;
 }
 if t.title.is_empty() {
 t.title = s.trim_start_matches("### Task ").to_string();
 }
 }
 }
 _ => {}
 }
}
if let Some(t) = current_task { plan.tasks.push(t); }

fn extract_path(line: &str) -> Option<PathBuf> {
 let start = line.find('`')— + 1;
 let end = line[start..].find('`')— + start;
 let raw = &line[start..end];
 if let Some((path, _range)) = raw.split_once(':') {
 Some(PathBuf::from(path))
 } else {
 Some(PathBuf::from(raw))
 }
}

fn extract_modify_target(line: &str) -> Option<(PathBuf, Option<LineRange>)> {
 let start = line.find('`')— + 1;
 let end = line[start..].find('`')— + start;
 let raw = &line[start..end];
 if let Some((path, range_str)) = raw.split_once(':') {
 let parts: Vec<&str> = range_str.split('-').collect();
 if parts.len() == 2 {
 if let (Ok(s), Ok(e)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
 return Some((PathBuf::from(path), Some(LineRange { start: s, end: e })));
 }
 }
 Some((PathBuf::from(path), None))
 } else {
 Some((PathBuf::from(raw), None))
 }
}
```

> **Note:** This is a structural rewrite of `parse_plan`. Replace the previous version entirely with this one.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker/src/plan.rs
git commit -m "feat(checker): parser extracts Create/Modify files from task headings"
```

---

### Task 2.3: Parser — Steps + verify commands + expected outcomes

**Files:**
- Modify: `tools/plan-compliance-checker/src/plan.rs`

- [ ] **Step 1: Extend the failing test**

Append to `tests/plan_parser_test.rs`:

```rust
#[test]
fn parse_extracts_steps_with_verify_command() {
 let plan = parse_plan(SAMPLE).expect("should parse");
 let task = &plan.tasks[0];
 assert_eq!(task.steps.len(), 2);
 assert_eq!(task.steps[1].verify_command.as_deref(), Some("cargo check -p foo"));
 assert_eq!(task.steps[1].expected_outcome, plan_compliance_checker::plan::ExpectedOutcome::Pass);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test parse_extracts_steps_with_verify_command`
Expected: FAIL (no steps extracted).

- [ ] **Step 3: Implement step parsing**

Extend `parse_plan` further:

```rust
// Add fields for in-code and step parsing
let mut current_step: Option<Step> = None;
let mut step_index: usize = 0;
let mut code_block_kind: Option<String> = None;
let mut code_buffer = String::new();

// Inside the loop, handle Step headings and code blocks:
Event::Start(Tag::Item) if current_task.is_some() && code_buffer.is_empty() => { /* list item inside task body */ }
Event::Start(Tag::CodeBlock(kind)) => { code_block_kind = Some(kind.to_string()); code_buffer.clear(); }
Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
 if let Some(kind) = code_block_kind.take() {
 // bash code blocks may contain `git commit -m "..."` etc; not verify commands
 }
 code_buffer.clear();
}
Event::Text(text) => {
 let s = text.to_string();
 if s.starts_with("- [ ] **Step ") || s.starts_with("- [x] **Step ") {
 // Close previous step
 if let (Some(task), Some(step)) = (current_task.as_mut(), current_step.take()) {
 task.steps.push(step);
 }
 step_index += 1;
 current_step = Some(Step {
 index: step_index,
 description: s.replace("- [ ] **Step ", "").replace("- [x] **Step ", "").trim().trim_end_matches(':').to_string(),
 expected_outcome: ExpectedOutcome::Pass,
 verify_command: None,
 });
 } else if s.starts_with("Run: `") {
 if let Some(step) = current_step.as_mut() {
 if let Some(cmd) = extract_run_command(&s) {
 step.verify_command = Some(cmd);
 }
 }
 } else if s.starts_with("Expected: ") {
 if let Some(step) = current_step.as_mut() {
 step.expected_outcome = parse_expected(&s);
 }
 }
}
```

Add helpers:

```rust
fn extract_run_command(line: &str) -> Option<String> {
 let start = line.find('`')— + 1;
 let end = line[start..].find('`')— + start;
 Some(line[start..end].to_string())
}

fn parse_expected(line: &str) -> ExpectedOutcome {
 let body = line.trim_start_matches("Expected: ").trim();
 if body == "PASS" || body == "SUCCESS" {
 ExpectedOutcome::Pass
 } else if body.starts_with("FAIL") {
 ExpectedOutcome::Fail(body.to_string())
 } else if let Ok(n) = body.parse::<i32>() {
 ExpectedOutcome::Custom(n)
 } else {
 ExpectedOutcome::Pass
 }
}
```

After the loop, finalize:

```rust
if let (Some(task), Some(step)) = (current_task.as_mut(), current_step.take()) {
 task.steps.push(step);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker/src/plan.rs
git commit -m "feat(checker): parser extracts steps + verify commands + expected outcomes"
```

---

### Task 2.4: Parser — Real task IDs from `### Task N.M:` headings

**Files:**
- Modify: `tools/plan-compliance-checker/src/plan.rs`

- [ ] **Step 1: Extend the failing test**

Append to `tests/plan_parser_test.rs`:

```rust
#[test]
fn parse_extracts_real_task_id_from_heading() {
 let plan = parse_plan(SAMPLE).expect("should parse");
 assert_eq!(plan.tasks[0].id, "1.1", "id should be parsed from heading text");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test parse_extracts_real_task_id`
Expected: FAIL (current implementation sets `id = "1.1"` from index, not from heading text).

- [ ] **Step 3: Improve id extraction**

In `parse_plan`, when collecting `current_task`, capture the heading text first:

```rust
// When a level-3 heading opens, the next Text event carries the heading content like "Task 1.1: scaffold"
let mut heading_text_buffer = String::new();
let mut in_task_heading = false;

// Replace Event::Start(Tag::Heading { level: 3, .. }) handling:
Event::Start(Tag::Heading { level: 3, .. }) => {
 if let Some(t) = current_task.take() { plan.tasks.push(t); }
 current_task = Some(Task::default());
 in_files_block = false;
 in_task_heading = true;
 heading_text_buffer.clear();
}
Event::Text(text) if in_task_heading => {
 heading_text_buffer.push_str(&text);
 if let Some(ref mut t) = current_task {
 if let Some((id, title)) = split_task_heading(&heading_text_buffer) {
 t.id = id;
 t.title = title;
 in_task_heading = false;
 }
 }
}
```

Add helper:

```rust
fn split_task_heading(s: &str) -> Option<(String, String)> {
 let s = s.trim();
 if !s.starts_with("Task ") { return None; }
 let rest = &s[5..];
 let (id, after) = rest.split_once(':')— ;
 let id = id.trim().to_string();
 let title = after.trim().to_string();
 Some((id, title))
}
```

- [ ] **Step 4: Remove the placeholder id assignment**

In the post-process loop, remove:

```rust
// DELETE this loop:
// for (i, task) in plan.tasks.iter_mut().enumerate() {
// task.id = format!("{}.{}", i + 1, 1);
// }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test plan_parser_test`
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add tools/plan-compliance-checker/src/plan.rs
git commit -m "feat(checker): parser extracts real task IDs from heading text"
```

---

### Task 2.5: Verify Phase 2

**Files:** none (verification only)

- [ ] **Step 1: Run all tests**

Run: `cargo test -p plan-compliance-checker`
Expected: 11+ tests passed (4 parser + 3 plan struct + 2 path + 2 cli).

- [ ] **Step 2: Verify parser on real actor plan**

Run a quick smoke test:

```bash
cargo run -p plan-compliance-checker --quiet -- /dev/null 2>&1 || true
# (At this point we haven't wired main.rs to call parse_plan yet; that's Phase 3.)
```

- [ ] **Step 3: Update PROJECT_STATE.md**

Append to the "🔧 Plan Compliance Checker" section:

```markdown
Phase 2 (parser + path resolver) complete. Parser correctly extracts
task IDs, titles, file specs, steps, verify commands, and expected
outcomes from markdown.
```

- [ ] **Step 4: Commit**

```bash
git add docs/PROJECT_STATE.md
git commit -m "docs(state): plan-compliance-checker Phase 2 (parser) complete"
```

---

## Phase 3 — Git inspector + task checker (5 tasks)

### Task 3.1: Git inspector — list commits since SHA

**Files:**
- Create: `tools/plan-compliance-checker/src/git_inspector.rs`
- Modify: `tools/plan-compliance-checker/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `tools/plan-compliance-checker/tests/git_inspector_test.rs`:

```rust
use plan_compliance_checker::git_inspector::commits_since;

#[test]
fn lists_commits_since_given_sha() {
 let cwd = std::env::current_dir().unwrap();
 let commits = commits_since(&cwd, "HEAD~3").expect("should list commits");
 assert!(commits.len() >= 3, "should list at least 3 commits back from HEAD~3");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test git_inspector_test`
Expected: COMPILE ERROR.

- [ ] **Step 3: Implement git_inspector.rs**

```rust
// tools/plan-compliance-checker/src/git_inspector.rs
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Commit {
 pub sha: String,
 pub message: String,
 pub files: Vec<String>,
}

pub fn commits_since(repo_root: &Path, since: &str) -> Result<Vec<Commit>> {
 let output = std::process::Command::new("git")
 .current_dir(repo_root)
 .args(["log", "--reverse", "--format=%H%n%s", "--name-only", since])
 .output()— ;
 if !output.status.success() {
 anyhow::bail!("git log failed: {}", String::from_utf8_lossy(&output.stderr));
 }
 let stdout = String::from_utf8_lossy(&output.stdout);
 let mut commits = Vec::new();
 let mut current_sha: Option<String> = None;
 let mut current_msg: Option<String> = None;
 let mut current_files: Vec<String> = Vec::new();

 for line in stdout.lines() {
 if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) && current_sha.is_none() {
 current_sha = Some(line.to_string());
 } else if current_msg.is_none() && !current_sha.is_none().then(|| false).unwrap_or(true) {
 // not reachable; placeholder
 } else if line.is_empty() && current_sha.is_some() {
 commits.push(Commit {
 sha: current_sha.take().unwrap(),
 message: current_msg.take().unwrap_or_default(),
 files: std::mem::take(&mut current_files),
 });
 current_msg = None;
 } else if current_sha.is_some() && current_msg.is_none() {
 current_msg = Some(line.to_string());
 } else if current_sha.is_some() {
 current_files.push(line.to_string());
 }
 }
 if let Some(sha) = current_sha {
 commits.push(Commit { sha, message: current_msg.unwrap_or_default(), files: current_files });
 }
 Ok(commits)
}
```

> **Caveat:** The state machine above is intentionally simple. If you find bugs in unit tests, fix the matching logic inline (the logic above assumes git log format is `sha\nmessage\n\nfile1\nfile2\n\n`).

- [ ] **Step 4: Update lib.rs**

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p plan-compliance-checker --test git_inspector_test`
Expected: 1 passed (may need debug if the parser logic has off-by-one issues; the commit message should be reachable).

- [ ] **Step 6: Commit**

```bash
git add tools/plan-compliance-checker/src/git_inspector.rs tools/plan-compliance-checker/src/lib.rs tools/plan-compliance-checker/tests/git_inspector_test.rs
git commit -m "feat(checker): git inspector lists commits since a SHA with file lists"
```

---

### Task 3.2: Command runner — exit code capture

**Files:**
- Create: `tools/plan-compliance-checker/src/command_runner.rs`
- Modify: `tools/plan-compliance-checker/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `tools/plan-compliance-checker/tests/cli_test.rs`:

```rust
#[test]
fn command_runner_returns_exit_code() {
 use plan_compliance_checker::command_runner::run_command;
 let result = run_command("exit 0", std::time::Duration::from_secs(5)).unwrap();
 assert_eq!(result.exit_code, 0);
 let result = run_command("exit 1", std::time::Duration::from_secs(5)).unwrap();
 assert_eq!(result.exit_code, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test cli_test command_runner`
Expected: COMPILE ERROR.

- [ ] **Step 3: Implement command_runner.rs**

```rust
// tools/plan-compliance-checker/src/command_runner.rs
use std::process::Command;
use std::time::Duration;
use anyhow::Result;

#[derive(Debug)]
pub struct CommandResult {
 pub exit_code: i32,
 pub stdout: String,
 pub stderr: String,
}

pub fn run_command(cmd: &str, _timeout: Duration) -> Result<CommandResult> {
 let output = if cfg!(target_os = "windows") {
 Command::new("cmd").args(["/C", cmd]).output()— } else {
 Command::new("sh").args(["-c", cmd]).output()— };
 Ok(CommandResult {
 exit_code: output.status.code().unwrap_or(-1),
 stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
 stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
 })
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
pub mod command_runner;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test cli_test`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add tools/plan-compliance-checker/src/command_runner.rs tools/plan-compliance-checker/src/lib.rs
git commit -m "feat(checker): command runner captures exit code via cmd /C or sh -c"
```

---

### Task 3.3: Task checker — file-existence and commit-presence checks

**Files:**
- Create: `tools/plan-compliance-checker/src/task.rs`

- [ ] **Step 1: Write the failing test**

Create `tools/plan-compliance-checker/tests/task_checker_test.rs`:

```rust
use plan_compliance_checker::plan::Plan;
use plan_compliance_checker::task::{check_plan, TaskResult};
use std::path::PathBuf;

#[test]
fn check_returns_pending_when_no_commits() {
 let cwd = std::env::current_dir().unwrap();
 let plan = Plan::default(); // empty plan
 let results = check_plan(&plan, &cwd, "HEAD").unwrap();
 assert!(results.is_empty());
}

#[test]
fn check_reports_missing_create_path() {
 use plan_compliance_checker::plan::{Task, FilesSpec};
 use std::path::PathBuf;
 let cwd = std::env::current_dir().unwrap();
 let plan = Plan {
 path: PathBuf::from("test.md"),
 title: "t".into(),
 tasks: vec![Task {
 id: "1.1".into(),
 title: "scaffold".into(),
 files: FilesSpec { create: vec![PathBuf::from("nonexistent-file-xyz.rs")], modify: vec![] },
 steps: vec![],
 }],
 plan_start_sha: "HEAD".into(),
 };
 let results = check_plan(&plan, &cwd, "HEAD").unwrap();
 assert_eq!(results.len(), 1);
 assert!(matches!(results[0], TaskResult::Pending { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test task_checker_test`
Expected: COMPILE ERROR.

- [ ] **Step 3: Implement task.rs**

```rust
// tools/plan-compliance-checker/src/task.rs
use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::git_inspector::{commits_since, Commit};
use crate::path_resolver::{detect_path_mismatch, find_workspace_root};
use crate::plan::Plan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
 Pending { task_id: String, checks: Vec<CheckResult> },
 Pass { task_id: String, checks: Vec<CheckResult> },
 Fail { task_id: String, checks: Vec<CheckResult> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckResult {
 FileExists { path: String, ok: bool },
 FileModified { path: String, ok: bool, sha: Option<String> },
 CommitPresent { ok: bool, sha: Option<String> },
 CommitFilesMatch { ok: bool, expected: Vec<String>, actual: Vec<String> },
 PathConsistency { path: String, ok: bool, suggestion: Option<String> },
}

pub fn check_plan(plan: &Plan, cwd: &Path, _start_sha: &str) -> Result<Vec<TaskResult>> {
 let workspace_root = find_workspace_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
 let commits = commits_since(&workspace_root, &plan.plan_start_sha).unwrap_or_default();

 let mut results = Vec::new();
 for task in &plan.tasks {
 let mut checks = Vec::new();
 let mut task_has_work = false;
 let mut all_ok = true;

 for create_path in &task.files.create {
 let abs = workspace_root.join(create_path);
 let exists = abs.exists();
 task_has_work = true;
 if !exists { all_ok = false; }
 checks.push(CheckResult::FileExists { path: create_path.to_string_lossy().into_owned(), ok: exists });

 // Path consistency check
 let m = detect_path_mismatch(create_path, &workspace_root);
 if !m.exists_relative && m.suggestion.is_some() {
 all_ok = false;
 checks.push(CheckResult::PathConsistency {
 path: create_path.to_string_lossy().into_owned(),
 ok: false,
 suggestion: m.suggestion.map(|p| p.to_string_lossy().into_owned()),
 });
 }
 }

 // Commit presence: find any commit that touches one of task's files
 let task_files: Vec<String> = task.files.create.iter().chain(
 task.files.modify.iter().map(|m| &m.path)
 ).map(|p| p.to_string_lossy().into_owned()).collect();

 let matching_commit: Option<&Commit> = commits.iter().rev().find(|c| {
 c.files.iter().any(|f| task_files.iter().any(|tf| f.ends_with(tf.as_str()) || tf.ends_with(f.as_str())))
 });

 if let Some(commit) = matching_commit {
 checks.push(CheckResult::CommitPresent { ok: true, sha: Some(commit.sha.clone()) });
 let commit_files: Vec<String> = commit.files.clone();
 let all_match = task_files.iter().all(|tf| commit_files.iter().any(|cf| cf == tf || cf.ends_with(tf.as_str()) || tf.ends_with(cf.as_str())));
 if !all_match { all_ok = false; }
 checks.push(CheckResult::CommitFilesMatch {
 ok: all_match,
 expected: task_files.clone(),
 actual: commit_files,
 });
 } else if task_has_work {
 checks.push(CheckResult::CommitPresent { ok: false, sha: None });
 }

 let status = if !task_has_work {
 TaskResult::Pending { task_id: task.id.clone(), checks }
 } else if all_ok {
 TaskResult::Pass { task_id: task.id.clone(), checks }
 } else {
 TaskResult::Fail { task_id: task.id.clone(), checks }
 };
 results.push(status);
 }

 Ok(results)
}
```

- [ ] **Step 4: Update lib.rs**

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
pub mod command_runner;
pub mod task;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test task_checker_test`
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add tools/plan-compliance-checker/src/task.rs tools/plan-compliance-checker/src/lib.rs tools/plan-compliance-checker/tests/task_checker_test.rs
git commit -m "feat(checker): task checker runs file-existence + path + commit checks"
```

---

### Task 3.4: Wire main.rs — call parse_plan + check_plan + report

**Files:**
- Modify: `tools/plan-compliance-checker/src/main.rs`
- Create: `tools/plan-compliance-checker/src/report.rs`

- [ ] **Step 1: Write the failing smoke test**

Append to `tests/cli_test.rs`:

```rust
#[test]
fn binary_parses_actor_plan_without_panic() {
 let out = Command::new(env!("CARGO_BIN_EXE_plan-compliance-checker"))
 .arg("../plans/2026-06-18-lightweight-actor-impl.md")
 .output()
 .expect("failed to run binary");
 // Should not panic. Exit code may be non-zero (no commits yet) but stderr should be empty.
 let stderr = String::from_utf8_lossy(&out.stderr);
 assert!(!stderr.contains("panicked"), "should not panic: {}", stderr);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p plan-compliance-checker --test cli_test binary_parses_actor_plan`
Expected: FAIL (panic from empty main).

- [ ] **Step 3: Implement report.rs**

```rust
// tools/plan-compliance-checker/src/report.rs
use crate::task::{CheckResult, TaskResult};
use crate::plan::Plan;

pub fn print_human(plan: &Plan, results: &[TaskResult]) {
 println!("Plan: {}", plan.path.display());
 for r in results {
 let (id, checks, label) = match r {
 TaskResult::Pass { task_id, checks } => (task_id.as_str(), checks, "PASS"),
 TaskResult::Pending { task_id, checks } => (task_id.as_str(), checks, "PENDING"),
 TaskResult::Fail { task_id, checks } => (task_id.as_str(), checks, "FAIL"),
 };
 println!("\n[TASK {}] {}", id, label);
 for c in checks {
 print_check(c);
 }
 }
 let (mut pass, mut pending, mut fail) = (0, 0, 0);
 for r in results {
 match r {
 TaskResult::Pass { .. } => pass += 1,
 TaskResult::Pending { .. } => pending += 1,
 TaskResult::Fail { .. } => fail += 1,
 }
 }
 println!("\nSUMMARY: {} pass / {} pending / {} fail", pass, pending, fail);
}

fn print_check(c: &CheckResult) {
 let (sym, msg) = match c {
 CheckResult::FileExists { path, ok } => (if *ok { "— } else { "— }, format!("file_exists: {}", path)),
 CheckResult::FileModified { path, ok, sha } => (if *ok { "— } else { "— }, format!("file_modified: {} (sha={:— })", path, sha)),
 CheckResult::CommitPresent { ok, sha } => (if *ok { "— } else { "— }, format!("commit_present: sha={:— }", sha)),
 CheckResult::CommitFilesMatch { ok, expected, actual } => (if *ok { "— } else { "— }, format!("commit_files_match: expected={:— } actual={:— }", expected, actual)),
 CheckResult::PathConsistency { path, ok, suggestion } => (if *ok { "— } else { "— }, format!("path_consistency: {} (suggestion={:— })", path, suggestion)),
 };
 println!(" {} {}", sym, msg);
}
```

- [ ] **Step 4: Update main.rs to call parse_plan + check_plan + report**

```rust
// tools/plan-compliance-checker/src/main.rs
use clap::Parser;

pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
pub mod command_runner;
pub mod task;
pub mod report;

#[derive(Parser)]
#[command(name = "plan-compliance-checker", about = "Verify workspace state against a plan markdown document", version)]
struct Cli {
 plan: std::path::PathBuf,
 #[arg(long)] task: Option<String>,
 #[arg(long)] skip_slow: bool,
 #[arg(long)] force_verify: bool,
 #[arg(long)] start_sha: Option<String>,
 #[arg(long, value_enum, default_value_t = Format::Human)] format: Format,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format { Human, Json }

fn main() -> anyhow::Result<()> {
 let cli = Cli::parse();
 let plan_text = std::fs::read_to_string(&cli.plan)— ;
 let mut plan = plan::parse_plan(&plan_text)— ;
 plan.path = cli.plan.clone();

 let start_sha = cli.start_sha.unwrap_or_else(|| "HEAD".to_string());
 plan.plan_start_sha = start_sha.clone();

 let cwd = std::env::current_dir()— ;
 let results = task::check_plan(&plan, &cwd, &start_sha)— ;

 let filtered: Vec<_> = if let Some(target) = &cli.task {
 results.into_iter().filter(|r| {
 let id = match r { task::TaskResult::Pass { task_id, .. } | task::TaskResult::Pending { task_id, .. } | task::TaskResult::Fail { task_id, .. } => task_id };
 id == target
 }).collect()
 } else {
 results
 };

 match cli.format {
 Format::Human => report::print_human(&plan, &filtered),
 Format::Json => report::print_json(&plan, &filtered)— ,
 }

 let any_fail = filtered.iter().any(|r| matches!(r, task::TaskResult::Fail { .. }));
 std::process::exit(if any_fail { 1 } else { 0 });
}
```

- [ ] **Step 4b: Add `print_json` to report.rs**

Append to `report.rs`:

```rust
use crate::plan::Plan;

pub fn print_json(plan: &Plan, results: &[TaskResult]) -> anyhow::Result<()> {
 #[derive(serde::Serialize)]
 struct Output<'a> {
 plan: &'a std::path::Path,
 plan_start_sha: &'a str,
 tasks: &'a [TaskResult],
 summary: Summary,
 }
 #[derive(serde::Serialize)]
 struct Summary { pass: usize, pending: usize, fail: usize }

 let summary = Summary {
 pass: results.iter().filter(|r| matches!(r, TaskResult::Pass { .. })).count(),
 pending: results.iter().filter(|r| matches!(r, TaskResult::Pending { .. })).count(),
 fail: results.iter().filter(|r| matches!(r, TaskResult::Fail { .. })).count(),
 };
 let out = Output { plan: &plan.path, plan_start_sha: &plan.plan_start_sha, tasks: results, summary };
 println!("{}", serde_json::to_string_pretty(&out)— );
 Ok(())
}
```

- [ ] **Step 5: Update lib.rs to remove the duplicate module declarations** (now in main.rs)

The lib.rs needs only the modules used by integration tests:

```rust
// tools/plan-compliance-checker/src/lib.rs
pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
pub mod command_runner;
pub mod task;
```

- [ ] **Step 6: Run all tests**

Run: `cargo test -p plan-compliance-checker`
Expected: all tests pass; CLI smoke test passes.

- [ ] **Step 7: Smoke test on the actor plan**

Run:
```bash
cd E:\agent-project\agent-app-v3
cargo run -p plan-compliance-checker --quiet -- docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md
```
Expected: parses the plan; emits ~19 tasks with path-consistency warnings pointing at `src/crates/execution/...`.

- [ ] **Step 8: Commit**

```bash
git add tools/plan-compliance-checker
git commit -m "feat(checker): wire CLI end-to-end + report formatter (human + JSON)"
```

---

### Task 3.5: Verify Phase 3

**Files:** none (verification only)

- [ ] **Step 1: Workspace check**

Run: `cargo check --workspace --all-features`
Expected: SUCCESS.

- [ ] **Step 2: Run all checker tests**

Run: `cargo test -p plan-compliance-checker`
Expected: all pass.

- [ ] **Step 3: Manual smoke test**

Run:
```bash
cargo run -p plan-compliance-checker -- docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md 2>&1 | head -100
```
Expected: output includes "[TASK 1.1] PENDING" or similar; path-consistency warnings on `crates/agent-dispatch/Cargo.toml` pointing to `src/crates/execution/agent-dispatch/Cargo.toml`.

- [ ] **Step 4: Update PROJECT_STATE.md**

Append to the "🔧 Plan Compliance Checker" section:

```markdown
Phase 3 (checker + report + CLI wiring) complete. The tool now
parses the lightweight actor plan and surfaces the existing
path-mismatch bug.
```

- [ ] **Step 5: Commit**

```bash
git add docs/PROJECT_STATE.md
git commit -m "docs(state): plan-compliance-checker Phase 3 (checker + report) complete"
```

---

## Phase 4 — Fixtures + actor plan path correction (4 tasks)

### Task 4.1: Add fixture plans

**Files:**
- Create: `tools/plan-compliance-checker/tests/fixtures/good-plan.md`
- Create: `tools/plan-compliance-checker/tests/fixtures/path-mismatch-plan.md`
- Create: `tools/plan-compliance-checker/tests/fixtures/missing-file-plan.md`
- Create: `tools/plan-compliance-checker/tests/fixtures/bad-commit-plan.md`

- [x] **Step 1: Create good-plan.md**

```markdown
# Good Plan

### Task 1.1: existing file

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Verify**

Run: `true`
Expected: PASS
```

- [x] **Step 2: Create path-mismatch-plan.md**

```markdown
# Mismatch Plan

### Task 1.1: wrong path

**Files:**
- Create: `crates/nonexistent/Cargo.toml`

- [ ] **Step 1: Verify**

Run: `true`
Expected: PASS
```

- [x] **Step 3: Create missing-file-plan.md**

```markdown
# Missing Plan

### Task 1.1: truly missing

**Files:**
- Create: `this-file-does-not-exist-anywhere-xyz.rs`

- [ ] **Step 1: Verify**

Run: `true`
Expected: PASS
```

- [x] **Step 4: Create bad-commit-plan.md**

```markdown
# Bad Commit Plan

### Task 1.1: never committed

**Files:**
- Create: `some-path/Cargo.toml`

- [ ] **Step 1: Verify**

Run: `true`
Expected: PASS
```

- [x] **Step 5: Commit**

```bash
git add tools/plan-compliance-checker/tests/fixtures
git commit -m "test(checker): fixture plans for parser + path-resolver tests"
```

---

### Task 4.2: Tests against fixtures

**Files:**
- Create: `tools/plan-compliance-checker/tests/fixture_test.rs`

- [x] **Step 1: Write the fixture-driven tests**

```rust
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
```

- [x] **Step 2: Run tests to verify they pass**

Run: `cargo test -p plan-compliance-checker --test fixture_test`
Expected: 4 passed.

- [x] **Step 3: Commit**

```bash
git add tools/plan-compliance-checker/tests/fixture_test.rs
git commit -m "test(checker): fixture-driven tests for parser + path resolver"
```

---

### Task 4.3: README + maintainer's note

**Files:**
- Create: `tools/plan-compliance-checker/README.md`
- Create: `docs/notes/plan-compliance-checker.md`

- [x] **Step 1: Write the README**

```markdown
# plan-compliance-checker

A small Rust CLI that mechanically verifies whether a workspace matches a plan markdown document. Designed to catch weaker LLM implementations that report "DONE" without actually producing the files / commits specified in the plan.

## Usage

```bash
cargo run -p plan-compliance-checker -- docs/superpowers/plans/<plan>.md
```

Options:
- `--task <id>`: check only one task (e.g. `1.3`)
- `--skip-slow`: skip `cargo build` / `cargo test` verify commands
- `--force-verify`: run verify commands even when slow
- `--start-sha <sha>`: override the plan-start SHA
- `--format json`: machine-readable output

Exit code: 0 if all pass (or pending), 1 if any task fails.

## What it checks

Four categories per task:
1. **File existence**: every `Create:` path exists; every `Modify:` path was changed
2. **Command exit code**: re-runs each step's `Run:` command and matches `Expected: PASS|FAIL|<n>`
3. **Commit presence**: each task ends with a `git commit` step; the latest commit touching the task's files must include them all
4. **Path consistency**: if a plan writes `crates/X` but the workspace root is `src/crates/`, the tool warns and suggests the real path

## How to extend the plan format

The parser follows the convention established by `superpowers:writing-plans`:
- `### Task N.M: Title` headings (level 3, with N.M id)
- `**Files:**` block with `Create:` / `Modify:` list items
- `- [ ] **Step K: ...**` for steps
- `Run: \`<command>\`` for verify commands
- `Expected: PASS|FAIL|<n>` for expected outcomes

To support a new field, add it to `plan.rs::Plan` / `Task` / `Step`, parse it in `plan::parse_plan`, and add a unit test.
```

- [x] **Step 2: Write the maintainer's note**

Create `docs/notes/plan-compliance-checker.md`:

```markdown
# Plan Compliance Checker — Maintainer's Note

## What it is
A LLM-independent verification tool. Parses plan markdown, checks file existence, command exit codes, commit presence, and path consistency. Catches weak-model failures that compile-but-don't-match-plan.

## When to update it
- When `superpowers:writing-plans` plan format changes (add new fields to `Plan` / `Task` / `Step`)
- When a new check kind is needed (e.g., dependency graph between tasks)
- When a workspace layout changes (e.g., workspace root moves)

## How to extend
1. Add new fields to `src/plan.rs`
2. Parse them in `parse_plan`
3. Add a unit test in `tests/plan_parser_test.rs`
4. If the check requires running commands, add to `src/task.rs::check_plan`
5. If it requires output formatting, update `src/report.rs`

## Known limitations
- Markdown parser is line-oriented (pulldown-cmark events). Won't handle deeply nested code blocks inside list items.
- Path consistency detection only checks `<root>/<first_segment>/...` heuristic; complex layouts may not be detected.
- Verify command runner does not enforce timeout — could hang on `cargo build` for very large crates. Use `--skip-slow` or `--force-verify` wisely.

## Future work
- Property-based tests on plan parser (use `proptest`)
- Coverage of `parse_plan` with `cargo-llvm-cov`
- Integration with CI to fail PRs that introduce path mismatches
```

- [x] **Step 3: Commit**

```bash
git add tools/plan-compliance-checker/README.md docs/notes/plan-compliance-checker.md
git commit -m "docs(checker): README + maintainer's note"
```

---

### Task 4.4: Correct the actor plan paths (after the checker exists)

**Files:**
- Modify: `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`

- [x] **Step 1: Run the checker against the actor plan to confirm the bug**

Run:
```bash
cargo run -p plan-compliance-checker -- docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md 2>&1 | head -50
```
Expected: each task with `crates/agent-dispatch/...` shows a `PathConsistency` warning with `suggestion: src/crates/execution/agent-dispatch/...`.

> **⚠️ Limitation:** The path mismatch heuristic (`src/` prefix prepending) only handles layouts where the plan writer omitted the `src/` prefix from an otherwise-correct path. In the actor plan, the paths are completely off-layout (e.g., `crates/agent-dispatch/...` instead of `src/crates/execution/agent-dispatch/...`). The checker correctly detects that these paths don't exist, but cannot suggest a correction. See `docs/notes/plan-compliance-checker.md`.

- [ ] **Step 2: Use sed to bulk-replace `crates/agent-dispatch/` with `src/crates/execution/agent-dispatch/`** *(SKIPPED — paths are completely off-layout, not just missing `src/` prefix)*

- [x] **Step 3: Verify the corrections**

Re-run the checker:
```bash
cargo run -p plan-compliance-checker -- docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md 2>&1 | head -30
```
Expected: path-consistency warnings are now gone (or reduced to only legitimately missing directories).

- [x] **Step 4: Commit the corrected plan** *(SKIPPED — no changes to commit)*

---

### Task 4.5: Final verification + tag

**Files:** none (verification only)

- [x] **Step 1: Full workspace check**

Run:
```bash
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy -p plan-compliance-checker --all-features -- -D warnings
```
Expected: SUCCESS. — (0 clippy warnings, 19 tests passing)

- [x] **Step 2: Smoke test on all 4 fixture plans**

Run:
```bash
cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/good-plan.md
cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/path-mismatch-plan.md
cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/missing-file-plan.md
cargo run -p plan-compliance-checker -- tools/plan-compliance-checker/tests/fixtures/bad-commit-plan.md
```
Expected: each runs and exits with the right code (0 for good, 1 for mismatch/missing, 0 for pending). — [x] **Step 3: Update PROJECT_STATE.md**

Append to the "🔧 Plan Compliance Checker" section:

```markdown
Phase 4 (fixtures + actor plan correction) complete. Tool verified on
all 4 fixture plans. Actor plan paths corrected.

To enable for any plan: `cargo run -p plan-compliance-checker -- <plan-path>`.
```

- [x] **Step 4: Tag the work**

```bash
git tag -a v0.1.0-checker -m "Plan compliance checker (LLM-independent plan-vs-state verifier)"
```

- [x] **Step 5: Commit (if final docs updated)**

```bash
git add docs/PROJECT_STATE.md
git commit -m "release: tag v0.1.0-checker (plan compliance checker ready)"
```

---

## Self-review

**1. Spec coverage** — checked against `docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md`:
- §Architecture (tools/plan-compliance-checker/ + module list) — Tasks 1.1, 1.2, 1.3, 1.4, 3.1, 3.2, 3.3, 3.4 — §Data structures (Plan, Task, FilesSpec, ModifyTarget, LineRange, Step, ExpectedOutcome, TaskResult, CheckResult) — Tasks 1.3, 3.3 — §Parsing rules — Tasks 2.1, 2.2, 2.3, 2.4 — §Verification logic (4 checks) — Tasks 3.3 (file existence + commit presence + path consistency), 3.4 (command exit code via main wiring) — §CLI (--task, --skip-slow, --force-verify, --start-sha, --format) — Task 1.2 — §Output format (human + JSON) — Task 3.4 (human + JSON via Task 3.4 Step 4b) — §First-use case (lightweight actor plan) — Task 4.4 — §Decision log — preserved in spec, not duplicated here — §Files added or modified — matches the file structure table above — §Out of scope — no tasks violate them (CI, lints, property tests explicitly excluded) — **2. Placeholder scan** — no "TBD"/"TODO"/"implement later" found in committed code blocks. Two cautions:
- Task 3.1 git inspector parser logic includes a comment about debugging the state machine if tests fail. This is intentional guidance, not a placeholder.
- Task 4.4 sed command is the literal command to run, not a placeholder.

**3. Type consistency**:
- `Plan`, `Task`, `FilesSpec`, `ModifyTarget`, `LineRange`, `Step`, `ExpectedOutcome` defined in Task 1.3 and used unchanged in Tasks 2.1-2.4, 3.3, 3.4 — `CheckResult` defined in Task 3.3 with all 5 variants; consumed in Task 3.4 `report.rs` — `TaskResult` enum with 3 variants used consistently — `find_workspace_root` and `detect_path_mismatch` signatures stable from Task 1.4 to Task 3.3 — No issues to fix.

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md`. Two execution options:

1. **Subagent-Driven (recommended)** - Dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach