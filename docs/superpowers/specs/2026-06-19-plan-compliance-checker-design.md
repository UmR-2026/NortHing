# Plan Compliance Checker Design

> **Status:** Approved (brainstorming session, 2026-06-19)
> **For implementers:** After this spec is approved by the user, invoke `superpowers:writing-plans` to convert this into an implementation plan.

## Goal

Provide an **LLM-independent verification tool** that mechanically checks whether the post-task state of a workspace matches the plan it is supposed to implement. This protects against weaker models that report "DONE" when the underlying state does not actually reflect the plan's requirements.

The tool parses a plan markdown document, walks each `### Task N.M: Title` section, and runs four categories of checks per task:

1. **File existence**— every `Files:` `Create:` path exists; every `Modify:` path was changed since the plan started
2. **Command exit code**— every step with `Expected: PASS` (or `FAIL`) re-runs the embedded `Run:` command and matches the exit code
3. **Commit presence**— every task ends with a `git commit` step; the most recent commit after the plan's start SHA must touch at least one file listed in that task's `Files:` block
4. **Path consistency**— if a plan writes `crates/X/...` but the workspace root is `src/crates/`, the tool warns (and proposes the likely real path)

The tool is **not** a test framework, linter, or coverage analyzer. It only checks plan— state consistency.

## Non-goals

- Parsing the `## Non-goals` / `## Decision log` / `## Out of scope` sections of a plan
- Validating that the implementation matches the spec (this is `superpowers:writing-plans` self-review + human review)
- Running `cargo test` / `cargo clippy` (those are separate tools)
- Auto-fixing detected issues (always human-applies)
- Cross-plan consistency (each plan is verified against itself only)

## Context discovered during brainstorming

The user is concerned that a less-capable model will produce code that:

- Compiles (passing naive CI) but quietly breaks invariants
- Diverges from the plan (wrong paths, wrong flags, missing files)
- Reports "DONE" honestly but incorrectly (a model-side truthfulness gap)

The `v3-restructure` branch has a fragile CI today:

- `cargo check --workspace --exclude northhing-cli` runs on all PRs but **only** `cargo test --locked -p northhing-core` actually runs tests, and that is `northhing-core` only (834 of 1963 = 42% of test annotations)
- No `cargo clippy -D warnings` in CI
- No `cargo fmt --check` in CI
- No `[workspace.lints]` configured
- No property-based / fuzz / snapshot testing anywhere
- No coverage tooling configured

The lightweight actor plan (`docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`) also has a **path bug**: it writes `crates/agent-dispatch/Cargo.toml` repeatedly, but the workspace root is `E:\agent-project\northhing-v3\src\crates\`, not `E:\agent-project\northhing-v3\crates\`. The path-consistency check in this spec is what will catch that bug for any future plan-author and any executor (human or model).

## Architecture overview

```
tools/plan-compliance-checker/
├── Cargo.toml
├── README.md
├── src/— ├── main.rs # CLI entry (clap)— ├── plan.rs # parse plan markdown— Plan struct— ├── task.rs # per-task verification logic— ├── command_runner.rs # run shell command, capture exit— ├── git_inspector.rs # inspect git history / diff— ├── path_resolver.rs # detect path mismatches between plan and workspace— └── report.rs # print human-readable report
└── tests/
 ├── fixtures/— ├── good-plan.md— ├── path-mismatch-plan.md— ├── missing-file-plan.md— └── bad-commit-plan.md
 ├── plan_parser_test.rs
 ├── task_checker_test.rs
 ├── path_resolver_test.rs
 └── cli_test.rs
```

Dependency direction:

```
main— plan— task— { command_runner, git_inspector, path_resolver }— report
```

## Data structures

```rust
// plan.rs
pub struct Plan {
 pub path: PathBuf,
 pub title: String,
 pub tasks: Vec<Task>,
 pub plan_start_sha: String, // git SHA at the moment the plan was created
}

pub struct Task {
 pub id: String, // "1.1", "2.3"
 pub title: String,
 pub files: FilesSpec,
 pub steps: Vec<Step>,
}

pub struct FilesSpec {
 pub create: Vec<PathBuf>,
 pub modify: Vec<ModifyTarget>,
}

pub struct ModifyTarget {
 pub path: PathBuf,
 pub range: Option<LineRange>,
}

pub struct Step {
 pub index: usize,
 pub description: String,
 pub expected_outcome: ExpectedOutcome,
 pub verify_command: Option<String>,
}

pub enum ExpectedOutcome {
 Pass,
 Fail(String),
 Custom(i32),
}

// task.rs
pub struct TaskResult {
 pub task_id: String,
 pub status: TaskStatus,
 pub checks: Vec<CheckResult>,
}

pub enum TaskStatus {
 Pass,
 Fail,
 Pending, // no commits touched the task yet
}

