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
