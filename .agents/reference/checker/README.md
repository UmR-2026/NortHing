# Plan Compliance Checker — Reference

> Code mirrors for the `plan-compliance-checker` crate. Read
> [`SIGNATURES.md`](./SIGNATURES.md) first, then [`NOTES.md`](./NOTES.md)
> for "do NOT copy" warnings.

## What this crate is

A CLI tool that takes a plan markdown file and reports which tasks are
done / pending / failed against the actual workspace state. Used to
verify that a multi-step plan has actually been implemented (not just
documented).

## File ordering — read in this sequence

| # | File | Why |
|---|---|---|
| 01 | [`01-crate-layout.md`](./01-crate-layout.md) | Overview, file inventory, phase status. |
| 02 | [`02-plan-struct-and-parser.rs`](./02-plan-struct-and-parser.rs) | `Plan`/`Task`/`Step`/`FilesSpec` data + `parse_plan` algorithm. ★★★ Heaviest file. |
| 04 | [`04-check-plan.rs`](./04-check-plan.rs) | `check_plan` + `CheckResult`/`TaskResult` enums. ★★★ Core check logic. |
| 05 | [`05-path-resolver.rs`](./05-path-resolver.rs) | `find_workspace_root` + `detect_path_mismatch`. The only path logic. |
| 06 | [`06-git-inspector.rs`](./06-git-inspector.rs) | `commits_since` via `git log`. The only git code. |
| 07 | [`07-report-formatter.rs`](./07-report-formatter.rs) | `format_human` + `format_json`. Output shape. |
| 08 | [`08-cli-dispatch.rs`](./08-cli-dispatch.rs) | clap `Cli` struct + dispatcher. The entry point. |
| 09 | [`09-fixture-format.md`](./09-fixture-format.md) | The 4 fixture files + format grammar. |

## How a check runs (end-to-end)

```
clap CLI (08)
   │
   │  parses args: --plan <path>, --task, --skip-slow, --force-verify, --start-sha
   ▼
parse_plan (02)  ──>  Plan { title, tasks, plan_start_sha }
   │
   ▼
check_plan (04)
   │
   ├── find_workspace_root(cwd)  (05)
   ├── commits_since(root, plan_start_sha)  (06)
   │
   └── for each task:
       ├── FileExists check (04)
       ├── PathConsistency check (04 + 05)
       ├── CommitPresent check (04 + 06)
       └── CommitFilesMatch check (04 + 06)
   │
   ▼
format_human | format_json (07)
```

## How to add a new check rule

(per `docs/notes/plan-compliance-checker.md` "How to extend")

1. Add the new variant to `CheckResult` in `task.rs`.
2. Add the new branch in `check_plan` (per-task loop in `task.rs`).
3. Add the new check to `report.rs` in **3 places**:
   - `check_to_json` (JSON output)
   - `print_check` (human output)
   - the destructuring in `format_human` / `format_json`
4. Add a unit test in `tests/task_checker_test.rs`.
5. Add a fixture in `tests/fixtures/` exercising the new check.
6. Update `09-fixture-format.md` (this directory).
7. Update `SIGNATURES.md` (this directory).

## Public API

```rust
// In tools/plan-compliance-checker/src/lib.rs:
pub use plan::{Plan, Task, Step, FilesSpec, ModifyTarget, LineRange, ExpectedOutcome};
pub use task::{TaskResult, CheckResult, check_plan};
pub use path_resolver::{find_workspace_root, detect_path_mismatch, PathMismatch};
pub use git_inspector::commits_since;
pub use report::{format_human, format_json, Report};

// CLI entry:
$ plan-compliance-checker <plan.md> [--task 1.3] [--skip-slow] [--force-verify] \
                                  [--start-sha <sha>] [--format human|json]
```