pub enum CheckResult {
 FileExists { path: PathBuf, ok: bool },
 FileModified { path: PathBuf, ok: bool },
 CommandExit { step: usize, ok: bool, expected: ExpectedOutcome, actual: i32 },
 CommitPresent { ok: bool, sha: Option<String> },
 CommitFilesMatch { ok: bool, expected: Vec<PathBuf>, actual: Vec<PathBuf> },
 PathConsistency { path: PathBuf, ok: bool, suggestion: Option<PathBuf> },
}
```

## Parsing rules

The parser accepts plan markdown that follows the convention established by `superpowers:writing-plans`:

```markdown
### Task 1.1: scaffold crate manifest

**Files:**
- Create: `crates/agent-dispatch/Cargo.toml`
- Modify: `Cargo.toml`

- [ ] **Step 1: Write the manifest**

```toml
[package]
name = "northhing-agent-dispatch"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p northhing-agent-dispatch`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add crates/agent-dispatch Cargo.toml
git commit -m "feat(agent-dispatch): scaffold crate manifest"
```
```

The parser extracts:

- `### Task N.M: Title`— `Task.id = "N.M"`, `Task.title`
- `**Files:**` block:
 - `Create: \`<path>\``— `files.create`
 - `Modify: \`<path>\`` (optional `:N-M` range)— `files.modify`
- `- [ ] **Step K: ...**`— `steps[i].description`
- `Run: \`<command>\`` (within a step)— `steps[i].verify_command`
- `Expected: PASS|FAIL|...`— `steps[i].expected_outcome`

The parser is **lenient** about non-task sections; if it cannot find any `### Task N.M` headings, it returns `ParseError::NoTasksFound`.

## Verification logic

Each task runs four categories of checks:

### Check 1: File existence

For each `Create` path: `Path::exists()` must be `true`.

For each `Modify` path: `git log --since=<plan_start_sha> -- <path>` must show at least one commit. If a `LineRange` is specified, `git diff <plan_start_sha>..HEAD -- <path>` must touch lines in that range.

### Check 2: Command exit code

For each step with a `verify_command`:
1. Run `tokio::process::Command::new("sh").arg("-c").arg(cmd).status().await— `
2. Match exit code against `ExpectedOutcome`
3. If `Pass`, exit code must be `0`; if `Fail`, non-zero; if `Custom(n)`, exactly `n`

The verify is **skipped by default** if a `--task <id>` argument targets a specific task whose verify_command involves a long-running operation (heuristic: any command containing `cargo build` or `cargo test`). Use `--force-verify` to override.

### Check 3: Commit presence

For each task:
1. List commits since `plan_start_sha`: `git log --reverse --format=%H --since=<plan_start_sha>`
2. Find the most recent commit whose message contains `Task <id>` OR whose diff touches one of the task's `Files:` paths
3. If no such commit exists: `CommitPresent { ok: false }`
4. Verify the commit's diff touches **all** files in `Files:`. Mismatch— `CommitFilesMatch { ok: false }`

### Check 4: Path consistency

For each `Create` and `Modify` path:
1. If the path exists relative to CWD— `ok`
2. Else if the path is of the form `<root>/<rest>` where `<root>` is a workspace top-level directory (e.g., `crates`, `apps`, `tools`, `docs`, `tests`) but does not exist, AND `<workspace_root>/<root>/<rest>` exists— emit `PathConsistency { ok: false, suggestion: Some(<workspace_root>/<root>/<rest>) }`

The "workspace root" is detected by walking up from CWD until a `Cargo.toml` with `[workspace]` is found.

For the actor plan specifically, `crates/agent-dispatch/Cargo.toml` does not exist relative to `E:\agent-project\northhing-v3\` (workspace root), but `src/crates/execution/agent-dispatch/Cargo.toml` is where the crate should live. The path resolver detects this and suggests the correction.

## CLI

```bash
# Check entire plan against current workspace state
plan-compliance-checker docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md

# Check one task only
plan-compliance-checker --task 1.3 docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md

# Skip long-running verify commands (cargo build / cargo test)
plan-compliance-checker --skip-slow docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md

# Use a specific plan-start SHA (otherwise reads from .plan-compliance-checker.toml)
plan-compliance-checker --start-sha abc1234 docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md

# JSON output for CI consumption
plan-compliance-checker --format json docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md > report.json
```

## Output format

Human-readable (default):

```
[TASK 1.1] scaffold crate manifest + workspace registration— file_exists: crates/agent-dispatch/Cargo.toml— file_modified: Cargo.toml— step 2 exit: 0 (expected PASS)— commit_present: 0eaea42— path_warning: crates/agent-dispatch/Cargo.toml does not exist; did you mean src/crates/execution/agent-dispatch/Cargo.toml— 

[TASK 1.2] const flags + telemetry trait— pending (no commits touching tasks files yet)
 ...

SUMMARY: 0 pass / 19 pending / 0 fail / 1 path-warning
EXIT CODE: 0 (all pass or pending only) | 1 (any fail) | 2 (parse error)
```

JSON (`--format json`):

```json
{
 "plan": "docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md",
 "plan_start_sha": "abc1234",
 "tasks": [
 {
 "id": "1.1",
 "title": "scaffold crate manifest + workspace registration",
 "status": "Pending",
 "checks": [
 {"kind": "FileExists", "path": "crates/agent-dispatch/Cargo.toml", "ok": true},
 {"kind": "PathConsistency", "path": "crates/agent-dispatch/Cargo.toml", "ok": false, "suggestion": "src/crates/execution/agent-dispatch/Cargo.toml"}
 ]
 }
 ],
 "summary": {"pass": 0, "pending": 19, "fail": 0, "warnings": 1}
}
```

## First-use case: lightweight actor plan

The tool ships with a "golden test" fixture: when run against the actor plan (`docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`) with no commits applied, every task should be `Pending` (no commits yet) and **every path should produce a `PathConsistency` warning pointing at `src/crates/execution/...`**. This proves the path-detection logic works and surfaces the existing plan bug.

The actor plan will also need its **19 task paths corrected** during the implementation of *this* spec's plan— that is a separate concern, addressed by the writing-plans handoff.

## Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-06-19 | Tool is **LLM-independent** | Weak models cannot be trusted to self-report; independent verification is the only safe option |
| 2026-06-19 | Path consistency check is **first-class** | The actor plan has a real path bug; any tool that doesn't catch it would be useless |
| 2026-06-19 | Verify commands are **skipped by default** for slow commands | Avoid 5-min waits for `cargo build`; user can opt in with `--force-verify` |
| 2026-06-19 | Tool lives in `tools/`, not `crates/` | Tool is a developer convenience, not part of the agent runtime; matches existing `scripts/` convention |
| 2026-06-19 | No automatic fix-up | Auto-fixing hides issues; human always sees the warning and decides |
| 2026-06-19 | First-use case is the actor plan | Validates path-detection works on real bug; future plans inherit the check |

## Files added or modified

| Path | Change |
|---|---|
| `tools/plan-compliance-checker/Cargo.toml` | NEW: tool manifest |
| `tools/plan-compliance-checker/README.md` | NEW: usage docs |
| `tools/plan-compliance-checker/src/main.rs` | NEW: clap CLI entry |
| `tools/plan-compliance-checker/src/plan.rs` | NEW: markdown parser |
| `tools/plan-compliance-checker/src/task.rs` | NEW: task-level checks |
| `tools/plan-compliance-checker/src/command_runner.rs` | NEW: shell command runner |
| `tools/plan-compliance-checker/src/git_inspector.rs` | NEW: git history inspection |
| `tools/plan-compliance-checker/src/path_resolver.rs` | NEW: workspace root + path mismatch detection |
| `tools/plan-compliance-checker/src/report.rs` | NEW: human + JSON formatter |
| `tools/plan-compliance-checker/tests/plan_parser_test.rs` | NEW: parser unit tests |
| `tools/plan-compliance-checker/tests/task_checker_test.rs` | NEW: checker unit tests |
| `tools/plan-compliance-checker/tests/path_resolver_test.rs` | NEW: path resolution tests |
| `tools/plan-compliance-checker/tests/cli_test.rs` | NEW: end-to-end CLI tests |
| `tools/plan-compliance-checker/tests/fixtures/*.md` | NEW: golden test plans |
| `Cargo.toml` | MODIFY: add `tools/plan-compliance-checker` to `[workspace.members]` |
| `docs/notes/plan-compliance-checker.md` | NEW: maintainer's note |
| `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md` | MODIFY (during this spec's implementation plan): correct all 19 task paths from `crates/...` to `src/crates/execution/...`— one commit per file, applied **after** the plan-compliance-checker is built so the checker can verify the corrected paths |

## Out of scope

- Rewriting the actor plan or any other plan beyond path correction
- Adding CI integration (CI will need a separate spec to decide which checks to enable)
- Adding `cargo clippy -D warnings` or other workspace lints (separate spec)
- Property-based testing (separate spec)
- Coverage measurement (separate spec)
- Plan-vs-spec consistency (handled by `superpowers:writing-plans` self-review)

## Related

- Parent brainstorming context: 2026-06-19 session
- Plans it will verify: `docs/superpowers/plans/2026-06-18-*.md`, `docs/superpowers/plans/2026-06-19-*.md`
- Actor plan with the path bug it will catch: `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md`
- v3 testing baseline discovered during brainstorming: 1963 test annotations, 42% CI coverage
- Skills used: `superpowers:brainstorming`, `superpowers:writing-plans` (next), `superpowers:verification-before-completion`
